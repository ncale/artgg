use std::sync::mpsc::Sender;

use crate::app::{BuildMessage, DisplayProfile, TasteProfile};
use crate::collection;
use crate::renderer;

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

pub struct BuildParams {
    pub taste:              TasteProfile,
    pub display:            DisplayProfile,
    pub output_dir:         String,
    pub count:              usize,
    pub collection_db_path: String,
    pub cache_dir:          String,
    pub artgg_db_path:      String,
}

/// Run the full build pipeline in a background thread.
/// Sends progress via `tx`; errors are non-fatal (skipped artworks).
pub fn run(params: BuildParams, tx: Sender<BuildMessage>) {
    macro_rules! send {
        ($msg:expr) => {
            let _ = tx.send($msg);
        };
    }

    // ── 1. Query artworks ──────────────────────────────────────────────────
    send!(BuildMessage::Phase("Querying collection…".to_string()));

    let artworks = match collection::query_artworks(
        &params.collection_db_path,
        &params.taste,
        params.count,
    ) {
        Ok(a) => a,
        Err(e) => {
            send!(BuildMessage::Error(format!(
                "Failed to query collection DB: {}\n\
                 Make sure collection.db exists and 'fetch-images' has been run.",
                e
            )));
            return;
        }
    };

    if artworks.is_empty() {
        // Try to give a specific reason.
        let seeded = collection::count_seeded(&params.collection_db_path).unwrap_or(0);
        let msg = if seeded == 0 {
            "collection.db has no image URLs yet.\n\
             Run:  python scripts/build_db.py fetch-images\n\
             (this takes a while — it's resumable with Ctrl+C)"
                .to_string()
        } else {
            format!(
                "No artworks match your taste profile ({} images available in collection).\n\
                 Try removing date or keyword filters.",
                seeded
            )
        };
        send!(BuildMessage::Error(msg));
        return;
    }

    let total = artworks.len();
    send!(BuildMessage::Phase(format!(
        "Found {} artworks — downloading & rendering…",
        total
    )));

    // ── 2. Create directories ──────────────────────────────────────────────
    for dir in [&params.cache_dir, &params.output_dir] {
        if let Err(e) = std::fs::create_dir_all(dir) {
            send!(BuildMessage::Error(format!(
                "Cannot create directory '{}': {}",
                dir, e
            )));
            return;
        }
    }

    // ── 3. Load font (once) ────────────────────────────────────────────────
    let font = renderer::load_font();
    if font.is_none() {
        send!(BuildMessage::Progress {
            current: 0,
            total,
            message: "⚠ No font found — placards will be text-free.\
                      Place a TTF font at assets/fonts/font.ttf"
                .to_string(),
        });
    }

    // ── 4. Canvas size & background ────────────────────────────────────────
    let canvas_w        = params.display.canvas_width;
    let canvas_h        = params.display.canvas_height;
    let bg_color        = renderer::parse_hex_color(&params.display.wallpaper_color);
    let placard_color   = renderer::parse_hex_color(&params.display.placard_color);
    let placard_text    = renderer::parse_hex_color(&params.display.placard_text_color);
    let placard_opacity = params.display.placard_opacity.min(100) as f32 / 100.0;

    // ── 5. HTTP client ─────────────────────────────────────────────────────
    let client = match reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .user_agent("artgg/0.1 (wallpaper generator)")
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            send!(BuildMessage::Error(format!("HTTP client init failed: {}", e)));
            return;
        }
    };

    // ── 6. Download + render loop ──────────────────────────────────────────
    let mut produced = 0usize;
    let mut skipped  = 0usize;

    for (i, artwork) in artworks.iter().enumerate() {
        let cache_path  = format!("{}/{}.jpg", params.cache_dir, artwork.object_id);
        let output_path = format!("{}/{}.jpg", params.output_dir, artwork.object_id);

        // Download if not already cached.
        if !std::path::Path::new(&cache_path).exists() {
            send!(BuildMessage::Progress {
                current: i,
                total,
                message: format!("↓ Downloading: {}", artwork.title),
            });

            match client.get(&artwork.image_url).send() {
                Ok(resp) => match resp.bytes() {
                    Ok(bytes) => {
                        if let Err(e) = std::fs::write(&cache_path, &bytes) {
                            send!(BuildMessage::Progress {
                                current: i,
                                total,
                                message: format!("✗ Save failed ({}): {}", artwork.title, e),
                            });
                            skipped += 1;
                            continue;
                        }
                    }
                    Err(e) => {
                        send!(BuildMessage::Progress {
                            current: i,
                            total,
                            message: format!("✗ Download failed ({}): {}", artwork.title, e),
                        });
                        skipped += 1;
                        std::thread::sleep(std::time::Duration::from_secs(1));
                        continue;
                    }
                },
                Err(e) => {
                    send!(BuildMessage::Progress {
                        current: i,
                        total,
                        message: format!("✗ Request failed ({}): {}", artwork.title, e),
                    });
                    skipped += 1;
                    std::thread::sleep(std::time::Duration::from_secs(1));
                    continue;
                }
            }

            // Polite rate-limiting between downloads.
            std::thread::sleep(std::time::Duration::from_millis(400));
        } else {
            send!(BuildMessage::Progress {
                current: i,
                total,
                message: format!("✓ Cached: {}", artwork.title),
            });
        }

        // Render wallpaper.
        send!(BuildMessage::Progress {
            current: i,
            total,
            message: format!("⚙ Rendering: {}", artwork.title),
        });

        match renderer::render_wallpaper(
            &cache_path,
            artwork,
            canvas_w,
            canvas_h,
            bg_color,
            placard_color,
            placard_text,
            placard_opacity,
            font.as_ref(),
        ) {
            Ok(img) => match img.save(&output_path) {
                Ok(_) => {
                    produced += 1;
                    send!(BuildMessage::Progress {
                        current: i + 1,
                        total,
                        message: format!("✓ {}", artwork.title),
                    });
                }
                Err(e) => {
                    send!(BuildMessage::Progress {
                        current: i + 1,
                        total,
                        message: format!("✗ Save failed ({}): {}", artwork.title, e),
                    });
                    skipped += 1;
                }
            },
            Err(e) => {
                send!(BuildMessage::Progress {
                    current: i + 1,
                    total,
                    message: format!("✗ Render failed ({}): {}", artwork.title, e),
                });
                skipped += 1;
            }
        }
    }

    // ── 7. Record build history ────────────────────────────────────────────
    let _ = record_build(&params, produced as i64);

    send!(BuildMessage::Done {
        produced,
        skipped,
        output_dir: params.output_dir.clone(),
    });
}

// ---------------------------------------------------------------------------
// Build history
// ---------------------------------------------------------------------------

fn record_build(params: &BuildParams, count: i64) -> anyhow::Result<()> {
    let conn = rusqlite::Connection::open(&params.artgg_db_path)?;
    conn.execute(
        "INSERT INTO builds (created_at, taste_profile_id, display_profile_id, output_dir, count)
         VALUES (unixepoch(), ?1, ?2, ?3, ?4)",
        rusqlite::params![
            params.taste.id,
            params.display.id,
            params.output_dir,
            count
        ],
    )?;
    Ok(())
}
