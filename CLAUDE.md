# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Vellum is a desktop note-taking app inspired by Obsidian, built in Rust with egui. Notes are Typst (`.typ`) documents stored in a local vault (`~/vellum`) and optionally synced via Git. Helix is used as the external editor, launched as a subprocess.

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
- **`vault`** — vault directory scanning, file CRUD for `.typ` files, default at `~/vellum`
- **`editor_backend`** — `open_in_helix(path)` spawns an external terminal running `hx <file>`; `FileWatcher` detects external changes and triggers reload
- **`helix_editor`** — plain egui `TextEdit` buffer wrapper (`HelixEditor`); no modal keymap
- **`typst_compiler`** — Typst CLI compile → PDF → pdfium rasterization pipeline; pages stored as egui `TextureHandle`
- **`search`** — filename and content search; parses `[[wiki-links]]` for backlinks
- **`git`** — optional Git sync (auto-commit, push/pull)
- **`ui/`** — egui panels: vault explorer, editor view, preview pane, backlinks panel

### Data Flow

1. Vault scan loads `.typ` files into file tree
2. Selecting a note loads its contents into the editor buffer
3. Save (`Ctrl+S`) or external Helix edit (detected via file-watcher) triggers Typst recompile
4. Typst CLI produces a PDF; pdfium rasterizes it at `target_width=2400px` into egui textures
5. Backlinks updated by scanning all notes for `[[note-name]]` references

### UI Layout

- Left sidebar: vault explorer + search
- Center: plain text editor (`TextEdit`); toolbar has Save, Reload, Open in Helix buttons
- Right: Typst PDF preview (split-pane mode); preview header has Open in Helix + Open PDF buttons
- Backlinks panel below editor in split-pane mode; tab-toggle mode shows Editor/Preview tabs

### External Editor (Helix)

`open_in_helix()` in `editor_backend.rs` tries terminals in order: `alacritty`, `kitty`, `foot`, `wezterm`, `ghostty`, `gnome-terminal`, `konsole`, `xterm`. Override with `$TERMINAL` env var. The dirty buffer is saved before launching. File-watcher reloads the buffer when Helix writes the file (only if buffer is clean).

### Config

App config lives at `~/.config/vellum/config.toml`.

## Key Shortcuts

- `Ctrl+E` — open current note in Helix (works from any panel)
- `Ctrl+S` — save current note

## Implementation Notes

- PDF preview rasterizes at `target_width=2400` (set in `PreviewState::new()`); adjust for performance vs. quality trade-off
- pdfium requires `PDFIUM_DYNAMIC_LIB_PATH` env var pointing to the directory containing `libpdfium.so`; the Nix dev shell sets this automatically
- Search uses regex; Tantivy is a future upgrade if zstd dependency conflicts are resolved
- Obsidian-style `[[links]]` are parsed for backlink tracking
