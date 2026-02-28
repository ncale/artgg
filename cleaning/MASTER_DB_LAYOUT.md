# Master DB Source of Truth and FS Layout

This project uses a local, API-free build step to produce canonical SQLite artifacts for client pulls.

## Canonical Inputs

- `cleaning/filtered_objects.csv`
  - Metadata-only object list.
  - This file is intentionally not expected to contain `Primary Image` columns.
- `cleaning/image_cache.db`
  - Local image-url cache from prior fetches.
  - This is the primary source for image availability (`primary_image`, `primary_image_small`).

## Source-of-Truth Hierarchy

1. `catalog_master.db` is the authoritative built catalog.
2. `pull_master.db` is a derived projection for client-side filtering and selection.

### `catalog_master.db` (authoritative)

- Scope: only objects with at least one image URL.
- Main tables:
  - `objects` (full metadata fields + image URL fields + `metadata_json` snapshot)
  - `tags`
  - `object_tags`
  - `build_info`
- Purpose: durable canonical dataset for rebuilds, audits, and future projections.

### `pull_master.db` (derived/client projection)

- Scope: only objects with at least one image URL.
- Main tables:
  - `objects` (query-focused subset)
  - `tags`
  - `object_tags`
  - `tag_stats` (precomputed counts)
  - `build_info`
- Purpose: efficient client pull/filter operations.

## Inclusion Rule (Objects with Images)

An object is included only if at least one URL is non-empty:

- `image_cache.image_cache.primary_image`
- `image_cache.image_cache.primary_image_small`

Current builder behavior also supports CSV fallback `Primary Image` columns if present, but that is optional and not required by the current pipeline.

## Filesystem Organization

Generated output root (default `cleaning/master/`) is release-oriented:

```text
cleaning/master/
  CURRENT_BUILD
  current/
    catalog_master.db
    pull_master.db
    manifest.json
    checksums.txt
  releases/
    <build_id>/
      catalog_master.db
      pull_master.db
      manifest.json
      checksums.txt
```

- `releases/<build_id>/` is immutable output for that build.
- `current/` is a copy of the active release.
- `CURRENT_BUILD` stores the active build id.

## Build Command

From repo root:

```bash
python cleaning/build_master_dbs.py
```

Explicit paths:

```bash
python cleaning/build_master_dbs.py \
  --csv cleaning/filtered_objects.csv \
  --image-cache-db cleaning/image_cache.db \
  --output-root cleaning/master
```

## Client Consumption

Clients should read from:

- `cleaning/master/current/pull_master.db`
- `cleaning/master/current/manifest.json` (build metadata + checksums)

Do not treat `filtered_objects.csv` as client runtime truth; it is an input artifact only.
