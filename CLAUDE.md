# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Vellum is a desktop note-taking app inspired by Obsidian, built in Rust with egui. Notes are Typst (`.typ`) documents stored in a local vault (`~/vellum`). The editor splits each note into block-level segments using Typst's syntax tree, compiles every segment in-process, and renders it as an image; clicking a segment flips it to a source `TextEdit`. Helix can be launched as an external editor.

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

- **`app`** — `App` struct, eframe event loop, `AppAction` dispatch, keyboard shortcuts (`shortcut_actions`), file-watcher polling
- **`vault`** — vault directory scanning, file CRUD for `.typ` files, default at `~/vellum`; `open_or_init` calls `ensure_directories` / `ensure_manifest` / `ensure_theme`
- **`editor/`** — editor subsystem:
  - **`segment`** — tree-based splitter; walks `typst::syntax::parse` output and emits one segment per heading / block-math / top-level `#`-code (alone on its line) / text paragraph
  - **`preamble`** — preamble detection (`is_preamble_only`, `collect`) and theme-template source wrapping (`wrap_for_render`)
  - **`mixed`** — mixed inline editor (`MixedEditor`): every segment renders via `TypstEngine` and flips to a monospace source `TextEdit` on click; owns the dirty flag and is the single source of truth for buffer state
  - **`typst_engine`** — in-process Typst 0.14 compiler; implements `typst::World`; bundles fonts via `typst-assets`
- **`external_editor`** — `open_in_helix(path)` spawns an external terminal running `hx <file>`
- **`file_watcher`** — `FileWatcher` reports external `.typ` changes; `App::poll_watcher` consumes them
- **`search`** — filename and content search; parses `[[wiki-links]]` for backlinks
- **`style`** — fonts, text styles, sizing constants (`UI_PT`, `EDITOR_PT`, `CONTENT_WIDTH_PT`), and the edit-mode accent outline (`paint_edit_outline`)
- **`ui/`** — egui panels: `topbar`, `vault_explorer`, `editor_view`, `backlinks_panel`

### Data Flow

1. Vault scan loads `.typ` files from `~/vellum/note/` into the sidebar file tree
2. Selecting a note loads its contents into `MixedEditor` via `load(&source)`
3. `MixedEditor` runs `segment::parse_segments` over the source, producing a `Vec<String>` of block segments
4. Each segment is wrapped (`preamble::wrap_for_render`) and compiled by `TypstEngine`, then displayed as an image
5. Clicking a rendered segment flips it to a source `TextEdit` (with a blue edit outline); focus loss re-splits the buffer
6. `Ctrl+S` serializes segments back to source (joined with `\n\n`) and writes to disk
7. File-watcher reports external writes; `App::poll_watcher` reloads the buffer if it is clean
8. Backlinks updated by scanning all notes for `[[note-name]]` references

### Segment Splitting

`segment::parse_segments` walks the top-level children of Typst's `Markup` syntax tree:

- **`Heading`** (`= …`) — always its own segment, even without a surrounding blank line.
- **`Equation`** where `ast::Equation::block() == true` (i.e. `$ … $` with whitespace immediately inside the dollars) — own segment.
- **`Hash`** + following code expression (`FuncCall`, `LetBinding`, `SetRule`, `ShowRule`, `ModuleImport`, …) — own segment **only when the pair is alone on its source line**. That keeps inline `Hello #strong[bold] world` and `Hello $x$ world` as single text segments.
- **`Parbreak`** (blank line at the top level) — ends the current text segment.
- Everything else (`Text`, `Space`, `Linebreak`, list/enum/term items, inline math, inline `Strong`/`Emph`, …) accumulates into text segments.

Because the parser is tree-aware, a blank line *inside* a function call's content block (`#table()[\n  a\n\n  b\n]`) is **not** a top-level `Parbreak` and does not split the segment.

### Preamble Propagation

`editor::preamble::collect` walks the leading run of "preamble-only" segments — segments whose lines start only with `#let` / `#import` / `#set` / `#show`, `//` comments, or are blank. The joined preamble text is prepended to every later segment before compilation so bindings and imports flow through the whole note.

### Segment States

Each segment is in one of four states each frame:

- **Editing** — monospace `TextEdit` with a blue edit outline (`style::paint_edit_outline`)
- **Compile error** — red banner + error text + source label; click to edit
- **Rendered** — compiled Typst image at 1 egui pt ↔ 1 typst pt; click to edit
- **Pending** — `⟳ rendering…` placeholder while the engine compiles

The per-frame scratch state (`FrameState` in `mixed.rs`) collects events from each segment's helper and is applied after the egui closures unwind.

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
│  [sidebar toggle]              [status]       │  ← topbar (ui::topbar)
├──────────┬───────────────────────────────────┤
│  Vault   │                                   │
│  Search  │    MixedEditor                    │
│  ──────  │    rendered Typst image            │
│  Notes   │      └─ click → source TextEdit    │
│          │            (blue edit outline)     │
│          ├───────────────────────────────────┤
│          │   Backlinks panel                 │
└──────────┴───────────────────────────────────┘
```

### External Editor (Helix)

`open_in_helix()` in `external_editor.rs` tries terminals in order: `alacritty`, `kitty`, `foot`, `wezterm`, `ghostty`, `gnome-terminal`, `konsole`, `xterm`. Override with `$TERMINAL` env var. The dirty buffer is saved before launching. `FileWatcher` (in `file_watcher.rs`) reloads the buffer when Helix writes the file (only if buffer is clean).

### Config

No on-disk config file yet. Tunables live as constants in `src/style.rs`: `UI_PT`, `EDITOR_PT`, `CONTENT_WIDTH_PT`, `SANS_FAMILIES`, `EDIT_OUTLINE_COLOR`. External-editor selection is overridden via the `$TERMINAL` env var (handled in `external_editor.rs`).

## Key Shortcuts

- `Ctrl+E` — open current note in Helix (works from any panel)
- `Ctrl+S` — save current note

`App::shortcut_actions` produces these from `ctx.input(...)` at the start of every frame; UI panels emit `AppAction` values handled by `App::perform` at the end of the frame.

## Implementation Notes

- `editor::preamble::wrap_for_render` wraps each snippet body with `#import "/asset/theme.typ": template\n#show: template.with(width: …pt, size: …pt)\n\n{body}\n` before handing it to `TypstEngine::render`. Width and size come from `style::CONTENT_WIDTH_PT` / `EDITOR_PT`, so the rendered image stays in lock-step with the surrounding egui layout.
- `comemo::evict(0)` is called before each compile to flush Typst's memoization cache.
- `typst-assets` provides bundled fonts including New Computer Modern Math (required for math rendering). System fonts are loaded via `fontdb` in addition.
- The render cache key is the *fully wrapped* source (template + preamble + body), so changing any of those parts invalidates the entry. Failed compiles are also cached (in `failed: HashMap<String, String>`) to avoid retrying every frame.
- Search uses regex; Tantivy is a future upgrade if zstd dependency conflicts are resolved.
- Obsidian-style `[[links]]` are parsed for backlink tracking.
- `typst::Library::default()` requires `use typst::LibraryExt` to be in scope (typst 0.14+).
