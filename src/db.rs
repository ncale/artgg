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
        "CREATE TABLE IF NOT EXISTS taste_profiles (
            id   INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL
         );
         CREATE TABLE IF NOT EXISTS display_profiles (
            id   INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL
         );",
    )?;
    Ok(conn)
}

pub fn load_taste_profiles(conn: &Connection) -> Result<Vec<TasteProfile>> {
    let mut stmt = conn.prepare("SELECT id, name FROM taste_profiles ORDER BY id")?;
    let profiles = stmt
        .query_map([], |row| {
            Ok(TasteProfile {
                id: row.get(0)?,
                name: row.get(1)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
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
