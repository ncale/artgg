# 🏛️ artgg

**artgg** (Art Gallery Generator) is a cross-platform Rust-based CLI that turns your wallpaper into a rotating gallery of open access public domain art. It fetches artwork and generates configurable museum placards for regular education.

## ✨ Highlights

* **Curated Feeds:** Set preferences for artists (e.g., Monet), eras (e.g., Renaissance), or mediums (Oil on Canvas).
* **Educational Placards:** Includes a rendered placard with the painting name and date, the artist's bio, and a blurb.
* **Local-First:** Uses a local SQLite database to track your history so you never see the same piece twice unless you want to.
* **Static:** Generates images on command; No long-running background daemons.
* **Simple Configuration:** No configuration required out-of-the-box; run `init` to generate a config file with a list of configuration options if desired.

## 🚀 Quick Start

TODO: add quick start

## Core Workflow

1. **Init:** Generate a default configuration.
2. **Fetch:** Query museum APIs (The Met, Art Institute of Chicago) to find high-res CC0 artworks matching user "vibes" or artists. Store metadata in SQLite.
3. **Build:** Select a "Draft" from the pool, resize it onto a high-res canvas (matte), and draw a typography-focused placard.
4. **Prune:** Automatically remove old or "seen" raw files to save disk space based on a retention policy.

## Command Structure

* `artgg init`: Creates `~/.config/artgg/config.toml` with all options commented out.
* `artgg fetch`: Downloads images to `~/.cache/artgg/pool/`.
* `artgg build [--prefetch]`:
* If `--prefetch` is present, runs `fetch` first.
* Processes an image and saves the final output to `~/.local/share/artgg/gallery/`.

* `artgg prune`: Deletes files in `pool/` older than `X` days or exceeding `Y` count.

## ⚙️ Configuration

artgg can be configured with `config.toml`.

```toml
# --- Display Settings ---
# resolution = [1920, 1080]
# background_color = "#1a1a1a"
# font_family = "LibreBaskerville"

# --- Content Settings ---
# providers = ["met", "artic"]
# artists = ["Vincent van Gogh", "Claude Monet", "Rembrandt"]
# categories = ["Painting", "Drawing"]

# --- Feed Hygiene ---
# pool_size = 50           # Number of raw images to keep staged
# retention_days = 14      # Auto-delete raw files after 2 weeks

```
