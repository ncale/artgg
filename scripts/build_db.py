#!/usr/bin/env python3
"""
build_db.py — Builds and enriches the Met Museum collection SQLite database.

Commands:
  build          Build the DB from the local CSV (default)
  fetch-images   Fetch real image URLs from the Met API for all artworks

Source CSV: ../assets/raw/MetObjects.csv
Output DB:  ../assets/collection.db
Schema:     ./schema.sql

Usage:
    pip install -r requirements.txt
    python build_db.py build
    python build_db.py fetch-images [--delay 0.5] [--limit N]

Python 3.10+ recommended.
"""

import sqlite3
import csv
import os
import re
import sys
import time
import argparse

try:
    import requests
except ImportError:
    requests = None

# ---------------------------------------------------------------------------
# Paths (all relative to this script's location)
# ---------------------------------------------------------------------------
SCRIPT_DIR  = os.path.dirname(os.path.abspath(__file__))
ASSETS_DIR  = os.path.join(SCRIPT_DIR, "..", "assets")
CSV_PATH    = os.path.join(ASSETS_DIR, "raw", "MetObjects.csv")
DB_PATH     = os.path.join(ASSETS_DIR, "collection.db")
SCHEMA_PATH = os.path.join(SCRIPT_DIR, "schema.sql")

MET_API_BASE = "https://collectionapi.metmuseum.org/public/collection/v1/objects"

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------
def build_artist_display(row: dict) -> str:
    """Combine artist name + nationality/dates into a single display string."""
    name        = row.get("Artist Display Name", "").strip()
    nationality = row.get("Artist Nationality", "").strip()
    begin_date  = row.get("Artist Begin Date", "").strip()
    end_date    = row.get("Artist End Date", "").strip()

    if not name:
        return ""
    parts = []
    if nationality:
        parts.append(nationality)
    if begin_date and end_date:
        parts.append(f"{begin_date}–{end_date}")
    elif begin_date:
        parts.append(f"b. {begin_date}")

    return f"{name} ({', '.join(parts)})" if parts else name


def normalize_tags(raw: str) -> str:
    """Convert pipe-separated tag strings to lowercase pipe-separated."""
    if not raw:
        return ""
    tags = [t.strip().lower() for t in raw.split("|") if t.strip()]
    return "|".join(tags)


def extract_year(date_display: str) -> int | None:
    """Extract the first 4-digit year from a date string."""
    if not date_display:
        return None
    m = re.search(r'\b(\d{4})\b', date_display)
    return int(m.group(1)) if m else None


def should_include(row: dict) -> bool:
    """Only keep public-domain rows that have some link resource."""
    return (
        row.get("Is Public Domain", "").strip() == "True"
        and row.get("Link Resource", "").strip() != ""
    )


# ---------------------------------------------------------------------------
# Command: build
# ---------------------------------------------------------------------------
def cmd_build():
    if not os.path.exists(CSV_PATH):
        print(f"ERROR: CSV not found at {CSV_PATH}")
        print("Download MetObjects.csv from https://github.com/metmuseum/openaccess")
        print("and place it in assets/raw/MetObjects.csv")
        sys.exit(1)

    if not os.path.exists(SCHEMA_PATH):
        print(f"ERROR: schema.sql not found at {SCHEMA_PATH}")
        sys.exit(1)

    if os.path.exists(DB_PATH):
        os.remove(DB_PATH)
        print(f"Removed existing database at {DB_PATH}")

    print(f"Reading schema from {SCHEMA_PATH}")
    with open(SCHEMA_PATH, "r") as f:
        schema_sql = f.read()

    print(f"Creating database at {DB_PATH}")
    conn = sqlite3.connect(DB_PATH)
    conn.executescript(schema_sql)

    print(f"Reading CSV from {CSV_PATH}")
    inserted = 0
    skipped  = 0

    with open(CSV_PATH, newline="", encoding="utf-8") as csvfile:
        reader = csv.DictReader(csvfile)

        for row in reader:
            if not should_include(row):
                skipped += 1
                continue

            try:
                object_id = int(row.get("Object ID", 0))
            except ValueError:
                skipped += 1
                continue

            date_display = row.get("Object Date", "").strip() or None
            year_approx  = extract_year(date_display) if date_display else None

            # Link Resource is a page URL, not an image URL.
            # The image_url field stays NULL until fetch-images is run.
            conn.execute(
                """
                INSERT OR IGNORE INTO artworks (
                    object_id, title, artist_display, date_display, medium,
                    dimensions, classification, culture, period, dynasty,
                    department, object_name, tags, image_url, is_public_domain,
                    year_approx
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, NULL, ?, ?)
                """,
                (
                    object_id,
                    row.get("Title", "").strip() or None,
                    build_artist_display(row) or None,
                    date_display,
                    row.get("Medium", "").strip() or None,
                    row.get("Dimensions", "").strip() or None,
                    row.get("Classification", "").strip() or None,
                    row.get("Culture", "").strip() or None,
                    row.get("Period", "").strip() or None,
                    row.get("Dynasty", "").strip() or None,
                    row.get("Department", "").strip() or None,
                    row.get("Object Name", "").strip() or None,
                    normalize_tags(row.get("Tags", "")),
                    1 if row.get("Is Public Domain", "").strip() == "True" else 0,
                    year_approx,
                ),
            )
            inserted += 1

            if inserted % 10_000 == 0:
                print(f"  ...{inserted} rows inserted")
                conn.commit()

    conn.commit()
    conn.close()

    print(f"\nDone. {inserted} artworks inserted, {skipped} rows skipped.")
    print(f"Database written to: {DB_PATH}")
    print(f"\nNext step: run 'python build_db.py fetch-images' to fetch real image URLs.")


# ---------------------------------------------------------------------------
# Command: fetch-images
# ---------------------------------------------------------------------------
def cmd_fetch_images(delay: float, limit: int | None):
    if requests is None:
        print("ERROR: 'requests' package not installed. Run: pip install requests")
        sys.exit(1)

    if not os.path.exists(DB_PATH):
        print(f"ERROR: Database not found at {DB_PATH}. Run 'python build_db.py build' first.")
        sys.exit(1)

    conn = sqlite3.connect(DB_PATH)

    # Count how many still need image URLs
    total_pending = conn.execute(
        "SELECT COUNT(*) FROM artworks WHERE image_url IS NULL"
    ).fetchone()[0]

    if limit:
        total_pending = min(total_pending, limit)

    print(f"Fetching image URLs for {total_pending} artworks (delay={delay}s between requests)")
    print("Press Ctrl+C to stop — progress is saved continuously.\n")

    query = "SELECT object_id FROM artworks WHERE image_url IS NULL ORDER BY object_id"
    if limit:
        query += f" LIMIT {limit}"

    rows = conn.execute(query).fetchall()

    session = requests.Session()
    session.headers.update({"User-Agent": "artgg/0.1 (wallpaper generator; educational use)"})

    fetched = 0
    no_image = 0
    errors   = 0

    for (object_id,) in rows:
        try:
            resp = session.get(f"{MET_API_BASE}/{object_id}", timeout=15)
            if resp.status_code == 200:
                data = resp.json()
                # Prefer the small version to save bandwidth; fall back to full
                image_url = (
                    data.get("primaryImageSmall")
                    or data.get("primaryImage")
                    or None
                )
                if image_url:
                    conn.execute(
                        "UPDATE artworks SET image_url = ? WHERE object_id = ?",
                        (image_url, object_id)
                    )
                    fetched += 1
                    print(f"  [{fetched}/{total_pending}] {object_id}: {image_url[:60]}...")
                else:
                    # No image available — mark as explicitly empty so we skip it next time
                    conn.execute(
                        "UPDATE artworks SET image_url = '' WHERE object_id = ?",
                        (object_id,)
                    )
                    no_image += 1

                conn.commit()
            elif resp.status_code == 404:
                conn.execute(
                    "UPDATE artworks SET image_url = '' WHERE object_id = ?",
                    (object_id,)
                )
                conn.commit()
                no_image += 1
            else:
                print(f"  WARNING: HTTP {resp.status_code} for object {object_id}")
                errors += 1

        except KeyboardInterrupt:
            print(f"\nInterrupted. Progress saved ({fetched} fetched so far).")
            break
        except Exception as e:
            print(f"  ERROR for object {object_id}: {e}")
            errors += 1

        time.sleep(delay)

    # Show remaining count before closing
    still_pending = conn.execute(
        "SELECT COUNT(*) FROM artworks WHERE image_url IS NULL"
    ).fetchone()[0]

    conn.close()

    print(f"\nDone.")
    print(f"  {fetched} image URLs fetched")
    print(f"  {no_image} objects have no image (marked as empty)")
    print(f"  {errors} errors (image_url left NULL — will retry on next run)")
    if still_pending > 0:
        print(f"  {still_pending} objects still pending — run again to continue.")


# ---------------------------------------------------------------------------
# Entry point
# ---------------------------------------------------------------------------
def main():
    parser = argparse.ArgumentParser(
        description="artgg database builder",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog=__doc__,
    )
    subparsers = parser.add_subparsers(dest="command")

    # build subcommand
    subparsers.add_parser("build", help="Build the DB from MetObjects.csv")

    # fetch-images subcommand
    fetch_parser = subparsers.add_parser(
        "fetch-images", help="Fetch real image URLs from the Met API"
    )
    fetch_parser.add_argument(
        "--delay", type=float, default=0.5,
        help="Seconds to wait between API requests (default: 0.5)"
    )
    fetch_parser.add_argument(
        "--limit", type=int, default=None,
        help="Maximum number of artworks to process (default: all)"
    )

    args = parser.parse_args()

    if args.command == "build" or args.command is None:
        cmd_build()
    elif args.command == "fetch-images":
        cmd_fetch_images(delay=args.delay, limit=args.limit)
    else:
        parser.print_help()


if __name__ == "__main__":
    main()
