#!/usr/bin/env python3
"""
Build local "master" SQLite databases from local Met artifacts.

Inputs:
  - filtered_objects.csv
  - image_cache.db

Outputs (under --output-root):
  - releases/<build_id>/catalog_master.db   (authoritative source of truth)
  - releases/<build_id>/pull_master.db      (client pull projection)
  - releases/<build_id>/manifest.json
  - releases/<build_id>/checksums.txt
  - current/*                               (copy of latest release)
  - CURRENT_BUILD

This script does not call any network APIs.
"""

from __future__ import annotations

import argparse
import csv
import hashlib
import json
import shutil
import sqlite3
from dataclasses import asdict, dataclass
from datetime import datetime, timezone
from pathlib import Path

BATCH_SIZE = 5_000


@dataclass
class BuildStats:
    rows_scanned: int = 0
    rows_with_images: int = 0
    rows_skipped_no_images: int = 0
    rows_skipped_bad_object_id: int = 0
    rows_public_domain: int = 0
    rows_highlight: int = 0
    rows_with_images_from_cache: int = 0
    rows_with_images_from_csv_fallback: int = 0


def parse_bool_int(raw: str | None) -> int:
    if raw is None:
        return 0
    value = raw.strip().lower()
    if value in {"1", "true", "t", "yes", "y"}:
        return 1
    return 0


def parse_int_or_none(raw: str | None) -> int | None:
    if raw is None:
        return None
    value = raw.strip()
    if not value:
        return None
    try:
        return int(value)
    except ValueError:
        return None


def text(raw: str | None) -> str:
    return (raw or "").strip()


def parse_tags(raw: str | None) -> list[str]:
    if raw is None:
        return []
    cleaned = [part.strip() for part in raw.split("|")]
    return sorted({tag for tag in cleaned if tag})


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def ensure_parent(path: Path) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)


def load_image_map(cache_db: Path) -> dict[int, tuple[str, str]]:
    if not cache_db.exists():
        raise FileNotFoundError(f"Image cache DB not found: {cache_db}")

    conn = sqlite3.connect(cache_db)
    try:
        rows = conn.execute(
            """
            SELECT object_id, primary_image, primary_image_small
            FROM image_cache
            WHERE primary_image != '' OR primary_image_small != ''
            """
        )
        image_map: dict[int, tuple[str, str]] = {}
        for object_id, primary_image, primary_image_small in rows:
            image_map[int(object_id)] = (
                text(primary_image),
                text(primary_image_small),
            )
        return image_map
    finally:
        conn.close()


def init_catalog_schema(conn: sqlite3.Connection) -> None:
    conn.execute("PRAGMA foreign_keys = ON")
    conn.execute("PRAGMA journal_mode = DELETE")
    conn.execute("PRAGMA synchronous = NORMAL")
    conn.execute("PRAGMA temp_store = MEMORY")
    conn.executescript(
        """
        CREATE TABLE build_info (
            key   TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );

        CREATE TABLE objects (
            object_id            INTEGER PRIMARY KEY,
            title                TEXT NOT NULL,
            artist_display_name  TEXT NOT NULL DEFAULT '',
            artist_display_bio   TEXT NOT NULL DEFAULT '',
            artist_nationality   TEXT NOT NULL DEFAULT '',
            artist_begin_date    TEXT NOT NULL DEFAULT '',
            artist_end_date      TEXT NOT NULL DEFAULT '',
            object_date          TEXT NOT NULL DEFAULT '',
            object_begin_date    INTEGER,
            object_end_date      INTEGER,
            department           TEXT NOT NULL DEFAULT '',
            classification       TEXT NOT NULL DEFAULT '',
            object_name          TEXT NOT NULL DEFAULT '',
            medium               TEXT NOT NULL DEFAULT '',
            culture              TEXT NOT NULL DEFAULT '',
            country              TEXT NOT NULL DEFAULT '',
            is_public_domain     INTEGER NOT NULL,
            is_highlight         INTEGER NOT NULL,
            link_resource        TEXT NOT NULL DEFAULT '',
            primary_image        TEXT NOT NULL DEFAULT '',
            primary_image_small  TEXT NOT NULL DEFAULT '',
            metadata_json        TEXT NOT NULL
        );

        CREATE TABLE tags (
            id    INTEGER PRIMARY KEY AUTOINCREMENT,
            value TEXT NOT NULL UNIQUE
        );

        CREATE TABLE object_tags (
            object_id INTEGER NOT NULL REFERENCES objects(object_id) ON DELETE CASCADE,
            tag_id    INTEGER NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
            PRIMARY KEY (object_id, tag_id)
        );
        """
    )


def finalize_catalog_schema(conn: sqlite3.Connection) -> None:
    conn.executescript(
        """
        CREATE INDEX idx_catalog_objects_public_domain ON objects(is_public_domain);
        CREATE INDEX idx_catalog_objects_end_date ON objects(object_end_date);
        CREATE INDEX idx_catalog_objects_department ON objects(department);
        CREATE INDEX idx_catalog_objects_classification ON objects(classification);
        CREATE INDEX idx_catalog_object_tags_tag_id ON object_tags(tag_id);
        """
    )


def init_pull_schema(conn: sqlite3.Connection) -> None:
    conn.execute("PRAGMA foreign_keys = ON")
    conn.execute("PRAGMA journal_mode = DELETE")
    conn.execute("PRAGMA synchronous = NORMAL")
    conn.execute("PRAGMA temp_store = MEMORY")
    conn.executescript(
        """
        CREATE TABLE build_info (
            key   TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );

        CREATE TABLE objects (
            object_id            INTEGER PRIMARY KEY,
            title                TEXT NOT NULL,
            artist_display_name  TEXT NOT NULL DEFAULT '',
            object_begin_date    INTEGER,
            object_end_date      INTEGER,
            department           TEXT NOT NULL DEFAULT '',
            classification       TEXT NOT NULL DEFAULT '',
            medium               TEXT NOT NULL DEFAULT '',
            is_public_domain     INTEGER NOT NULL,
            primary_image        TEXT NOT NULL DEFAULT '',
            primary_image_small  TEXT NOT NULL DEFAULT ''
        );

        CREATE TABLE tags (
            id    INTEGER PRIMARY KEY AUTOINCREMENT,
            value TEXT NOT NULL UNIQUE
        );

        CREATE TABLE object_tags (
            object_id INTEGER NOT NULL REFERENCES objects(object_id) ON DELETE CASCADE,
            tag_id    INTEGER NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
            PRIMARY KEY (object_id, tag_id)
        );

        CREATE TABLE tag_stats (
            tag_id               INTEGER PRIMARY KEY REFERENCES tags(id) ON DELETE CASCADE,
            object_count         INTEGER NOT NULL,
            public_domain_count  INTEGER NOT NULL
        );
        """
    )


def finalize_pull_schema(conn: sqlite3.Connection) -> None:
    conn.executescript(
        """
        INSERT INTO tag_stats(tag_id, object_count, public_domain_count)
        SELECT
            ot.tag_id,
            COUNT(*) AS object_count,
            SUM(o.is_public_domain) AS public_domain_count
        FROM object_tags ot
        JOIN objects o ON o.object_id = ot.object_id
        GROUP BY ot.tag_id;

        CREATE INDEX idx_pull_objects_public_domain_end_date ON objects(is_public_domain, object_end_date);
        CREATE INDEX idx_pull_objects_department ON objects(department);
        CREATE INDEX idx_pull_objects_classification ON objects(classification);
        CREATE INDEX idx_pull_object_tags_tag_id ON object_tags(tag_id);
        """
    )


def insert_build_info(conn: sqlite3.Connection, items: dict[str, str]) -> None:
    conn.executemany(
        "INSERT INTO build_info(key, value) VALUES (?, ?)",
        list(items.items()),
    )


def get_or_create_tag_id(
    conn: sqlite3.Connection,
    tag_cache: dict[str, int],
    tag_value: str,
) -> int:
    tag_id = tag_cache.get(tag_value)
    if tag_id is not None:
        return tag_id

    conn.execute("INSERT OR IGNORE INTO tags(value) VALUES (?)", (tag_value,))
    row = conn.execute("SELECT id FROM tags WHERE value = ?", (tag_value,)).fetchone()
    if row is None:
        raise RuntimeError(f"Unable to resolve tag id for: {tag_value}")
    tag_id = int(row[0])
    tag_cache[tag_value] = tag_id
    return tag_id


def publish_current_release(output_root: Path, release_dir: Path, build_id: str) -> None:
    current_dir = output_root / "current"
    if current_dir.exists():
        shutil.rmtree(current_dir)
    shutil.copytree(release_dir, current_dir)
    (output_root / "CURRENT_BUILD").write_text(f"{build_id}\n", encoding="utf-8")


def build_master_dbs(
    csv_path: Path,
    cache_db_path: Path,
    output_root: Path,
) -> dict:
    if not csv_path.exists():
        raise FileNotFoundError(f"CSV not found: {csv_path}")

    image_map = load_image_map(cache_db_path)
    now = datetime.now(timezone.utc)
    build_id = now.strftime("%Y%m%dT%H%M%SZ")

    release_dir = output_root / "releases" / build_id
    release_dir.mkdir(parents=True, exist_ok=False)

    catalog_db_path = release_dir / "catalog_master.db"
    pull_db_path = release_dir / "pull_master.db"
    manifest_path = release_dir / "manifest.json"
    checksums_path = release_dir / "checksums.txt"

    ensure_parent(catalog_db_path)
    ensure_parent(pull_db_path)

    catalog_conn = sqlite3.connect(catalog_db_path)
    pull_conn = sqlite3.connect(pull_db_path)

    stats = BuildStats()
    catalog_object_count = 0
    catalog_tag_count = 0
    pull_object_count = 0
    pull_tag_count = 0

    try:
        init_catalog_schema(catalog_conn)
        init_pull_schema(pull_conn)

        build_info = {
            "build_id": build_id,
            "generated_at_utc": now.isoformat(),
            "source_csv": str(csv_path),
            "source_image_cache_db": str(cache_db_path),
            "script": "cleaning/build_master_dbs.py",
        }
        insert_build_info(catalog_conn, build_info)
        insert_build_info(pull_conn, build_info)

        catalog_tag_cache: dict[str, int] = {}
        pull_tag_cache: dict[str, int] = {}

        catalog_conn.commit()
        pull_conn.commit()
        catalog_conn.execute("BEGIN")
        pull_conn.execute("BEGIN")

        with csv_path.open("r", newline="", encoding="utf-8") as handle:
            reader = csv.DictReader(handle)
            for row in reader:
                stats.rows_scanned += 1

                object_id_raw = row.get("Object ID")
                object_id = parse_int_or_none(object_id_raw)
                if object_id is None:
                    stats.rows_skipped_bad_object_id += 1
                    continue

                primary_image = ""
                primary_image_small = ""

                image_pair = image_map.get(object_id)
                if image_pair is not None:
                    primary_image, primary_image_small = image_pair
                    stats.rows_with_images_from_cache += 1
                else:
                    # Optional fallback if the CSV already has merged image columns.
                    primary_image = text(row.get("Primary Image"))
                    primary_image_small = text(row.get("Primary Image Small"))
                    if primary_image or primary_image_small:
                        stats.rows_with_images_from_csv_fallback += 1

                if not primary_image and not primary_image_small:
                    stats.rows_skipped_no_images += 1
                    continue

                is_public_domain = parse_bool_int(row.get("Is Public Domain"))
                is_highlight = parse_bool_int(row.get("Is Highlight"))

                if is_public_domain:
                    stats.rows_public_domain += 1
                if is_highlight:
                    stats.rows_highlight += 1

                metadata_json = json.dumps(row, ensure_ascii=True, sort_keys=True)

                catalog_conn.execute(
                    """
                    INSERT OR REPLACE INTO objects(
                        object_id, title, artist_display_name, artist_display_bio,
                        artist_nationality, artist_begin_date, artist_end_date, object_date,
                        object_begin_date, object_end_date, department, classification,
                        object_name, medium, culture, country, is_public_domain,
                        is_highlight, link_resource, primary_image, primary_image_small,
                        metadata_json
                    ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                    """,
                    (
                        object_id,
                        text(row.get("Title")),
                        text(row.get("Artist Display Name")),
                        text(row.get("Artist Display Bio")),
                        text(row.get("Artist Nationality")),
                        text(row.get("Artist Begin Date")),
                        text(row.get("Artist End Date")),
                        text(row.get("Object Date")),
                        parse_int_or_none(row.get("Object Begin Date")),
                        parse_int_or_none(row.get("Object End Date")),
                        text(row.get("Department")),
                        text(row.get("Classification")),
                        text(row.get("Object Name")),
                        text(row.get("Medium")),
                        text(row.get("Culture")),
                        text(row.get("Country")),
                        is_public_domain,
                        is_highlight,
                        text(row.get("Link Resource")),
                        primary_image,
                        primary_image_small,
                        metadata_json,
                    ),
                )

                pull_conn.execute(
                    """
                    INSERT OR REPLACE INTO objects(
                        object_id, title, artist_display_name, object_begin_date, object_end_date,
                        department, classification, medium, is_public_domain, primary_image,
                        primary_image_small
                    ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                    """,
                    (
                        object_id,
                        text(row.get("Title")),
                        text(row.get("Artist Display Name")),
                        parse_int_or_none(row.get("Object Begin Date")),
                        parse_int_or_none(row.get("Object End Date")),
                        text(row.get("Department")),
                        text(row.get("Classification")),
                        text(row.get("Medium")),
                        is_public_domain,
                        primary_image,
                        primary_image_small,
                    ),
                )

                for tag in parse_tags(row.get("Tags")):
                    catalog_tag_id = get_or_create_tag_id(
                        catalog_conn, catalog_tag_cache, tag
                    )
                    pull_tag_id = get_or_create_tag_id(pull_conn, pull_tag_cache, tag)
                    catalog_conn.execute(
                        "INSERT OR IGNORE INTO object_tags(object_id, tag_id) VALUES (?, ?)",
                        (object_id, catalog_tag_id),
                    )
                    pull_conn.execute(
                        "INSERT OR IGNORE INTO object_tags(object_id, tag_id) VALUES (?, ?)",
                        (object_id, pull_tag_id),
                    )

                stats.rows_with_images += 1

                if stats.rows_with_images % BATCH_SIZE == 0:
                    catalog_conn.commit()
                    pull_conn.commit()
                    catalog_conn.execute("BEGIN")
                    pull_conn.execute("BEGIN")

        catalog_conn.commit()
        pull_conn.commit()

        finalize_catalog_schema(catalog_conn)
        finalize_pull_schema(pull_conn)

        catalog_object_count = catalog_conn.execute(
            "SELECT COUNT(*) FROM objects"
        ).fetchone()[0]
        catalog_tag_count = catalog_conn.execute("SELECT COUNT(*) FROM tags").fetchone()[0]
        pull_object_count = pull_conn.execute("SELECT COUNT(*) FROM objects").fetchone()[0]
        pull_tag_count = pull_conn.execute("SELECT COUNT(*) FROM tags").fetchone()[0]

    except Exception:
        catalog_conn.close()
        pull_conn.close()
        shutil.rmtree(release_dir, ignore_errors=True)
        raise
    finally:
        if catalog_conn:
            try:
                catalog_conn.close()
            except sqlite3.Error:
                pass
        if pull_conn:
            try:
                pull_conn.close()
            except sqlite3.Error:
                pass

    checksums = {
        "catalog_master.db": sha256_file(catalog_db_path),
        "pull_master.db": sha256_file(pull_db_path),
    }

    checksums_path.write_text(
        "".join(f"{digest}  {name}\n" for name, digest in checksums.items()),
        encoding="utf-8",
    )

    manifest = {
        "build_id": build_id,
        "generated_at_utc": now.isoformat(),
        "inputs": {
            "csv": str(csv_path),
            "image_cache_db": str(cache_db_path),
        },
        "outputs": {
            "catalog_master_db": {
                "path": str(catalog_db_path),
                "sha256": checksums["catalog_master.db"],
                "bytes": catalog_db_path.stat().st_size,
                "objects": catalog_object_count,
                "tags": catalog_tag_count,
            },
            "pull_master_db": {
                "path": str(pull_db_path),
                "sha256": checksums["pull_master.db"],
                "bytes": pull_db_path.stat().st_size,
                "objects": pull_object_count,
                "tags": pull_tag_count,
            },
        },
        "stats": asdict(stats),
    }

    manifest_path.write_text(
        json.dumps(manifest, indent=2, ensure_ascii=True) + "\n",
        encoding="utf-8",
    )

    publish_current_release(output_root=output_root, release_dir=release_dir, build_id=build_id)

    return manifest


def parse_args() -> argparse.Namespace:
    script_dir = Path(__file__).resolve().parent
    parser = argparse.ArgumentParser(
        description="Build master SQLite DBs for local client pull workflows."
    )
    parser.add_argument(
        "--csv",
        type=Path,
        default=script_dir / "filtered_objects.csv",
        help="Path to filtered object CSV (default: cleaning/filtered_objects.csv).",
    )
    parser.add_argument(
        "--image-cache-db",
        type=Path,
        default=script_dir / "image_cache.db",
        help="Path to local image cache SQLite DB (default: cleaning/image_cache.db).",
    )
    parser.add_argument(
        "--output-root",
        type=Path,
        default=script_dir / "master",
        help="Output root for releases/current (default: cleaning/master).",
    )
    return parser.parse_args()


def main() -> None:
    args = parse_args()
    manifest = build_master_dbs(
        csv_path=args.csv,
        cache_db_path=args.image_cache_db,
        output_root=args.output_root,
    )
    print("Master build complete")
    print(f"Build ID: {manifest['build_id']}")
    print(f"Rows scanned: {manifest['stats']['rows_scanned']:,}")
    print(f"Rows with images: {manifest['stats']['rows_with_images']:,}")
    print(f"Catalog DB: {manifest['outputs']['catalog_master_db']['path']}")
    print(f"Pull DB: {manifest['outputs']['pull_master_db']['path']}")


if __name__ == "__main__":
    main()
