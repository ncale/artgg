use anyhow::Result;
use rusqlite::Connection;
use std::{env, fs};

use crate::app::{DisplayProfile, TasteProfile};

// ---------------------------------------------------------------------------
// Path helpers
// ---------------------------------------------------------------------------

pub fn db_path() -> String {
    let home = env::var("HOME").unwrap_or_else(|_| "~".to_string());
    format!("{}/.local/share/artgg/artgg.db", home)
}

// ---------------------------------------------------------------------------
// Open + migrate
// ---------------------------------------------------------------------------

pub fn open() -> Result<Connection> {
    let home = env::var("HOME").unwrap_or_else(|_| "~".to_string());
    let dir  = format!("{}/.local/share/artgg", home);
    fs::create_dir_all(&dir)?;
    let path = format!("{}/artgg.db", dir);
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
         CREATE TABLE IF NOT EXISTS keywords (
            id    INTEGER PRIMARY KEY AUTOINCREMENT,
            value TEXT NOT NULL UNIQUE
         );
         CREATE TABLE IF NOT EXISTS taste_profile_keywords (
            profile_id INTEGER NOT NULL REFERENCES taste_profiles(id) ON DELETE CASCADE,
            keyword_id INTEGER NOT NULL REFERENCES keywords(id) ON DELETE CASCADE,
            PRIMARY KEY (profile_id, keyword_id)
         );
         CREATE TABLE IF NOT EXISTS builds (
            id                 INTEGER PRIMARY KEY AUTOINCREMENT,
            created_at         INTEGER NOT NULL DEFAULT (unixepoch()),
            taste_profile_id   INTEGER,
            display_profile_id INTEGER,
            output_dir         TEXT NOT NULL DEFAULT '',
            count              INTEGER NOT NULL DEFAULT 0
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

    Ok(conn)
}

// ---------------------------------------------------------------------------
// Taste profiles
// ---------------------------------------------------------------------------

fn load_taste_profile_keywords(conn: &Connection, profile_id: i64) -> Result<Vec<String>> {
    let mut stmt = conn.prepare(
        "SELECT k.value FROM keywords k
         JOIN taste_profile_keywords tpk ON k.id = tpk.keyword_id
         WHERE tpk.profile_id = ?1
         ORDER BY k.value",
    )?;
    let keywords = stmt
        .query_map([profile_id], |row| row.get(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(keywords)
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
        let keywords = load_taste_profile_keywords(conn, id)?;
        profiles.push(TasteProfile {
            id, name, date_start, date_end,
            is_public_domain: pd_int != 0,
            keywords,
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

pub fn add_taste_profile_keyword(conn: &Connection, profile_id: i64, keyword_id: i64) -> Result<()> {
    conn.execute(
        "INSERT OR IGNORE INTO taste_profile_keywords (profile_id, keyword_id) VALUES (?1, ?2)",
        rusqlite::params![profile_id, keyword_id],
    )?;
    Ok(())
}

pub fn remove_taste_profile_keyword(conn: &Connection, profile_id: i64, keyword_id: i64) -> Result<()> {
    conn.execute(
        "DELETE FROM taste_profile_keywords WHERE profile_id = ?1 AND keyword_id = ?2",
        rusqlite::params![profile_id, keyword_id],
    )?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Keywords
// ---------------------------------------------------------------------------

pub fn load_keywords(conn: &Connection) -> Result<Vec<(i64, String)>> {
    let mut stmt = conn.prepare("SELECT id, value FROM keywords ORDER BY value")?;
    let keywords = stmt
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(keywords)
}

// ---------------------------------------------------------------------------
// Display profiles
// ---------------------------------------------------------------------------

pub fn load_display_profiles(conn: &Connection) -> Result<Vec<DisplayProfile>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, wallpaper_color, frame_style, orientation, canvas_width, canvas_height
         FROM display_profiles ORDER BY id",
    )?;
    let profiles = stmt
        .query_map([], |row| {
            Ok(DisplayProfile {
                id:             row.get(0)?,
                name:           row.get(1)?,
                wallpaper_color: row.get(2)?,
                frame_style:    row.get(3)?,
                orientation:    row.get(4)?,
                canvas_width:   row.get::<_, i64>(5)? as u32,
                canvas_height:  row.get::<_, i64>(6)? as u32,
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
) -> Result<i64> {
    conn.execute(
        "INSERT INTO display_profiles (name, wallpaper_color, frame_style, orientation, canvas_width, canvas_height)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        rusqlite::params![name, wallpaper_color, frame_style, orientation, canvas_width as i64, canvas_height as i64],
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
) -> Result<()> {
    conn.execute(
        "UPDATE display_profiles
         SET wallpaper_color = ?1, frame_style = ?2, orientation = ?3,
             canvas_width = ?4, canvas_height = ?5
         WHERE id = ?6",
        rusqlite::params![wallpaper_color, frame_style, orientation,
                          canvas_width as i64, canvas_height as i64, id],
    )?;
    Ok(())
}
