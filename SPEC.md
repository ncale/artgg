
# 🛠️ SPECIFICATION.md

`artgg` is a cross-platform CLI tool that statically generates desktop wallpapers consisting of classical artwork with museum-style placards.

## 1. System Architecture

artgg follows a **Stateless Pipeline** model. Data moves from external APIs to a local cache, is transformed by a renderer, and is finalized as a system-compatible image.

## 2. Data Management (SQLite)

A local SQLite database (`state.db`) tracks the lifecycle of every artwork.
**Table: `artworks`**

* `id`: Unique identifier (Source + Remote ID).
* `status`: `draft` (downloaded), `built` (rendered), `favorite`, `blacklisted`.
* `metadata`: JSON blob containing Title, Artist, Years, and Blurb.
* `last_shown`: Timestamp to prevent repetitive cycling.

## 3. Command Definition

| Command | Action |
| :--- | :--- |
| `init` | Creates `config.toml` in the user's config dir. All non-essential lines are commented out. |
| `fetch` | Queries APIs. Downloads raw images to `~/.cache/artgg/pool/`. Populates `state.db`. |
| `build` | Picks a `draft` from the DB. Renders the frame and placard. Saves to `~/.local/share/artgg/gallery/`. |
| `prune` | Deletes raw files in `pool/` based on `retention_days` and `pool_size` limits. |

## 4. Build & Rendering Requirements

The goal is a "Gallery Aesthetic" rather than a "Full-Screen Stretch."

### **A. The Matte (Frame)**

* The painting must be scaled to fit within a **70% inner-bound** of the target resolution.

* Use the "Rule of Thirds" for the placard, placing the text at the bottom-right or centered-bottom (outside the artwork's bounds but within the "matte").

* It must be **centered** horizontally and vertically (or slightly offset upward to leave room for the placard).
* **No cropping or blurring.** The original aspect ratio of the image must be preserved.

### **B. The Placard (Typography)**

* **Font:** Bundled high-quality Serif (e.g., *Libre Baskerville*).

* **Fields:**
  * `{artist_name}` ({birth}–{death})
  * *{work_title}*, {year}
  * {description_blurb}
* **Placement:** Configurable (Default: Center-Bottom below the art).

## 5. Configuration Options (`config.toml`)

Users can configure the following "commented-out" sections:

* **General:** `output_resolution`, `output_path`, `prefetch_on_build`.
* **Fetch:** `providers` (met, artic), `artists` (list), `era_range` (e.g., 1800-1900), `max_pool_size`.
* **Style:** `background_color`, `font_size`, `placard_position`, `text_color`.

## 6. Technical Implementation Guidance (Rust)

* **API Handling:** Use `reqwest` for async downloads.
* **Image Processing:** Use the `ril` (Rust Imaging Library) crate for high-level image composition and text drawing.
* **State:** Use `rusqlite` for the database.
* **Binary Portability:** Use `include_bytes!` to bundle the default `.ttf` font file directly into the compiled binary.
