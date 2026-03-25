#!/usr/bin/env python3
"""
build_db.py — Builds the Met Museum collection SQLite database from the local CSV.

Source CSV: ../assets/raw/MetObjects.csv
Output DB:  ../assets/collection.db
Schema:     ./schema.sql

Usage:
    uv run python build_db.py
"""

import sqlite3
import csv
import os
import re
import sys

SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
ASSETS_DIR = os.path.join(SCRIPT_DIR, "..", "assets")
CSV_PATH = os.path.join(ASSETS_DIR, "raw", "MetObjects.csv")
DB_PATH = os.path.join(ASSETS_DIR, "collection.db")
SCHEMA_PATH = os.path.join(SCRIPT_DIR, "schema.sql")


def normalize_tags(raw: str) -> str:
    if not raw:
        return ""
    tags = [t.strip().lower() for t in raw.split("|") if t.strip()]
    return "|".join(tags)


def extract_year(date_display: str) -> int | None:
    if not date_display:
        return None
    m = re.search(r"\b(\d{4})\b", date_display)
    return int(m.group(1)) if m else None


def should_include(row: dict) -> bool:
    # Include any artwork that has a web presence (needed to fetch images).
    return row.get("Link Resource", "").strip() != ""


def parse_artists(row: dict) -> list[dict]:
    """
    Split all pipe-separated artist fields into a list of per-artist dicts.

    Note: 'Constituent ID' is NOT reliably pipe-separated in the Met CSV for
    multi-artist records — it appears to be concatenated without a delimiter.
    We use 'Artist Display Name' as the artist's stable identifier instead.
    """
    columns = {
        "display_name": "Artist Display Name",
        "display_bio":  "Artist Display Bio",
        "nationality":  "Artist Nationality",
        "begin_date":   "Artist Begin Date",
        "end_date":     "Artist End Date",
        "gender":       "Artist Gender",
        "ulan_url":     "Artist ULAN URL",
        "wikidata_url": "Artist Wikidata URL",
        "role":         "Artist Role",
        "prefix":       "Artist Prefix",
        "suffix":       "Artist Suffix",
        "alpha_sort":   "Artist Alpha Sort",
    }

    split: dict[str, list[str]] = {}
    max_len = 1
    for key, col in columns.items():
        parts = [p.strip() for p in (row.get(col, "") or "").split("|")]
        split[key] = parts
        max_len = max(max_len, len(parts))

    artists = []
    for i in range(max_len):
        entry = {key: (split[key][i] if i < len(split[key]) else "") for key in columns}
        if entry["display_name"]:
            artists.append(entry)
    return artists


def check_conflict(
    name: str,
    existing: dict,
    new: dict,
    conflicts: list[str],
) -> None:
    """Log any metadata differences for the same artist display name."""
    fields = ["display_bio", "nationality", "begin_date", "end_date", "gender"]
    diffs = []
    for f in fields:
        a = existing.get(f, "")
        b = new.get(f, "")
        if a != b and a and b:
            diffs.append(f"{f}: {repr(a)} vs {repr(b)}")
    if diffs:
        conflicts.append(f"  {repr(name)}: {'; '.join(diffs)}")


def main():
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
    with open(SCHEMA_PATH) as f:
        schema_sql = f.read()

    print(f"Creating database at {DB_PATH}")
    conn = sqlite3.connect(DB_PATH)
    conn.execute("PRAGMA foreign_keys = ON")
    conn.execute("PRAGMA journal_mode = WAL")
    conn.executescript(schema_sql)

    print(f"Reading CSV from {CSV_PATH}")
    inserted = 0
    skipped = 0

    # artist display_name → first-seen metadata (for inconsistency detection)
    artists_seen: dict[str, dict] = {}
    conflicts: list[str] = []

    with open(CSV_PATH, newline="", encoding="utf-8-sig") as csvfile:
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
            year_approx = extract_year(date_display) if date_display else None

            artists = parse_artists(row)
            primary_artist_name = artists[0]["display_name"] if artists else None

            conn.execute(
                """
                INSERT OR IGNORE INTO artworks (
                    object_id, title, artist_display, date_display, medium,
                    dimensions, classification, culture, period, dynasty,
                    department, object_name, tags, year_approx
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                """,
                (
                    object_id,
                    row.get("Title", "").strip() or None,
                    primary_artist_name,
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
                    year_approx,
                ),
            )

            for artist in artists:
                name = artist["display_name"]

                if name in artists_seen:
                    check_conflict(name, artists_seen[name], artist, conflicts)
                else:
                    artists_seen[name] = artist
                    conn.execute(
                        """
                        INSERT OR IGNORE INTO artists (
                            display_name, display_bio, nationality,
                            begin_date, end_date, gender, ulan_url, wikidata_url
                        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?)
                        """,
                        (
                            name,
                            artist["display_bio"] or None,
                            artist["nationality"] or None,
                            artist["begin_date"] or None,
                            artist["end_date"] or None,
                            artist["gender"] or None,
                            artist["ulan_url"] or None,
                            artist["wikidata_url"] or None,
                        ),
                    )

                conn.execute(
                    """
                    INSERT OR IGNORE INTO artwork_artists (
                        object_id, artist_name, role, prefix, suffix, alpha_sort
                    ) VALUES (?, ?, ?, ?, ?, ?)
                    """,
                    (
                        object_id,
                        name,
                        artist["role"] or None,
                        artist["prefix"] or None,
                        artist["suffix"] or None,
                        artist["alpha_sort"] or None,
                    ),
                )

            inserted += 1

            if inserted % 10_000 == 0:
                print(f"  ...{inserted} rows inserted")
                conn.commit()

    conn.commit()
    conn.close()

    print(f"\nDone. {inserted} artworks inserted, {skipped} rows skipped.")
    print(f"Distinct artists: {len(artists_seen)}")
    print(f"Database written to: {DB_PATH}")
    print("Image URLs will be fetched at runtime by artgg.")

    if conflicts:
        print(f"\nWARNING: {len(conflicts)} artist metadata inconsistencies detected")
        print("(same display name appearing with different bio/nationality/dates):")
        for line in conflicts[:50]:
            print(line)
        if len(conflicts) > 50:
            print(f"  ... and {len(conflicts) - 50} more")
    else:
        print("\nNo artist metadata inconsistencies detected.")


if __name__ == "__main__":
    main()
