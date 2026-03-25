ALTER TABLE taste_profiles DROP COLUMN is_public_domain;

CREATE TABLE IF NOT EXISTS taste_profile_artists (
    profile_id INTEGER NOT NULL REFERENCES taste_profiles(id) ON DELETE CASCADE,
    artist     TEXT NOT NULL,
    PRIMARY KEY (profile_id, artist)
);
