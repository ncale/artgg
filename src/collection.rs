use anyhow::{Context, Result};
use rusqlite::Connection;
use std::env;

use crate::app::TasteProfile;

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
    pub image_url: String,
}

// ---------------------------------------------------------------------------
// DB location
// ---------------------------------------------------------------------------

pub fn find_collection_db() -> Option<String> {
    let home = env::var("HOME").unwrap_or_default();
    let candidates = vec![
        // Development: run from project root
        "./assets/collection.db".to_string(),
        // Installed alongside binary
        {
            if let Ok(exe) = std::env::current_exe() {
                exe.parent()
                    .map(|p| p.join("assets/collection.db").to_string_lossy().into_owned())
                    .unwrap_or_default()
            } else {
                String::new()
            }
        },
        // User data dir
        format!("{}/.local/share/artgg/collection.db", home),
    ];

    candidates
        .into_iter()
        .filter(|p| !p.is_empty())
        .find(|p| std::path::Path::new(p).exists())
}

// ---------------------------------------------------------------------------
// Query
// ---------------------------------------------------------------------------

/// How many artworks have a real (fetched) image URL in the DB.
/// Used to produce a helpful error when a build returns 0 results.
pub fn count_seeded(db_path: &str) -> Result<i64> {
    let conn = Connection::open(db_path)?;
    let n: i64 = conn.query_row(
        "SELECT COUNT(*) FROM artworks WHERE image_url LIKE 'https://images.metmuseum.org%'",
        [],
        |row| row.get(0),
    )?;
    Ok(n)
}

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
        "SELECT object_id, title, artist_display, date_display, medium, image_url
         FROM artworks
         WHERE is_public_domain = 1
           AND image_url LIKE 'https://images.metmuseum.org%'
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
    params.push(Value::Integer(count as i64));

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
        "SELECT object_id, title, artist_display, date_display, medium, image_url
         FROM artworks
         WHERE is_public_domain = 1
           AND image_url LIKE 'https://images.metmuseum.org%'
           {}ORDER BY RANDOM()
         LIMIT ?",
        dept_clause
    );

    let mut stmt = conn.prepare(&sql)?;

    let mut params: Vec<Value> = taste.departments.iter()
        .map(|d| Value::Text(d.clone()))
        .collect();
    params.push(Value::Integer(count as i64));

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
        image_url:      row.get::<_, Option<String>>(5)?.unwrap_or_default(),
    })
}
