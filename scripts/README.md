# scripts/

Dev tooling for building the embedded SQLite database. None of this is included
in the distributed binary — it's only needed when regenerating `assets/collection.db`.

---

## What's here

| File               | Purpose                                             |
| ------------------ | --------------------------------------------------- |
| `build_db.py`      | Reads the Met CSV and writes `assets/collection.db` |
| `schema.sql`       | Single source of truth for the database schema      |
| `requirements.txt` | Python dependencies                                 |

---

## Setup

Python 3.10+ required.

```bash
cd scripts
pip install -r requirements.txt
```

---

## Generating the database

The CSV is already in the repo at `assets/raw/MetObjects.csv`. Just run:

```bash
python scripts/build_db.py
```

This will create `assets/collection.db`, which is gitignored. You need to
regenerate it before building the Rust binary, since it gets embedded via
`include_bytes!`.

---

## CSV source

`assets/raw/MetObjects.csv` was downloaded from the [Met Museum's open access
repository](https://github.com/metmuseum/openaccess).

Last downloaded: 2026-03-01
CSV last updated by Met: 2022 (as of project creation)

The CSV is committed to the repo as an immutable snapshot so builds are fully
reproducible without network access. If the Met updates the CSV and you want to
pull in new data, replace the file and re-run `build_db.py`.

---

## Schema

`schema.sql` defines the `artworks` table and an FTS5 virtual table for
full-text search across interest fields (medium, classification, culture, tags, etc.).

If you change the schema, you need to:

1. Update `schema.sql`
2. Re-run `build_db.py` to regenerate the DB
3. Update any raw SQL queries in the Rust source that reference changed column names

The Rust side accesses the DB via `include_bytes!` — see `src/db.rs` for query
definitions. Column names there must match `schema.sql` exactly.
