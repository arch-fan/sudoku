# Sudoku

A Sudoku game written in Rust with [GPUI](https://github.com/zed-industries/zed/tree/main/crates/gpui)
(Zed's GPU UI framework). The same UI code runs **natively on macOS** (Metal) and **in the browser**
(WebGPU/WebGL via `gpui_web`).

## Quick start

Everything (nightly Rust, the wasm target, Cranelift, Trunk, watchexec) is installed by
[mise](https://mise.jdx.dev):

```bash
mise install
```

```bash
mise run dev          # native macOS app
```

```bash
mise run watch        # native app, rebuild + relaunch on every save
```

```bash
mise run web          # web build with live reload → http://127.0.0.1:8080
```

```bash
mise run web:lan      # same, reachable from your phone on the local network
```

```bash
mise run test         # core engine tests
```

```bash
mise run release      # optimized native binary → target/release/sudoku
```

```bash
mise run web:release  # optimized static site → apps/web/dist
```

## Install on iPhone (PWA)

The web build is an installable, offline-capable web app (manifest, service worker,
home-screen icons, safe-area aware, iOS status bar styling).

1. `mise run web:release` → upload `apps/web/dist/` to any static HTTPS host
   (Cloudflare Pages, Netlify, GitHub Pages, your own nginx…). HTTPS is required for the
   service worker (offline) and WebGPU; over plain `http://` on the LAN it still runs via WebGL,
   but won't work offline.
2. Open the URL in Safari → Share → **Add to Home Screen**.

All URLs are relative, so the same `dist/` works on any domain or sub-path, e.g. both
`https://sudoku.archfan.com` and `https://sudoku.otakulihao.com`. Each domain is a separate
"app" for the browser, with its **own** saved progress and records (storage is per origin),
so the recommended setup is one canonical domain (`sudoku.archfan.com`) and a 301 redirect
from the other.

## Deploy with Docker

The root `Dockerfile` builds the web bundle (pinned nightly + Trunk, assets pre-compressed with
brotli/gzip) and serves it with [static-web-server](https://static-web-server.net) **v3**
(`3.0.0-beta.3`, scratch-based image, config in `sws.toml`):

```bash
docker build -t sudoku-web .
```

```bash
docker run --rm -p 8080:8080 sudoku-web
```

Put it behind your HTTPS reverse proxy / platform (Caddy, Traefik, Coolify, Fly…) on
`sudoku.archfan.com`. Health checks: `GET /livez` (liveness) and `GET /readyz` (readiness). For an x86 server from an Apple Silicon Mac,
add `--platform linux/amd64` (only the tiny server layer changes; the wasm is built natively).

## Saved data

No server. Settings, statistics/records and the game in progress are saved as JSON:

* web: `localStorage` (`sudoku.save.v1`)
* macOS: `~/Library/Application Support/Sudoku/save.json`

The game is saved after every move (and every few seconds for the timer) and is restored on
launch (**Continue** on the home screen). The app pauses itself when it goes to the background.

## Project layout

```
crates/
  sudoku_core/   pure engine, no GPUI (optional `serde` feature for save files)
                 grid · solver · grader · generator · hint (explainable hints) · game
  sudoku_ui/     every screen and animation, written once with GPUI
                 app (state, navigation, input) · home (play/records/settings tabs)
                 board · chrome (header/stats/actions/numpad) · hint_panel
                 overlays (win/lose/streak burst/confetti) · store (save data) · platform
                 i18n (es/en) · theme · anim · assets (embedded icons + font)
apps/
  desktop/       macOS entry point: `sudoku_ui::run(gpui_platform::application())`
  web/           wasm entry point, index.html, PWA manifest, service worker, icons
                 (regenerate icons with `python3 tools/make_icons.py`)
```

To add a target (Linux, Windows, iOS…), add a thin `apps/<target>` crate that picks the
platform `Application` and calls `sudoku_ui::run`. `sudoku_core` can also be reused without
GPUI at all (CLI, server, tests, another UI toolkit).

## Gameplay

* 5 difficulties: Fácil, Medio, Difícil, Experto, Extremo (Easy → Extreme). Puzzles always have a unique solution
  and are rated by the human techniques needed to solve them (singles → locked candidates →
  pairs/triples → advanced), not just by the number of givens.
* Undo, erase, notes, 3 mistakes allowed (with a second chance; can be turned off), timer, pause.
* **Guided hints**: instead of just filling a cell, a hint walks you through it in 3 steps:
  where to look (row/column/box spotlight), why (the digits that rule out the alternatives are
  highlighted), and the answer with a button to place it. Mistakes are pointed out first.
* **Streak celebrations**: chaining correct moves (3, 5, 8, 12…) triggers a quick burst in the
  middle of the screen.
* Home screen with Play (continue / new game + difficulty sheet), Records (per difficulty:
  played, won, win rate, best/average time, best score, top games with dates) and Settings
  (language Español/English, theme, mistake limit, timer, highlight matches, reset stats).
* Placing the correct digit removes it from the notes in its row, column and box.
* Score with a combo multiplier for streaks of correct moves and bonuses for completing a row,
  column or box.
* 3 themes.

### Keyboard

| Key | Action |
| --- | --- |
| `1`–`9` | enter digit / toggle note |
| arrows or `WASD` | move the selection |
| `Backspace`, `Delete`, `0`, `X` | erase |
| `N` or `Space` | toggle notes mode |
| `Cmd/Ctrl+Z` | undo |
| `H` | hint |
| `P` or `Esc` | pause (`Esc` also closes the difficulty sheet) |
| `Enter` / `←` / `Esc` | guided hint: next / back / close |
| `F8` | debug builds only: play one correct digit (try streaks) |
| `F9` | debug builds only: fill every open cell but one |

## Fast iteration setup

* **Nightly toolchain** pinned in `mise.toml`.
* **Cranelift** compiles the workspace crates in the `dev` profile, so rebuilding after a UI change
  skips LLVM entirely. Dependencies, including GPUI, are compiled once with LLVM at `opt-level = 2`,
  so animations stay smooth in debug builds. `sudoku_core` always uses LLVM `-O3` because the
  generator is hot.
* `-Zshare-generics` and `-Zthreads=8` (parallel frontend) in `.cargo/config.toml`.
* `debug = "line-tables-only"` with unpacked split debuginfo.
* Metal shaders are compiled when the app starts (`runtime_shaders`), so you don't need the Metal
  toolchain at build time.
* The web build is single-threaded GPUI: no `SharedArrayBuffer`, no COOP/COEP headers, and no
  `build-std`. The `web-dev` profile uses LLVM because Cranelift can't target wasm.

## Notes

* macOS text needs gpui's `font-kit` feature (enabled in `apps/desktop`); without it gpui
  silently uses a no-op text system and draws no text. Check with
  `cargo run -p sudoku_desktop -- --check-text` (prints `TEXT OK`). On nightly, font-kit's
  `pathfinder_simd` is forced to its scalar path (`pf-no-simd`) to avoid a rustc crash.

* GPUI is pinned to a zed git revision in the root `Cargo.toml`, together with the
  `[patch.crates-io]` entries zed itself uses. To upgrade, bump `rev` and run `cargo update`.
* Icons are Lucide-style SVGs and the font is IBM Plex Sans (OFL). Both are embedded so the web
  build doesn't need a filesystem.
