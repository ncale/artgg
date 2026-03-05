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

Python 3.10+ required. Initialize the virtual environment, install the requirements, then run the `build_db.py` script. The `--help` flag has more details about how to use the script.

```bash
python build_db.py --help
```

---

## CSV source

`assets/raw/MetObjects.csv` was downloaded from the [Met Museum's open access
repository](https://github.com/metmuseum/openaccess).

Last downloaded: 2026-03-01
CSV last updated by Met: 2022 (as of project creation)

The CSV is committed to the repo as an immutable snapshot so builds are fully
reproducible without network access. If the Met updates the CSV and you want to
pull in new data, replace the file and re-run `build_db.py`.
