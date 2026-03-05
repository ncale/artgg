CREATE TABLE IF NOT EXISTS taste_profiles (
    id               INTEGER PRIMARY KEY AUTOINCREMENT,
    name             TEXT NOT NULL,
    date_start       INTEGER,
    date_end         INTEGER,
    is_public_domain INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS display_profiles (
    id                 INTEGER PRIMARY KEY AUTOINCREMENT,
    name               TEXT NOT NULL,
    wallpaper_color    TEXT NOT NULL DEFAULT '#1a1a1a',
    frame_style        TEXT NOT NULL DEFAULT '',
    orientation        TEXT NOT NULL DEFAULT 'horizontal',
    canvas_width       INTEGER NOT NULL DEFAULT 1920,
    canvas_height      INTEGER NOT NULL DEFAULT 1080,
    placard_color      TEXT NOT NULL DEFAULT '#4a4a4a',
    placard_text_color TEXT NOT NULL DEFAULT '#ffffff',
    placard_opacity    INTEGER NOT NULL DEFAULT 90
);

CREATE TABLE IF NOT EXISTS taste_profile_departments (
    profile_id INTEGER NOT NULL REFERENCES taste_profiles(id) ON DELETE CASCADE,
    department TEXT NOT NULL,
    PRIMARY KEY (profile_id, department)
);

CREATE TABLE IF NOT EXISTS builds (
    id                 INTEGER PRIMARY KEY AUTOINCREMENT,
    created_at         INTEGER NOT NULL DEFAULT (unixepoch()),
    taste_profile_id   INTEGER,
    display_profile_id INTEGER,
    output_dir         TEXT NOT NULL DEFAULT '',
    count              INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS url_cache (
    object_id  INTEGER PRIMARY KEY,
    image_url  TEXT,
    is_valid   INTEGER NOT NULL DEFAULT 1,
    fetched_at INTEGER NOT NULL DEFAULT (unixepoch())
);
