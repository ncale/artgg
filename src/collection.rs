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

pub fn query_artworks(db_path: &str, taste: &TasteProfile, count: usize) -> Result<Vec<Artwork>> {
    let conn = Connection::open(db_path)
        .with_context(|| format!("Cannot open collection DB at '{}'", db_path))?;

    // Try to run with year_approx; gracefully fall back if column is missing.
    match query_inner(&conn, taste, count) {
        Ok(rows) => Ok(rows),
        Err(e) => {
            // If the error mentions year_approx, the DB was built with old schema.
            // Retry without date filtering.
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
    let date_start = taste.date_start;
    let date_end   = taste.date_end;

    let artworks = if taste.keywords.is_empty() {
        // Plain query with optional date range
        let mut stmt = conn.prepare(
            "SELECT object_id, title, artist_display, date_display, medium, image_url
             FROM artworks
             WHERE is_public_domain = 1
               AND image_url IS NOT NULL
               AND image_url != ''
               AND (?1 IS NULL OR year_approx IS NULL OR year_approx >= ?1)
               AND (?2 IS NULL OR year_approx IS NULL OR year_approx <= ?2)
             ORDER BY RANDOM()
             LIMIT ?3",
        )?;
        let rows = stmt.query_map(
            rusqlite::params![date_start, date_end, count as i64],
            row_to_artwork,
        )?.collect::<rusqlite::Result<Vec<_>>>()?;
        rows
    } else {
        // FTS query: match any of the keywords
        let fts_query = taste
            .keywords
            .iter()
            .map(|k| format!("\"{}\"", k.replace('"', "")))
            .collect::<Vec<_>>()
            .join(" OR ");

        let mut stmt = conn.prepare(
            "SELECT a.object_id, a.title, a.artist_display, a.date_display, a.medium, a.image_url
             FROM artworks a
             JOIN artworks_fts ON artworks_fts.rowid = a.object_id
             WHERE artworks_fts MATCH ?1
               AND a.is_public_domain = 1
               AND a.image_url IS NOT NULL
               AND a.image_url != ''
               AND (?2 IS NULL OR a.year_approx IS NULL OR a.year_approx >= ?2)
               AND (?3 IS NULL OR a.year_approx IS NULL OR a.year_approx <= ?3)
             ORDER BY RANDOM()
             LIMIT ?4",
        )?;
        let rows = stmt.query_map(
            rusqlite::params![fts_query, date_start, date_end, count as i64],
            row_to_artwork,
        )?.collect::<rusqlite::Result<Vec<_>>>()?;
        rows
    };

    Ok(artworks)
}

/// Fallback query that skips year_approx (old DB schema).
fn query_no_year(conn: &Connection, taste: &TasteProfile, count: usize) -> Result<Vec<Artwork>> {
    let artworks = if taste.keywords.is_empty() {
        let mut stmt = conn.prepare(
            "SELECT object_id, title, artist_display, date_display, medium, image_url
             FROM artworks
             WHERE is_public_domain = 1
               AND image_url IS NOT NULL AND image_url != ''
             ORDER BY RANDOM()
             LIMIT ?1",
        )?;
        let rows = stmt.query_map(rusqlite::params![count as i64], row_to_artwork)?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        rows
    } else {
        let fts_query = taste
            .keywords
            .iter()
            .map(|k| format!("\"{}\"", k.replace('"', "")))
            .collect::<Vec<_>>()
            .join(" OR ");

        let mut stmt = conn.prepare(
            "SELECT a.object_id, a.title, a.artist_display, a.date_display, a.medium, a.image_url
             FROM artworks a
             JOIN artworks_fts ON artworks_fts.rowid = a.object_id
             WHERE artworks_fts MATCH ?1
               AND a.is_public_domain = 1
               AND a.image_url IS NOT NULL AND a.image_url != ''
             ORDER BY RANDOM()
             LIMIT ?2",
        )?;
        let rows = stmt.query_map(rusqlite::params![fts_query, count as i64], row_to_artwork)?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        rows
    };

    Ok(artworks)
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
