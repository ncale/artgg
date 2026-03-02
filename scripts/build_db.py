#!/usr/bin/env python3
"""
build_db.py — Builds the Met Museum collection SQLite database from the local CSV.

Source CSV: ../assets/raw/MetObjects.csv
Output DB:  ../assets/collection.db
Schema:     ./schema.sql

Run this script once before building the Rust binary. The generated .db file
is embedded into the binary via include_bytes! in the Rust build.

Usage:
    pip install -r requirements.txt
    python build_db.py

Python 3.10+ recommended.
"""

import sqlite3
import csv
import os
import sys

# ---------------------------------------------------------------------------
# Paths (all relative to this script's location)
# ---------------------------------------------------------------------------
SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
ASSETS_DIR = os.path.join(SCRIPT_DIR, "..", "assets")
CSV_PATH   = os.path.join(ASSETS_DIR, "raw", "MetObjects.csv")
DB_PATH    = os.path.join(ASSETS_DIR, "collection.db")
SCHEMA_PATH = os.path.join(SCRIPT_DIR, "schema.sql")

# ---------------------------------------------------------------------------
# CSV column names as they appear in MetObjects.csv
# Mapping: csv column -> our schema column
# ---------------------------------------------------------------------------
COLUMN_MAP = {
    "Object ID":          "object_id",
    "Title":              "title",
    "Artist Display Name": "artist_display",  # we'll build artist_display below
    "Object Date":        "date_display",
    "Medium":             "medium",
    "Dimensions":         "dimensions",
    "Classification":     "classification",
    "Culture":            "culture",
    "Period":             "period",
    "Dynasty":            "dynasty",
    "Department":         "department",
    "Object Name":        "object_name",
    "Tags":               "tags",
    "Link Resource":      "image_url",        # placeholder; real URL needs API
    "Is Public Domain":   "is_public_domain",
}

# Only rows where Is Public Domain is True and there's an image are useful
def should_include(row: dict) -> bool:
    return (
        row.get("Is Public Domain", "").strip() == "True"
        and row.get("Link Resource", "").strip() != ""
    )

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
    """Convert pipe or pipe-separated tag strings to lowercase pipe-separated."""
    if not raw:
        return ""
    tags = [t.strip().lower() for t in raw.replace("|", "|").split("|") if t.strip()]
    return "|".join(tags)

# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------
def main():
    if not os.path.exists(CSV_PATH):
        print(f"ERROR: CSV not found at {CSV_PATH}")
        print("Download MetObjects.csv from https://github.com/metmuseum/openaccess")
        print("and place it in assets/raw/MetObjects.csv")
        sys.exit(1)

    if not os.path.exists(SCHEMA_PATH):
        print(f"ERROR: schema.sql not found at {SCHEMA_PATH}")
        sys.exit(1)

    # Remove stale DB if present
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

            conn.execute(
                """
                INSERT OR IGNORE INTO artworks (
                    object_id, title, artist_display, date_display, medium,
                    dimensions, classification, culture, period, dynasty,
                    department, object_name, tags, image_url, is_public_domain
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                """,
                (
                    object_id,
                    row.get("Title", "").strip() or None,
                    build_artist_display(row) or None,
                    row.get("Object Date", "").strip() or None,
                    row.get("Medium", "").strip() or None,
                    row.get("Dimensions", "").strip() or None,
                    row.get("Classification", "").strip() or None,
                    row.get("Culture", "").strip() or None,
                    row.get("Period", "").strip() or None,
                    row.get("Dynasty", "").strip() or None,
                    row.get("Department", "").strip() or None,
                    row.get("Object Name", "").strip() or None,
                    normalize_tags(row.get("Tags", "")),
                    row.get("Link Resource", "").strip() or None,
                    1 if row.get("Is Public Domain", "").strip() == "True" else 0,
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


if __name__ == "__main__":
    main()