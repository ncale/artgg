-- Met Museum Collection Schema
-- Single source of truth for both the Python pipeline and Rust binary.
-- Any changes here must be reflected in queries on the Rust side.

CREATE TABLE IF NOT EXISTS artworks (
    -- Met's own identifier, stable and unique
    object_id         INTEGER PRIMARY KEY,

    -- Core display fields (used for the museum placard)
    title             TEXT,
    artist_display   TEXT,    -- e.g. "Vincent van Gogh (Dutch, 1853–1890)"
    date_display      TEXT,    -- e.g. "1889" or "ca. 1760–65"
    medium            TEXT,    -- e.g. "Oil on canvas"
    dimensions        TEXT,

    -- Classification / searchable interest fields
    classification    TEXT,    -- e.g. "Paintings", "Ceramics"
    culture           TEXT,    -- e.g. "French", "Japanese"
    period            TEXT,    -- e.g. "Edo period"
    dynasty           TEXT,
    department        TEXT,    -- e.g. "European Paintings"
    object_name       TEXT,    -- e.g. "Painting", "Vase"

    -- Tags: stored as pipe-separated string for simplicity
    -- e.g. "landscapes|trees|night sky"
    tags              TEXT,

    -- Image access
    image_url         TEXT,    -- primary image URL from the Met API
    is_public_domain  INTEGER  -- 0 or 1 (boolean)
);

-- Full-text search virtual table over the fields users might query by interest
CREATE VIRTUAL TABLE IF NOT EXISTS artworks_fts USING fts5(
    title,
    artist_display,
    medium,
    classification,
    culture,
    period,
    department,
    object_name,
    tags,
    content='artworks',
    content_rowid='object_id'
);

-- Keep FTS index in sync when artworks are inserted
CREATE TRIGGER IF NOT EXISTS artworks_ai AFTER INSERT ON artworks BEGIN
    INSERT INTO artworks_fts (
        rowid, title, artist_display, medium, classification,
        culture, period, department, object_name, tags
    ) VALUES (
        new.object_id, new.title, new.artist_display, new.medium,
        new.classification, new.culture, new.period, new.department,
        new.object_name, new.tags
    );
END;