# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Vellum is a desktop note-taking app inspired by Obsidian, built in Rust with egui. Notes are Typst (`.typ`) documents stored in a local vault (`~/vellum`). The app provides a mixed inline editor: plain text paragraphs are editable directly, while Typst blocks (math, tables, lists, etc.) are rendered in-process and flip to source-editing on click. Helix can be launched as an external editor.

## Dev Environment

This project uses Nix flakes for reproducible development:

```sh
nix develop        # enter dev shell with rustc, cargo, rust-analyzer
```

## Build & Run

```sh
cargo build
cargo run
cargo test
cargo test <test_name>   # run a single test
cargo clippy
```

If running on a system where Wayland libs are missing, set:
```sh
WINIT_UNIX_BACKEND=x11 cargo run
```

## Architecture

- **`app`** — main `App` struct, egui event loop, top-level state, coordinates all other modules
- **`vault`** — vault directory scanning, file CRUD for `.typ` files, default at `~/vellum`; subdirs: `note/`, `asset/`
- **`editor/`** — editor subsystem:
  - **`segment`** — paragraph-based parser splitting source into `Plain` and `Typst` segments
  - **`mixed`** — mixed inline editor (`MixedEditor`): Plain segments use `TextEdit`, Typst segments render via `TypstEngine` and flip to source-edit on click; owns the dirty flag and is the single source of truth for buffer state
  - **`typst_engine`** — in-process Typst 0.14 compiler; implements `typst::World`; bundles fonts via `typst-assets`
- **`external_editor`** — `open_in_helix(path)` spawns an external terminal running `hx <file>`
- **`file_watcher`** — `FileWatcher` detects external `.typ` changes and triggers reload
- **`search`** — filename and content search; parses `[[wiki-links]]` for backlinks
- **`ui/`** — egui panels: vault explorer, editor view, backlinks panel

### Data Flow

1. Vault scan loads `.typ` files from `~/vellum/note/` into the sidebar file tree
2. Selecting a note loads its contents into `MixedEditor` via `load(&source)`
3. `MixedEditor` parses the source into `Plain`/`Typst` segments
4. Typst segments are compiled in-process by `TypstEngine` and displayed as images
5. Clicking a rendered Typst block flips it to a source `TextEdit`; focus loss re-parses
6. `Ctrl+S` serializes segments back to source and writes to disk
7. File-watcher detects external writes and reloads (if buffer is clean)
8. Backlinks updated by scanning all notes for `[[note-name]]` references

### Segment Classification

A paragraph (blank-line-separated block) is classified as `Typst` if it:
- Contains `#` or `$` anywhere, OR
- Starts with `=`, `-`, `+`, or `/`

Otherwise it is `Plain`.

### Preamble Propagation

The first N contiguous Typst segments that contain only `#let`/`#import`/`#set`/`#show`/blank/comment lines are the "preamble". The preamble text is prepended to every subsequent Typst segment before compilation so that bindings and imports flow through all blocks.

### Vault Directory Structure

```
~/vellum/
  typst.toml        ← Typst package manifest (enables LSP root resolution)
  note/             ← all .typ note files
  asset/
    theme.typ       ← dark theme template (auto-generated on first run)
    (images, etc.)
```

`typst.toml` enables `tinymist` LSP to resolve `/asset/theme.typ` imports correctly.

### Render Cache

`MixedEditor` caches `TextureHandle` values in `HashMap<String, TextureHandle>` keyed by the *effective source* (preamble + block body). A failed compile is cached in `HashMap<String, String>` to avoid retrying every frame. Both caches are invalidated when the segment text changes (new key).

### UI Layout

```
┌──────────────────────────────────────────────┐
│  [sidebar toggle]              [status]       │  ← topbar
├──────────┬───────────────────────────────────┤
│  Vault   │                                   │
│  Search  │    MixedEditor                    │
│  ──────  │    (Plain: TextEdit inline)        │
│  Notes   │    (Typst: rendered image,         │
│          │     click → source TextEdit)       │
│          ├───────────────────────────────────┤
│          │   Backlinks panel                 │
└──────────┴───────────────────────────────────┘
```

### External Editor (Helix)

`open_in_helix()` in `external_editor.rs` tries terminals in order: `alacritty`, `kitty`, `foot`, `wezterm`, `ghostty`, `gnome-terminal`, `konsole`, `xterm`. Override with `$TERMINAL` env var. The dirty buffer is saved before launching. `FileWatcher` (in `file_watcher.rs`) reloads the buffer when Helix writes the file (only if buffer is clean).

### Config

App config lives at `~/.config/vellum/config.toml`.

## Key Shortcuts

- `Ctrl+E` — open current note in Helix (works from any panel)
- `Ctrl+S` — save current note

## Implementation Notes

- `TypstEngine::render_snippet` wraps each snippet in `#import "/asset/theme.typ": template\n#show: template\n\n{snippet}` before compiling
- `comemo::evict(0)` is called before each compile to flush Typst's memoization cache
- `typst-assets` provides bundled fonts including New Computer Modern Math (required for math rendering)
- System fonts are loaded via `fontdb` in addition to bundled fonts
- Search uses regex; Tantivy is a future upgrade if zstd dependency conflicts are resolved
- Obsidian-style `[[links]]` are parsed for backlink tracking
- `typst::Library::default()` requires `use typst::LibraryExt` to be in scope (typst 0.14+)
