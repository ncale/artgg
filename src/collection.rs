use anyhow::{Context, Result};
use rusqlite::Connection;

use crate::app::TasteProfile;
use crate::db;

// ---------------------------------------------------------------------------
// Artwork model
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct Artwork {
    pub object_id: i64,
    pub title: String,
    pub artist_display: Option<String>,
    pub date_display: Option<String>,
    pub medium: Option<String>,
}

// ---------------------------------------------------------------------------
// DB location
// ---------------------------------------------------------------------------

pub fn find_collection_db() -> Option<std::path::PathBuf> {
    let candidates: Vec<std::path::PathBuf> = {
        let mut v = vec![
            // Development: run from project root
            std::path::PathBuf::from("./assets/collection.db"),
            // Installed alongside binary
            std::env::current_exe()
                .ok()
                .and_then(|p| p.parent().map(|d| d.join("assets/collection.db")))
                .unwrap_or_default(),
        ];
        // Runtime-downloaded: dirs-based data dir
        if let Ok(data) = db::data_dir() {
            v.push(data.join("collection.db"));
        }
        v
    };

    candidates
        .into_iter()
        .filter(|p| !p.as_os_str().is_empty())
        .find(|p| p.exists())
}

// ---------------------------------------------------------------------------
// Query
// ---------------------------------------------------------------------------

/// Return all distinct department names from the collection DB, sorted.
pub fn load_departments(db_path: &str) -> Result<Vec<String>> {
    let conn = Connection::open(db_path)
        .with_context(|| format!("Cannot open collection DB at '{}'", db_path))?;
    let mut stmt = conn.prepare(
        "SELECT DISTINCT department FROM artworks
         WHERE department IS NOT NULL AND department != ''
         ORDER BY department",
    )?;
    let depts = stmt
        .query_map([], |row| row.get(0))?
        .collect::<rusqlite::Result<Vec<String>>>()?;
    Ok(depts)
}

pub fn query_artworks(db_path: &str, taste: &TasteProfile, count: usize) -> Result<Vec<Artwork>> {
    let conn = Connection::open(db_path)
        .with_context(|| format!("Cannot open collection DB at '{}'", db_path))?;

    // Try with year_approx; fall back gracefully if old schema.
    match query_inner(&conn, taste, count) {
        Ok(rows) => Ok(rows),
        Err(e) => {
            let msg = e.to_string();
            if msg.contains("year_approx") || msg.contains("no such column") {
                query_no_year(&conn, taste, count)
                    .context("Query failed even without year_approx")
            } else {
                Err(e)
            }
        }
    }
}

fn query_inner(conn: &Connection, taste: &TasteProfile, count: usize) -> Result<Vec<Artwork>> {
    use rusqlite::types::Value;

    let ds = taste.date_start.map(Value::Integer).unwrap_or(Value::Null);
    let de = taste.date_end.map(Value::Integer).unwrap_or(Value::Null);

    // Build optional department IN clause.
    let dept_clause = if taste.departments.is_empty() {
        String::new()
    } else {
        let ph = std::iter::repeat("?")
            .take(taste.departments.len())
            .collect::<Vec<_>>()
            .join(",");
        format!("AND department IN ({}) ", ph)
    };

    let sql = format!(
        "SELECT object_id, title, artist_display, date_display, medium
         FROM artworks
         WHERE is_public_domain = 1
           AND (? IS NULL OR year_approx IS NULL OR year_approx >= ?)
           AND (? IS NULL OR year_approx IS NULL OR year_approx <= ?)
           {}ORDER BY RANDOM()
         LIMIT ?",
        dept_clause
    );

    let mut stmt = conn.prepare(&sql)?;

    // date_start appears twice (IS NULL check + comparison), same for date_end.
    let mut params: Vec<Value> = vec![ds.clone(), ds, de.clone(), de];
    for dept in &taste.departments {
        params.push(Value::Text(dept.clone()));
    }
    params.push(Value::Integer((count * 2) as i64));

    let rows = stmt
        .query_map(rusqlite::params_from_iter(params), row_to_artwork)?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}

/// Fallback query that skips year_approx (old DB schema).
fn query_no_year(conn: &Connection, taste: &TasteProfile, count: usize) -> Result<Vec<Artwork>> {
    use rusqlite::types::Value;

    let dept_clause = if taste.departments.is_empty() {
        String::new()
    } else {
        let ph = std::iter::repeat("?")
            .take(taste.departments.len())
            .collect::<Vec<_>>()
            .join(",");
        format!("AND department IN ({}) ", ph)
    };

    let sql = format!(
        "SELECT object_id, title, artist_display, date_display, medium
         FROM artworks
         WHERE is_public_domain = 1
           {}ORDER BY RANDOM()
         LIMIT ?",
        dept_clause
    );

    let mut stmt = conn.prepare(&sql)?;

    let mut params: Vec<Value> = taste.departments.iter()
        .map(|d| Value::Text(d.clone()))
        .collect();
    params.push(Value::Integer((count * 2) as i64));

    let rows = stmt
        .query_map(rusqlite::params_from_iter(params), row_to_artwork)?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}

fn row_to_artwork(row: &rusqlite::Row<'_>) -> rusqlite::Result<Artwork> {
    Ok(Artwork {
        object_id:      row.get(0)?,
        title:          row.get::<_, Option<String>>(1)?.unwrap_or_else(|| "Untitled".to_string()),
        artist_display: row.get(2)?,
        date_display:   row.get(3)?,
        medium:         row.get(4)?,
    })
}
