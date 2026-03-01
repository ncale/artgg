#!/usr/bin/env python3
"""
Enrich filtered_objects.csv with image URLs from the Met Collection API.

Phase 1 (fetch): Hits GET /public/collection/v1/objects/{id} for every Object ID,
                 stores primaryImage and primaryImageSmall in image_cache.db.
Phase 2 (merge): Merges the cache back into the CSV, drops rows without a primaryImage,
                 and overwrites the CSV in place.

Usage:
    python enrich_images.py                  # full run (fetch + merge)
    python enrich_images.py --fetch-only     # fill cache, skip merge
    python enrich_images.py --merge-only     # merge existing cache, skip fetch
    python enrich_images.py --concurrency 30 # tune parallel requests (default: 20)
"""

import argparse
import asyncio
import sqlite3
import sys
from datetime import datetime, timezone

import aiohttp
import pandas as pd
from tqdm import tqdm

CSV_PATH = "filtered_objects.csv"
DB_PATH = "image_cache.db"
API_BASE = "https://collectionapi.metmuseum.org/public/collection/v1/objects"
BATCH_SIZE = 100


# ---------------------------------------------------------------------------
# SQLite helpers
# ---------------------------------------------------------------------------

def init_db(conn: sqlite3.Connection) -> None:
    conn.execute("""
        CREATE TABLE IF NOT EXISTS image_cache (
            object_id           INTEGER PRIMARY KEY,
            primary_image       TEXT    NOT NULL DEFAULT '',
            primary_image_small TEXT    NOT NULL DEFAULT '',
            fetched_at          TEXT    NOT NULL
        )
    """)
    conn.commit()


def load_cached_ids(conn: sqlite3.Connection) -> set[int]:
    rows = conn.execute("SELECT object_id FROM image_cache").fetchall()
    return {r[0] for r in rows}


def batch_insert(conn: sqlite3.Connection, rows: list[tuple]) -> None:
    """rows: list of (object_id, primary_image, primary_image_small, fetched_at)"""
    conn.executemany(
        "INSERT OR IGNORE INTO image_cache "
        "(object_id, primary_image, primary_image_small, fetched_at) "
        "VALUES (?, ?, ?, ?)",
        rows,
    )
    conn.commit()


# ---------------------------------------------------------------------------
# Async fetch
# ---------------------------------------------------------------------------

async def fetch_one(
    session: aiohttp.ClientSession,
    sem: asyncio.Semaphore,
    object_id: int,
) -> tuple[int, str, str]:
    """Return (object_id, primary_image, primary_image_small)."""
    url = f"{API_BASE}/{object_id}"
    backoff = 1
    for attempt in range(3):
        try:
            async with sem:
                await asyncio.sleep(1)
                async with session.get(url, timeout=aiohttp.ClientTimeout(total=15)) as resp:
                    if resp.status == 404:
                        return (object_id, "", "")
                    if resp.status in (429, 500, 502, 503, 504):
                        await asyncio.sleep(backoff)
                        backoff *= 2
                        continue
                    resp.raise_for_status()
                    data = await resp.json()
                    return (
                        object_id,
                        data.get("primaryImage") or "",
                        data.get("primaryImageSmall") or "",
                    )
        except (aiohttp.ClientError, asyncio.TimeoutError):
            if attempt < 2:
                await asyncio.sleep(backoff)
                backoff *= 2
    # All retries exhausted — record as empty so we don't retry again
    return (object_id, "", "")


async def run_async_fetcher(
    remaining_ids: list[int],
    conn: sqlite3.Connection,
    concurrency: int,
) -> None:
    sem = asyncio.Semaphore(concurrency)
    pending_batch: list[tuple] = []
    now = datetime.now(timezone.utc).isoformat()

    chunk_size = concurrency * 20  # avoid scheduling hundreds of thousands of tasks at once
    connector = aiohttp.TCPConnector(limit=concurrency)
    async with aiohttp.ClientSession(connector=connector) as session:
        with tqdm(total=len(remaining_ids), unit="obj", desc="Fetching") as pbar:
            for i in range(0, len(remaining_ids), chunk_size):
                chunk = remaining_ids[i : i + chunk_size]
                for oid, img, img_small in await asyncio.gather(
                    *[fetch_one(session, sem, oid) for oid in chunk]
                ):
                    pending_batch.append((oid, img, img_small, now))
                    pbar.update(1)
                    if len(pending_batch) >= BATCH_SIZE:
                        batch_insert(conn, pending_batch)
                        pending_batch.clear()

    if pending_batch:
        batch_insert(conn, pending_batch)


# ---------------------------------------------------------------------------
# Phase 1 — Fetch
# ---------------------------------------------------------------------------

def phase1_fetch(concurrency: int) -> None:
    print(f"\n=== PHASE 1: FETCH (concurrency={concurrency}) ===")

    PRIORITY_DEPTS = {"Modern and Contemporary Art", "European Paintings"}

    df = pd.read_csv(CSV_PATH, usecols=["Object ID", "Department"], low_memory=False)
    df["Object ID"] = df["Object ID"].dropna().astype(int)
    all_ids = set(df["Object ID"].tolist())
    print(f"Total Object IDs in CSV: {len(all_ids):,}")

    conn = sqlite3.connect(DB_PATH)
    init_db(conn)

    cached = load_cached_ids(conn)
    remaining_set = all_ids - cached

    priority_ids = sorted(
        df.loc[df["Department"].isin(PRIORITY_DEPTS), "Object ID"]
          .loc[lambda s: s.isin(remaining_set)]
          .tolist()
    )
    rest_ids = sorted(remaining_set - set(priority_ids))
    remaining = priority_ids + rest_ids

    priority_remaining = len(priority_ids)
    print(f"Already cached: {len(cached):,} / Remaining: {len(remaining):,} "
          f"({priority_remaining:,} priority dept IDs first)")

    if not remaining:
        print("Nothing to fetch.")
        conn.close()
        return

    asyncio.run(run_async_fetcher(remaining, conn, concurrency))

    final_count = conn.execute("SELECT COUNT(*) FROM image_cache").fetchone()[0]
    with_image = conn.execute(
        "SELECT COUNT(*) FROM image_cache WHERE primary_image != '' OR primary_image_small != ''"
    ).fetchone()[0]
    conn.close()

    print(f"\n✓ Cache complete: {final_count:,} entries, {with_image:,} have at least one image URL")


# ---------------------------------------------------------------------------
# Phase 2 — Merge & filter
# ---------------------------------------------------------------------------

def phase2_merge() -> None:
    print("\n=== PHASE 2: MERGE & FILTER ===")

    print(f"Loading {CSV_PATH}...")
    df = pd.read_csv(CSV_PATH, low_memory=False)
    original_count = len(df)

    conn = sqlite3.connect(DB_PATH)
    cache_df = pd.read_sql_query(
        "SELECT object_id, primary_image, primary_image_small FROM image_cache",
        conn,
    )
    conn.close()
    print(f"Cache entries: {len(cache_df):,}")

    cache_df = cache_df.rename(columns={
        "object_id": "Object ID",
        "primary_image": "Primary Image",
        "primary_image_small": "Primary Image Small",
    })

    df = df.merge(cache_df, on="Object ID", how="left")

    # Drop rows where both primaryImage and primaryImageSmall are empty
    before = len(df)
    has_primary = df["Primary Image"].notna() & (df["Primary Image"] != "")
    has_small = df["Primary Image Small"].notna() & (df["Primary Image Small"] != "")
    df = df[has_primary | has_small]
    dropped = before - len(df)

    print(f"Kept: {len(df):,} / Dropped (no image): {dropped:,} / Original: {original_count:,}")

    print(f"Writing {CSV_PATH}...")
    df.to_csv(CSV_PATH, index=False)
    print("✓ Done.")


# ---------------------------------------------------------------------------
# Entry point
# ---------------------------------------------------------------------------

def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--concurrency", type=int, default=20, metavar="N",
                        help="Max parallel API requests (default: 20)")
    mode = parser.add_mutually_exclusive_group()
    mode.add_argument("--fetch-only", action="store_true", help="Phase 1 only (fill cache)")
    mode.add_argument("--merge-only", action="store_true", help="Phase 2 only (merge cache into CSV)")
    args = parser.parse_args()

    if args.merge_only:
        phase2_merge()
    elif args.fetch_only:
        phase1_fetch(args.concurrency)
    else:
        phase1_fetch(args.concurrency)
        phase2_merge()


if __name__ == "__main__":
    main()
