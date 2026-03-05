use std::sync::mpsc::Sender;

use crate::app::{BuildMessage, DisplayProfile, TasteProfile};
use crate::collection;
use crate::db;
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

// ---------------------------------------------------------------------------
// Met API
// ---------------------------------------------------------------------------

#[derive(serde::Deserialize)]
struct MetObjectResponse {
    #[serde(rename = "primaryImageSmall", default)]
    primary_image_small: String,
    #[serde(rename = "primaryImage", default)]
    primary_image: String,
}

fn fetch_image_url(
    client: &reqwest::blocking::Client,
    object_id: i64,
) -> anyhow::Result<Option<String>> {
    let url = format!(
        "https://collectionapi.metmuseum.org/public/collection/v1/objects/{}",
        object_id
    );
    let resp = client.get(&url).send()?;

    if resp.status().as_u16() == 404 {
        return Ok(None);
    }
    if !resp.status().is_success() {
        return Err(anyhow::anyhow!("Met API error: HTTP {}", resp.status()));
    }

    let data: MetObjectResponse = resp.json()?;
    let image_url = if !data.primary_image_small.is_empty() {
        Some(data.primary_image_small)
    } else if !data.primary_image.is_empty() {
        Some(data.primary_image)
    } else {
        None
    };
    Ok(image_url)
}

// ---------------------------------------------------------------------------
// Build runner
// ---------------------------------------------------------------------------

/// Run the full build pipeline in a background thread.
/// Sends progress via `tx`; errors are non-fatal (skipped artworks).
pub fn run(params: BuildParams, tx: Sender<BuildMessage>) {
    macro_rules! send {
        ($msg:expr) => {
            let _ = tx.send($msg);
        };
    }

    // ── 1. Query artworks (2x headroom for filtering) ──────────────────────
    send!(BuildMessage::Phase("Querying collection…".to_string()));

    let candidates = match collection::query_artworks(
        &params.collection_db_path,
        &params.taste,
        params.count,
    ) {
        Ok(a) => a,
        Err(e) => {
            send!(BuildMessage::Error(format!(
                "Failed to query collection DB: {}",
                e
            )));
            return;
        }
    };

    if candidates.is_empty() {
        send!(BuildMessage::Error(
            "No artworks match your taste profile.".to_string()
        ));
        return;
    }

    let total = candidates.len();
    send!(BuildMessage::Phase(format!(
        "Found {} candidates — fetching URLs & rendering…",
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

    // ── 6. Open artgg.db for url_cache ─────────────────────────────────────
    let url_conn = match rusqlite::Connection::open(&params.artgg_db_path) {
        Ok(c) => c,
        Err(e) => {
            send!(BuildMessage::Error(format!("Cannot open artgg.db: {}", e)));
            return;
        }
    };

    // ── 7. Download + render loop ──────────────────────────────────────────
    let mut produced = 0usize;
    let mut skipped  = 0usize;

    for (i, artwork) in candidates.iter().enumerate() {
        if produced == params.count {
            break;
        }

        let cache_path  = format!("{}/{}.jpg", params.cache_dir, artwork.object_id);
        let output_path = format!("{}/{}.jpg", params.output_dir, artwork.object_id);

        // ── Resolve image URL (cache → Met API) ────────────────────────────
        let image_url: String = match db::get_url_cache(&url_conn, artwork.object_id) {
            Ok(Some(entry)) if !entry.is_valid => {
                // Permanently no image — skip.
                skipped += 1;
                continue;
            }
            Ok(Some(entry)) => {
                // Cache hit.
                match entry.image_url {
                    Some(u) => u,
                    None => { skipped += 1; continue; }
                }
            }
            _ => {
                // Cache miss — fetch from Met API.
                send!(BuildMessage::Progress {
                    current: i,
                    total,
                    message: format!("↓ Fetching URL: {}", artwork.title),
                });

                match fetch_image_url(&client, artwork.object_id) {
                    Ok(Some(url)) => {
                        let _ = db::upsert_url_cache_valid(&url_conn, artwork.object_id, &url);
                        std::thread::sleep(std::time::Duration::from_millis(400));
                        url
                    }
                    Ok(None) => {
                        let _ = db::upsert_url_cache_invalid(&url_conn, artwork.object_id);
                        skipped += 1;
                        continue;
                    }
                    Err(e) => {
                        send!(BuildMessage::Progress {
                            current: i,
                            total,
                            message: format!("✗ URL fetch failed ({}): {}", artwork.title, e),
                        });
                        skipped += 1;
                        std::thread::sleep(std::time::Duration::from_secs(1));
                        continue;
                    }
                }
            }
        };

        // ── Download image (if not disk-cached) ────────────────────────────
        if !std::path::Path::new(&cache_path).exists() {
            send!(BuildMessage::Progress {
                current: i,
                total,
                message: format!("↓ Downloading: {}", artwork.title),
            });

            match client.get(&image_url).send() {
                Ok(resp) if !resp.status().is_success() => {
                    let _ = db::upsert_url_cache_invalid(&url_conn, artwork.object_id);
                    send!(BuildMessage::Progress {
                        current: i,
                        total,
                        message: format!("✗ HTTP {} for: {}", resp.status(), artwork.title),
                    });
                    skipped += 1;
                    continue;
                }
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
                        std::thread::sleep(std::time::Duration::from_millis(400));
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
        } else {
            send!(BuildMessage::Progress {
                current: i,
                total,
                message: format!("✓ Cached: {}", artwork.title),
            });
        }

        // ── Render wallpaper ───────────────────────────────────────────────
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

    if produced < params.count {
        send!(BuildMessage::Progress {
            current: produced,
            total,
            message: format!(
                "Note: produced {} of {} requested (not enough valid artworks)",
                produced, params.count
            ),
        });
    }

    // ── 8. Record build history ────────────────────────────────────────────
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
