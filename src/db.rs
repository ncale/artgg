use anyhow::Result;
use rusqlite::Connection;
use std::{env, fs};

use crate::app::{DisplayProfile, TasteProfile};

pub fn open() -> Result<Connection> {
    let home = env::var("HOME").unwrap_or_else(|_| "~".to_string());
    let dir = format!("{}/.local/share/artgg", home);
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
         );",
    )?;
    // Migrations — silently ignore "duplicate column" errors on existing DBs
    let _ = conn.execute("ALTER TABLE taste_profiles ADD COLUMN date_start INTEGER", []);
    let _ = conn.execute("ALTER TABLE taste_profiles ADD COLUMN date_end INTEGER", []);
    let _ = conn.execute(
        "ALTER TABLE taste_profiles ADD COLUMN is_public_domain INTEGER NOT NULL DEFAULT 0",
        [],
    );
    Ok(conn)
}

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
        .query_map([], |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
            ))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;

    let mut profiles = Vec::new();
    for (id, name, date_start, date_end, is_public_domain_int) in rows {
        let keywords = load_taste_profile_keywords(conn, id)?;
        profiles.push(TasteProfile {
            id,
            name,
            date_start,
            date_end,
            is_public_domain: is_public_domain_int != 0,
            keywords,
        });
    }
    Ok(profiles)
}

pub fn insert_taste_profile(conn: &Connection, name: &str) -> Result<i64> {
    conn.execute("INSERT INTO taste_profiles (name) VALUES (?1)", [name])?;
    Ok(conn.last_insert_rowid())
}

pub fn delete_taste_profile(conn: &Connection, id: i64) -> Result<()> {
    conn.execute("DELETE FROM taste_profiles WHERE id = ?1", [id])?;
    Ok(())
}

pub fn load_keywords(conn: &Connection) -> Result<Vec<(i64, String)>> {
    let mut stmt = conn.prepare("SELECT id, value FROM keywords ORDER BY value")?;
    let keywords = stmt
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(keywords)
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

pub fn add_taste_profile_keyword(
    conn: &Connection,
    profile_id: i64,
    keyword_id: i64,
) -> Result<()> {
    conn.execute(
        "INSERT OR IGNORE INTO taste_profile_keywords (profile_id, keyword_id) VALUES (?1, ?2)",
        rusqlite::params![profile_id, keyword_id],
    )?;
    Ok(())
}

pub fn remove_taste_profile_keyword(
    conn: &Connection,
    profile_id: i64,
    keyword_id: i64,
) -> Result<()> {
    conn.execute(
        "DELETE FROM taste_profile_keywords WHERE profile_id = ?1 AND keyword_id = ?2",
        rusqlite::params![profile_id, keyword_id],
    )?;
    Ok(())
}

pub fn load_display_profiles(conn: &Connection) -> Result<Vec<DisplayProfile>> {
    let mut stmt = conn.prepare("SELECT id, name FROM display_profiles ORDER BY id")?;
    let profiles = stmt
        .query_map([], |row| {
            Ok(DisplayProfile {
                id: row.get(0)?,
                name: row.get(1)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(profiles)
}

pub fn insert_display_profile(conn: &Connection, name: &str) -> Result<i64> {
    conn.execute("INSERT INTO display_profiles (name) VALUES (?1)", [name])?;
    Ok(conn.last_insert_rowid())
}

pub fn delete_display_profile(conn: &Connection, id: i64) -> Result<()> {
    conn.execute("DELETE FROM display_profiles WHERE id = ?1", [id])?;
    Ok(())
}
