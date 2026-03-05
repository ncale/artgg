use anyhow::Result;
use rusqlite::Connection;
use std::{fs, path::PathBuf};

use crate::app::{DisplayProfile, TasteProfile};

// ---------------------------------------------------------------------------
// Path helpers
// ---------------------------------------------------------------------------

pub fn data_dir() -> anyhow::Result<PathBuf> {
    let dir = dirs::data_local_dir()
        .ok_or_else(|| anyhow::anyhow!("Cannot determine user data directory"))?
        .join("artgg");
    fs::create_dir_all(&dir)?;
    Ok(dir)
}

pub fn cache_dir() -> anyhow::Result<PathBuf> {
    let dir = dirs::cache_dir()
        .ok_or_else(|| anyhow::anyhow!("Cannot determine cache directory"))?
        .join("artgg");
    fs::create_dir_all(&dir)?;
    Ok(dir)
}

pub fn db_path() -> anyhow::Result<String> {
    Ok(data_dir()?.join("artgg.db").to_string_lossy().into_owned())
}

// ---------------------------------------------------------------------------
// Open + migrate
// ---------------------------------------------------------------------------

pub fn open() -> Result<Connection> {
    let path = db_path()?;
    let conn = Connection::open(&path)?;
    conn.execute_batch(
        "PRAGMA foreign_keys = ON;
         CREATE TABLE IF NOT EXISTS taste_profiles (
            id   INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL
         );
         CREATE TABLE IF NOT EXISTS display_profiles (
            id   INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL
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
         );",
    )?;

    // taste_profiles migrations
    let _ = conn.execute("ALTER TABLE taste_profiles ADD COLUMN date_start INTEGER", []);
    let _ = conn.execute("ALTER TABLE taste_profiles ADD COLUMN date_end INTEGER", []);
    let _ = conn.execute(
        "ALTER TABLE taste_profiles ADD COLUMN is_public_domain INTEGER NOT NULL DEFAULT 0",
        [],
    );

    // display_profiles migrations
    let _ = conn.execute(
        "ALTER TABLE display_profiles ADD COLUMN wallpaper_color TEXT NOT NULL DEFAULT '#1A1A2E'",
        [],
    );
    let _ = conn.execute(
        "ALTER TABLE display_profiles ADD COLUMN frame_style TEXT NOT NULL DEFAULT ''",
        [],
    );
    let _ = conn.execute(
        "ALTER TABLE display_profiles ADD COLUMN orientation TEXT NOT NULL DEFAULT 'horizontal'",
        [],
    );
    // Replace aspect_ratio with explicit pixel dimensions.
    let _ = conn.execute(
        "ALTER TABLE display_profiles ADD COLUMN canvas_width  INTEGER NOT NULL DEFAULT 1920",
        [],
    );
    let _ = conn.execute(
        "ALTER TABLE display_profiles ADD COLUMN canvas_height INTEGER NOT NULL DEFAULT 1080",
        [],
    );
    let _ = conn.execute(
        "ALTER TABLE display_profiles ADD COLUMN placard_color TEXT NOT NULL DEFAULT '#F5F1E8'",
        [],
    );
    let _ = conn.execute(
        "ALTER TABLE display_profiles ADD COLUMN placard_text_color TEXT NOT NULL DEFAULT '#1E160C'",
        [],
    );
    let _ = conn.execute(
        "ALTER TABLE display_profiles ADD COLUMN placard_opacity INTEGER NOT NULL DEFAULT 90",
        [],
    );

    seed_defaults(&conn)?;

    Ok(conn)
}

// ---------------------------------------------------------------------------
// Taste profiles
// ---------------------------------------------------------------------------

fn load_taste_profile_departments(conn: &Connection, profile_id: i64) -> Result<Vec<String>> {
    let mut stmt = conn.prepare(
        "SELECT department FROM taste_profile_departments
         WHERE profile_id = ?1
         ORDER BY department",
    )?;
    let depts = stmt
        .query_map([profile_id], |row| row.get(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(depts)
}

pub fn load_taste_profiles(conn: &Connection) -> Result<Vec<TasteProfile>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, date_start, date_end, is_public_domain FROM taste_profiles ORDER BY id",
    )?;
    let rows: Vec<(i64, String, Option<i64>, Option<i64>, i64)> = stmt
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?)))?
        .collect::<rusqlite::Result<Vec<_>>>()?;

    let mut profiles = Vec::new();
    for (id, name, date_start, date_end, pd_int) in rows {
        let departments = load_taste_profile_departments(conn, id)?;
        profiles.push(TasteProfile {
            id, name, date_start, date_end,
            is_public_domain: pd_int != 0,
            departments,
        });
    }
    Ok(profiles)
}

pub fn insert_taste_profile(
    conn: &Connection,
    name: &str,
    date_start: Option<i64>,
    date_end: Option<i64>,
    is_public_domain: bool,
) -> Result<i64> {
    conn.execute(
        "INSERT INTO taste_profiles (name, date_start, date_end, is_public_domain) VALUES (?1, ?2, ?3, ?4)",
        rusqlite::params![name, date_start, date_end, is_public_domain as i64],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn delete_taste_profile(conn: &Connection, id: i64) -> Result<()> {
    conn.execute("DELETE FROM taste_profiles WHERE id = ?1", [id])?;
    Ok(())
}

pub fn update_taste_profile_fields(
    conn: &Connection,
    id: i64,
    date_start: Option<i64>,
    date_end: Option<i64>,
    is_public_domain: bool,
) -> Result<()> {
    conn.execute(
        "UPDATE taste_profiles SET date_start = ?1, date_end = ?2, is_public_domain = ?3 WHERE id = ?4",
        rusqlite::params![date_start, date_end, is_public_domain as i64, id],
    )?;
    Ok(())
}

pub fn add_taste_profile_department(conn: &Connection, profile_id: i64, department: &str) -> Result<()> {
    conn.execute(
        "INSERT OR IGNORE INTO taste_profile_departments (profile_id, department) VALUES (?1, ?2)",
        rusqlite::params![profile_id, department],
    )?;
    Ok(())
}

pub fn remove_taste_profile_department(conn: &Connection, profile_id: i64, department: &str) -> Result<()> {
    conn.execute(
        "DELETE FROM taste_profile_departments WHERE profile_id = ?1 AND department = ?2",
        rusqlite::params![profile_id, department],
    )?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Display profiles
// ---------------------------------------------------------------------------

pub fn load_display_profiles(conn: &Connection) -> Result<Vec<DisplayProfile>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, wallpaper_color, frame_style, orientation, canvas_width, canvas_height,
                placard_color, placard_text_color, placard_opacity
         FROM display_profiles ORDER BY id",
    )?;
    let profiles = stmt
        .query_map([], |row| {
            Ok(DisplayProfile {
                id:                  row.get(0)?,
                name:                row.get(1)?,
                wallpaper_color:     row.get(2)?,
                frame_style:         row.get(3)?,
                orientation:         row.get(4)?,
                canvas_width:        row.get::<_, i64>(5)? as u32,
                canvas_height:       row.get::<_, i64>(6)? as u32,
                placard_color:       row.get(7)?,
                placard_text_color:  row.get(8)?,
                placard_opacity:     row.get::<_, i64>(9)? as u32,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(profiles)
}

pub fn insert_display_profile(
    conn: &Connection,
    name: &str,
    wallpaper_color: &str,
    frame_style: &str,
    orientation: &str,
    canvas_width: u32,
    canvas_height: u32,
    placard_color: &str,
    placard_text_color: &str,
    placard_opacity: u32,
) -> Result<i64> {
    conn.execute(
        "INSERT INTO display_profiles (name, wallpaper_color, frame_style, orientation,
                                       canvas_width, canvas_height,
                                       placard_color, placard_text_color, placard_opacity)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        rusqlite::params![
            name, wallpaper_color, frame_style, orientation,
            canvas_width as i64, canvas_height as i64,
            placard_color, placard_text_color, placard_opacity as i64,
        ],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn delete_display_profile(conn: &Connection, id: i64) -> Result<()> {
    conn.execute("DELETE FROM display_profiles WHERE id = ?1", [id])?;
    Ok(())
}

pub fn update_display_profile_fields(
    conn: &Connection,
    id: i64,
    wallpaper_color: &str,
    frame_style: &str,
    orientation: &str,
    canvas_width: u32,
    canvas_height: u32,
    placard_color: &str,
    placard_text_color: &str,
    placard_opacity: u32,
) -> Result<()> {
    conn.execute(
        "UPDATE display_profiles
         SET wallpaper_color = ?1, frame_style = ?2, orientation = ?3,
             canvas_width = ?4, canvas_height = ?5,
             placard_color = ?6, placard_text_color = ?7, placard_opacity = ?8
         WHERE id = ?9",
        rusqlite::params![
            wallpaper_color, frame_style, orientation,
            canvas_width as i64, canvas_height as i64,
            placard_color, placard_text_color, placard_opacity as i64,
            id,
        ],
    )?;
    Ok(())
}

// ---------------------------------------------------------------------------
// URL cache
// ---------------------------------------------------------------------------

pub struct UrlCacheEntry {
    #[allow(dead_code)]
    pub object_id: i64,
    pub image_url: Option<String>,
    pub is_valid: bool,
}

pub fn get_url_cache(conn: &Connection, object_id: i64) -> Result<Option<UrlCacheEntry>> {
    let mut stmt = conn.prepare(
        "SELECT object_id, image_url, is_valid FROM url_cache WHERE object_id = ?1",
    )?;
    let mut rows = stmt.query_map([object_id], |row| {
        Ok(UrlCacheEntry {
            object_id: row.get(0)?,
            image_url: row.get(1)?,
            is_valid: row.get::<_, i64>(2)? != 0,
        })
    })?;
    Ok(rows.next().transpose()?)
}

pub fn upsert_url_cache_valid(conn: &Connection, object_id: i64, image_url: &str) -> Result<()> {
    conn.execute(
        "INSERT INTO url_cache (object_id, image_url, is_valid, fetched_at)
         VALUES (?1, ?2, 1, unixepoch())
         ON CONFLICT(object_id) DO UPDATE SET image_url = excluded.image_url,
             is_valid = 1, fetched_at = unixepoch()",
        rusqlite::params![object_id, image_url],
    )?;
    Ok(())
}

pub fn upsert_url_cache_invalid(conn: &Connection, object_id: i64) -> Result<()> {
    conn.execute(
        "INSERT INTO url_cache (object_id, image_url, is_valid, fetched_at)
         VALUES (?1, NULL, 0, unixepoch())
         ON CONFLICT(object_id) DO UPDATE SET image_url = NULL,
             is_valid = 0, fetched_at = unixepoch()",
        rusqlite::params![object_id],
    )?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Default seed data
// ---------------------------------------------------------------------------

fn seed_defaults(conn: &Connection) -> Result<()> {
    let taste_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM taste_profiles", [], |r| r.get(0),
    )?;
    if taste_count == 0 {
        let id = insert_taste_profile(conn, "European Paintings", None, None, true)?;
        add_taste_profile_department(conn, id, "European Paintings")?;
    }

    let display_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM display_profiles", [], |r| r.get(0),
    )?;
    if display_count == 0 {
        insert_display_profile(
            conn, "Default", "#1a1a1a", "", "horizontal", 1920, 1080,
            "#4a4a4a", "#ffffff", 90,
        )?;
    }

    Ok(())
}
