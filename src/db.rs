use anyhow::Result;
use rusqlite::Connection;
use std::{fs, path::PathBuf};

use crate::app::{DisplayProfile, TasteProfile};

mod embedded {
    refinery::embed_migrations!("migrations");
}

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
    let mut conn = Connection::open(&path)?;
    conn.execute_batch("PRAGMA foreign_keys = ON;")?;
    embedded::migrations::runner().run(&mut conn)?;
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
// Image cache helpers
// ---------------------------------------------------------------------------

pub fn compute_image_cache_size() -> anyhow::Result<u64> {
    let images_dir = cache_dir()?.join("images");
    if !images_dir.exists() {
        return Ok(0);
    }
    let mut total = 0u64;
    for entry in fs::read_dir(&images_dir)? {
        if let Ok(meta) = entry?.metadata() {
            total += meta.len();
        }
    }
    Ok(total)
}

pub fn format_cache_size(bytes: u64) -> String {
    if bytes >= 1_073_741_824 {
        format!("{:.1} GB", bytes as f64 / 1_073_741_824.0)
    } else if bytes >= 1_048_576 {
        format!("{:.1} MB", bytes as f64 / 1_048_576.0)
    } else if bytes >= 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{} B", bytes)
    }
}

pub fn clear_image_cache(conn: &Connection) -> anyhow::Result<()> {
    let images_dir = cache_dir()?.join("images");
    if images_dir.exists() {
        fs::remove_dir_all(&images_dir)?;
    }
    conn.execute("DELETE FROM url_cache", [])?;
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

#[cfg(test)]
mod tests {
    use super::*;

    fn test_conn() -> Connection {
        let mut conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("PRAGMA foreign_keys = ON;").unwrap();
        embedded::migrations::runner().run(&mut conn).unwrap();
        conn
    }

    // ── Migrations ──────────────────────────────────────────────────────────

    #[test]
    fn migrations_run_on_fresh_db() {
        let conn = test_conn();
        let tables: Vec<String> = {
            let mut s = conn.prepare(
                "SELECT name FROM sqlite_master WHERE type='table' ORDER BY name"
            ).unwrap();
            s.query_map([], |r| r.get(0)).unwrap()
                .map(|r| r.unwrap()).collect()
        };
        for t in &["builds", "display_profiles", "refinery_schema_history",
                   "taste_profile_departments", "taste_profiles", "url_cache"] {
            assert!(tables.contains(&t.to_string()), "missing table: {t}");
        }
    }

    // ── Taste profiles ───────────────────────────────────────────────────────

    #[test]
    fn taste_profile_insert_and_load() {
        let conn = test_conn();
        let id = insert_taste_profile(&conn, "Modern Art", Some(1900), Some(2000), false).unwrap();
        add_taste_profile_department(&conn, id, "Paintings").unwrap();
        add_taste_profile_department(&conn, id, "Sculpture").unwrap();

        let profiles = load_taste_profiles(&conn).unwrap();
        assert_eq!(profiles.len(), 1);
        let p = &profiles[0];
        assert_eq!(p.name, "Modern Art");
        assert_eq!(p.date_start, Some(1900));
        assert_eq!(p.date_end, Some(2000));
        assert!(!p.is_public_domain);
        assert_eq!(p.departments, vec!["Paintings", "Sculpture"]);
    }

    #[test]
    fn taste_profile_update() {
        let conn = test_conn();
        let id = insert_taste_profile(&conn, "Old", None, None, false).unwrap();
        update_taste_profile_fields(&conn, id, Some(1800), Some(1900), true).unwrap();

        let profiles = load_taste_profiles(&conn).unwrap();
        assert_eq!(profiles[0].date_start, Some(1800));
        assert_eq!(profiles[0].date_end, Some(1900));
        assert!(profiles[0].is_public_domain);
    }

    #[test]
    fn taste_profile_delete() {
        let conn = test_conn();
        let id = insert_taste_profile(&conn, "Temp", None, None, true).unwrap();
        assert_eq!(load_taste_profiles(&conn).unwrap().len(), 1);
        delete_taste_profile(&conn, id).unwrap();
        assert_eq!(load_taste_profiles(&conn).unwrap().len(), 0);
    }

    #[test]
    fn taste_profile_department_toggle() {
        let conn = test_conn();
        let id = insert_taste_profile(&conn, "Test", None, None, true).unwrap();
        add_taste_profile_department(&conn, id, "Prints").unwrap();
        add_taste_profile_department(&conn, id, "Prints").unwrap(); // duplicate — ignored
        remove_taste_profile_department(&conn, id, "Prints").unwrap();

        let profiles = load_taste_profiles(&conn).unwrap();
        assert!(profiles[0].departments.is_empty());
    }

    // ── Display profiles ─────────────────────────────────────────────────────

    #[test]
    fn display_profile_insert_and_load() {
        let conn = test_conn();
        insert_display_profile(
            &conn, "4K", "#000000", "", "horizontal", 3840, 2160,
            "#ffffff", "#000000", 80,
        ).unwrap();

        let profiles = load_display_profiles(&conn).unwrap();
        assert_eq!(profiles.len(), 1);
        let p = &profiles[0];
        assert_eq!(p.name, "4K");
        assert_eq!(p.canvas_width, 3840);
        assert_eq!(p.canvas_height, 2160);
        assert_eq!(p.placard_opacity, 80);
    }

    #[test]
    fn display_profile_update() {
        let conn = test_conn();
        let id = insert_display_profile(
            &conn, "Old", "#000000", "", "horizontal", 1920, 1080,
            "#ffffff", "#000000", 90,
        ).unwrap();
        update_display_profile_fields(
            &conn, id, "#111111", "", "vertical", 1080, 1920,
            "#eeeeee", "#111111", 50,
        ).unwrap();

        let p = &load_display_profiles(&conn).unwrap()[0];
        assert_eq!(p.wallpaper_color, "#111111");
        assert_eq!(p.orientation, "vertical");
        assert_eq!(p.canvas_width, 1080);
        assert_eq!(p.placard_opacity, 50);
    }

    #[test]
    fn display_profile_delete() {
        let conn = test_conn();
        insert_display_profile(
            &conn, "Temp", "#000000", "", "horizontal", 1920, 1080,
            "#ffffff", "#000000", 90,
        ).unwrap();
        let id = load_display_profiles(&conn).unwrap()[0].id;
        delete_display_profile(&conn, id).unwrap();
        assert_eq!(load_display_profiles(&conn).unwrap().len(), 0);
    }

    // ── URL cache ────────────────────────────────────────────────────────────

    #[test]
    fn url_cache_valid_entry() {
        let conn = test_conn();
        upsert_url_cache_valid(&conn, 42, "https://example.com/image.jpg").unwrap();

        let entry = get_url_cache(&conn, 42).unwrap().unwrap();
        assert!(entry.is_valid);
        assert_eq!(entry.image_url.as_deref(), Some("https://example.com/image.jpg"));
    }

    #[test]
    fn url_cache_invalid_entry() {
        let conn = test_conn();
        upsert_url_cache_invalid(&conn, 99).unwrap();

        let entry = get_url_cache(&conn, 99).unwrap().unwrap();
        assert!(!entry.is_valid);
        assert!(entry.image_url.is_none());
    }

    #[test]
    fn url_cache_upsert_overwrites() {
        let conn = test_conn();
        upsert_url_cache_valid(&conn, 7, "https://example.com/old.jpg").unwrap();
        upsert_url_cache_invalid(&conn, 7).unwrap();

        let entry = get_url_cache(&conn, 7).unwrap().unwrap();
        assert!(!entry.is_valid);
        assert!(entry.image_url.is_none());
    }

    #[test]
    fn url_cache_miss_returns_none() {
        let conn = test_conn();
        assert!(get_url_cache(&conn, 999).unwrap().is_none());
    }

    // ── Seed defaults ────────────────────────────────────────────────────────

    #[test]
    fn seed_defaults_populates_empty_db() {
        let conn = test_conn();
        seed_defaults(&conn).unwrap();

        let taste = load_taste_profiles(&conn).unwrap();
        assert_eq!(taste.len(), 1);
        assert_eq!(taste[0].name, "European Paintings");

        let display = load_display_profiles(&conn).unwrap();
        assert_eq!(display.len(), 1);
        assert_eq!(display[0].name, "Default");
    }

    #[test]
    fn seed_defaults_is_idempotent() {
        let conn = test_conn();
        seed_defaults(&conn).unwrap();
        seed_defaults(&conn).unwrap();

        assert_eq!(load_taste_profiles(&conn).unwrap().len(), 1);
        assert_eq!(load_display_profiles(&conn).unwrap().len(), 1);
    }

    // ── format_cache_size ────────────────────────────────────────────────────

    #[test]
    fn format_cache_size_units() {
        assert_eq!(format_cache_size(0), "0 B");
        assert_eq!(format_cache_size(512), "512 B");
        assert_eq!(format_cache_size(1024), "1.0 KB");
        assert_eq!(format_cache_size(1_536), "1.5 KB");
        assert_eq!(format_cache_size(1_048_576), "1.0 MB");
        assert_eq!(format_cache_size(1_073_741_824), "1.0 GB");
    }
}
