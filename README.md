# 🏛️ artgg

**artgg** (Art Gallery Generator) is a cross-platform Rust-based TUI that generates desktop wallpapers from classical artwork with museum-style placards.

## ✨ Highlights

- **Curated Feeds:** Set preferences for artists (e.g., Monet), eras (e.g., Renaissance), or mediums (Oil on Canvas).
- **Educational Placards:** Includes a rendered placard with the painting name and date, the artist's bio, and a blurb.
- **Local-First:** Uses a local SQLite database to track your history so you never see the same piece twice unless you want to.
- **Static:** Generates images on command; No long-running background daemons.
- **Simple Configuration:** No out-of-the-box configuration required.

## 🚀 Quick Start

Install via cargo.

```shell
cargo install artgg
```

Then start the TUI.

```shell
artgg
```
