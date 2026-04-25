# rxy (/'a:rki/)

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)

A terminal UI for browsing recent arXiv papers, built with Rust and [ratatui](https://github.com/ratatui-org/ratatui).

> **Disclaimer:** rxy and its authors are not affiliated with, endorsed by, or in any way associated with arXiv or Cornell University. arXiv is a registered trademark of Cornell University. This tool uses the publicly available [arXiv API](https://info.arxiv.org/help/api/index.html) in accordance with its terms of use.

```
┌─ Categories ───────────────────────┐┌─ Feed ──────────────────────────────────────────┐
│ [★] math.CT  Category Theory       ││ ▶ ★ Topos Theory and Homotopy Types (2025-02-21)│
│ [★] gr-qc    Gen. Relativity & QC  ││   Black Hole Information Paradox    (2025-02-20)│
│ [ ] math-ph  Mathematical Physics  ││   QFT on Curved Spacetime           (2025-02-20)│
└────────────────────────────────────┘└─────────────────────────────────────────────────┘
                                      ┌─ Abstract ──────────────────────────────────────┐
                                      │ Topos Theory and Homotopy Types                 │
                                      │                                                 │
                                      │ Authors: Lurie, J., Rezk, C., ...               │
                                      │                                                 │
                                      │ Abstract:                                       │
                                      │ We develop a unified framework for higher topos │
                                      │ theory using the language of ∞-categories...    │
                                      │                                                 │
                                      │ [o] Abstract: https://arxiv.org/abs/2502...     │
                                      │ [p] PDF:      https://arxiv.org/pdf/2502...     │
                                      └─────────────────────────────────────────────────┘
```

## Features

- Browse papers from any arXiv category
- Star categories as favourites — the combined feed loads on startup
- Save individual papers across sessions
- Mark papers as read; hide or show read papers
- Open abstracts or PDFs directly in your browser
- **Topic filter** — supply a JSON filter to score, hide, and rank papers by relevance
- All state persists in `~/.config/rxy/`

## Build

Requires Rust (stable). Install via [rustup](https://rustup.rs) if needed.

```bash
git clone <repo>
cd rxy
cargo build --release
```

The binary is at `target/release/rxy`. Copy it somewhere on your `$PATH`:

```bash
cp target/release/rxy ~/.local/bin/
```

## Usage

```bash
rxy                          # plain browser, no filtering
rxy --filter my-filter.json  # load with topic filter active
rxy --version                # print version
rxy --help                   # print usage
```

### Topic filter

A topic filter is a JSON file that scores every fetched paper against category weights, keyword rules, author rules, and anti-keywords. Papers with a score ≤ 0 are hidden; the rest are sorted highest-score-first. A green `[score]` badge appears next to each title and the Feed panel shows `[filtered]` when a filter is active.

**Generate a starter filter:**

```bash
rxy --new-filter my-filter.json
```

This writes a well-commented demo file you can edit to match your research interests. The schema supports:

| Section | Effect |
|---------|--------|
| `categories.primary/secondary/tertiary` | Add weight when a paper belongs to that arXiv category |
| `keyword_rules.<group>` | Add weight when any listed term appears in title or abstract |
| `author_rules.<tier>` | Add weight when a listed name matches any author |
| `exclusions.anti_keywords` | Subtract weight (negative terms) to push irrelevant papers below zero |
| `scoring_thresholds` | Documents what each score band means (informational) |

**Run with the filter:**

```bash
rxy --filter my-filter.json
```

## Keyboard shortcuts

### Global

| Key | Action |
|-----|--------|
| `Tab` | Cycle focus: Categories → Feed → Abstract |
| `r` | Refresh / reload the current feed |
| `?` | Toggle help overlay |
| `q` / `Esc` | Quit |

### Categories panel

| Key | Action |
|-----|--------|
| `↑` / `↓` / `j` / `k` | Navigate |
| `Enter` / `Space` | Load feed for the selected category |
| `f` | Toggle favourite on the highlighted category |
| `+` / `=` | Open picker to add a new favourite |
| `-` | Remove the highlighted category from favourites |
| `a` | Toggle between favourites-only and all categories |

### Feed / Saved panels

| Key | Action |
|-----|--------|
| `↑` / `↓` / `j` / `k` | Navigate |
| `Enter` / `Space` | Focus the Abstract panel |
| `s` | Save / unsave the selected paper |
| `S` | Switch between Feed and Saved tabs |
| `x` | Toggle read / unread |
| `h` | Hide / show read papers (Feed tab only) |
| `o` | Open abstract page in browser |
| `p` | Open PDF in browser |

### Abstract panel

| Key | Action |
|-----|--------|
| `↑` / `↓` / `j` / `k` | Scroll |
| `Enter` / `Space` | Return focus to Feed |
| `o` | Open abstract page in browser |
| `p` | Open PDF in browser |

## Config & data files

| File | Contents |
|------|----------|
| `~/.config/rxy/config.toml` | Favourite categories |
| `~/.config/rxy/read.txt` | Read paper URLs (one per line) |
| `~/.config/rxy/saved.json` | Saved papers (full metadata) |

All files are created automatically on first run.

## License

MIT — see [LICENSE](LICENSE).
