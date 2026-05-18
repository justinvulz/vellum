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
```

If running on a system where Wayland libs are missing, set:
```sh
WINIT_UNIX_BACKEND=x11 cargo run
```

## Architecture

- **`app`** — `App` struct, eframe event loop, `AppAction` dispatch, keyboard shortcuts (`shortcut_actions`), file-watcher polling
- **`vault`** — vault directory scanning, file CRUD for `.typ` files, default at `~/vellum`; `open_or_init` calls `ensure_directories` / `ensure_manifest` / `ensure_theme`; holds `notes: Vec<PathBuf>` and `folders: Vec<PathBuf>` populated by `rescan`; CRUD: `create_note`, `delete_note`, `create_folder`, `delete_folder`, `move_note(from, to_folder)` (`to_folder: None` moves back to root `note/`)
- **`editor/`** — editor subsystem:
  - **`segment`** — tree-based splitter; walks `typst::syntax::parse` output and emits one segment per heading / block-math / top-level `#`-code (alone on its line) / text paragraph
  - **`preamble`** — preamble detection (`is_preamble_only`, `collect`) and theme-template source wrapping (`wrap_for_render`)
  - **`highlight`** — syntax highlighter for the source `TextEdit`; walks Typst's parse tree and builds a coloured `LayoutJob` keyed on `style::SyntaxColors`
  - **`mixed`** — mixed inline editor (`MixedEditor`): every segment renders via `TypstEngine` and flips to a monospace source `TextEdit` on click; hit-tests clicks against per-segment link rectangles and emits an `Option<String>` nav target; suppresses egui's full-row caret and paints its own blinking font-sized one; owns the dirty flag and `EditorConfig`
  - **`typst_engine`** — in-process Typst 0.14 compiler; implements `typst::World`; bundles fonts via `typst-assets`; `render()` returns a `RenderedPage { texture, links }`, with `links` collected from `FrameItem::Link` entries that use the `vellum://` URL scheme
- **`external_editor`** — `open_in_helix(path)` spawns an external terminal running `hx <file>`
- **`file_watcher`** — `FileWatcher` reports external `.typ` changes; `App::poll_watcher` consumes them
- **`search`** — content search; parses `[[wiki-links]]` for backlinks; `find_note_by_stem(vault, name)` resolves `#line-note` click targets — if `name` contains `/` it is treated as a vault-relative path (`"ideas/foo"` → `note/ideas/foo.typ`), otherwise falls back to case-insensitive stem match (first alphabetically wins)
- **`style`** — fonts, text styles, sizing constants (`UI_PT`, `EDITOR_PT`, `CONTENT_WIDTH_PT`), the edit-mode accent outline (`paint_edit_outline`), and the editor config types (`EditorConfig`, `SyntaxColors`)
- **`ui/`** — egui panels: `topbar`, `vault_explorer`, `editor_view`, `backlinks_panel`
- **Debug tracing** — `log` + `env_logger` initialised in `main`; default filter `info,vellum=debug`, overridable via `RUST_LOG`

### Data Flow

1. Vault scan loads `.typ` files and subdirectories from `~/vellum/note/` into the sidebar file tree
2. Selecting a note loads its contents into `MixedEditor` via `load(&source)`
3. `MixedEditor` runs `segment::parse_segments` over the source, producing a `Vec<String>` of block segments
4. Each frame, `ensure_rendered` compiles uncached segments within a 16 ms budget; compiled segments enter the render cache as `RenderedPage { texture, links }`, the rest show `⟳ rendering…` and are compiled on the next repaint
5. Clicking a rendered segment flips it to a source `TextEdit` (with a blue edit outline) — unless the click landed in a link rectangle, in which case `MixedEditor::show` returns `Some(target)` and `editor_view` emits `OpenNoteByName(target)` instead; focus loss re-splits the buffer
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

### Inter-note Links

`#line-note(name, body: none)` in `assets/default_theme.typ` compiles to a Typst `link("vellum://" + name, …)`. `editor::preamble::wrap_for_render` co-imports `line-note` with `template`, so user code can call it without an explicit `#import`. The function is a normal `link`, so Typst's layout records both the destination and the rectangle as a `FrameItem::Link(Destination::Url, Size)` on the page frame.

`TypstEngine::render` walks `page.frame` (folding only the translation component of group transforms — `line-note` is plain inline text, no rotation/scale) and returns each `vellum://` link as a `LinkRect { rect, target }` in typst points. Because we render at `PIXEL_PER_PT = 2.0` but draw back at `pixels / PIXEL_PER_PT`, 1 typst pt ↔ 1 egui pt and the rectangles overlay the image without scaling.

`mixed::show_rendered` hit-tests clicks in the response's local coordinates rather than allocating per-link `Sense::click()` widgets — single interaction target, no z-order surprises when a link rectangle straddles a row boundary. A match returns `SegmentClick::Link(target)`; `MixedEditor::show` propagates that up as `Option<String>`. `ui::editor_view::show` wraps it in `AppAction::OpenNoteByName(name)`; `App::open_note_by_name` resolves via `search::find_note_by_stem` — if the name contains `/` it matches the vault-relative path (e.g. `"ideas/foo"` → `note/ideas/foo.typ`), otherwise it does a case-insensitive stem match. Unresolved targets set `status = "note not found: X"`. The cursor becomes `CursorIcon::PointingHand` while hovering a link rectangle.

### Segment States

Each segment is in one of four states each frame:

- **Editing** — monospace `TextEdit` with a blue edit outline (`style::paint_edit_outline`). Text is laid out via a custom layouter that runs `editor::highlight::highlight` over the source on every keystroke, producing a coloured `LayoutJob`. egui's built-in caret is suppressed and `mixed::paint_caret` draws a `font_size`-tall blinking caret centred in the row.
- **Compile error** — red banner + error text + source label; click to edit
- **Rendered** — compiled Typst image at 1 egui pt ↔ 1 typst pt; click to edit
- **Pending** — `⟳ rendering…` placeholder while the engine compiles

The per-frame scratch state (`FrameState` in `mixed.rs`) collects events from each segment's helper and is applied after the egui closures unwind.

### Editor Config

`style::EditorConfig` (exposed as `MixedEditor::config`, `pub` field) holds runtime knobs for the source view:

- `font_size`, `font_family` — TextEdit font.
- `line_space: Option<f32>` — extra baseline-to-baseline gap on top of `font_size`. `None` keeps egui's natural row height; `Some(x)` widens lines (and would widen egui's caret — except we paint our own, so the caret stays at `font_size`).
- `colors: SyntaxColors` — per-token-kind palette (`dollar`, `hash`, `heading_marker`, `comment`, `string`, `number`, `keyword`, `ident`, `punct`, `emphasis`, `list_marker`, `default`).

Mutate after `MixedEditor::new()` to retheme at runtime.

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

`MixedEditor` caches `RenderedPage` values (texture + link rectangles) in `HashMap<String, RenderedPage>` keyed by the *effective source* (preamble + block body). A failed compile is cached in `HashMap<String, String>` to avoid retrying every frame. Both caches are invalidated when the segment text changes (new key); link rectangles are extracted at compile time, so they're cached together with the texture.

### Progressive Rendering

`MixedEditor::ensure_rendered` compiles at most `FRAME_COMPILE_BUDGET_MS` (16 ms) worth of segments per frame. Segments beyond the budget remain in **Pending** state (`⟳ rendering…`) and are compiled on the next repaint via `ctx.request_repaint()`. This keeps the UI responsive when opening long notes — segments load top-to-bottom while the app stays interactive.

### UI Layout

```
┌──────────────────────────────────────────────┐
│  [sidebar toggle]              [status]       │  ← topbar (ui::topbar)
├──────────┬───────────────────────────────────┤
│  Vellum  │                                   │
│  [note…] │    MixedEditor                    │
│  [fold…] │    rendered Typst image            │
│  search… │      └─ click → source TextEdit    │
│  ──────  │            (blue edit outline)     │
│  ▶ ideas │                                   │
│    note  │                                   │
│  note B  │                                   │
│          ├───────────────────────────────────┤
│          │   Backlinks panel                 │
└──────────┴───────────────────────────────────┘
```

The sidebar is a VS Code-style file tree (`ui::vault_explorer`):
- Folders show with a `▶`/`▼` chevron; clicking the row toggles expansion. Expand state is stored in egui's per-frame-persistent memory keyed by folder path.
- Notes are leaf nodes, indented one chevron-width deeper than their parent folder.
- Folders are rendered first (alphabetical), then notes.
- Notes have `Sense::click_and_drag()` via a second `ui.interact` call over the label rect. This sidesteps egui's hit-tester rule that drops a click when a drag-only widget sits on top of a click widget at the same rect. `Response::dnd_set_drag_payload` sets the payload on `drag_started()`.
- Each folder row is wrapped in `ui.dnd_drop_zone::<PathBuf>` — dropping a note emits `AppAction::MoveNote { from, to_folder }`.
- A `📂 (root)` drop zone appears at the top of the tree only while a note is being dragged, for moving notes back to the root `note/` directory.
- Search query filters notes by stem; ancestor folders of matching notes are force-opened.

### External Editor (Helix)

`open_in_helix()` in `external_editor.rs` tries terminals in order: `alacritty`, `kitty`, `foot`, `wezterm`, `ghostty`, `gnome-terminal`, `konsole`, `xterm`. Override with `$TERMINAL` env var. The dirty buffer is saved before launching. `FileWatcher` (in `file_watcher.rs`) reloads the buffer when Helix writes the file (only if buffer is clean).

### Config

No on-disk config file yet. Two layers of in-code tunables:

- **Global style** — constants in `src/style.rs`: `UI_PT`, `EDITOR_PT`, `CONTENT_WIDTH_PT`, `SANS_FAMILIES`, `EDIT_OUTLINE_COLOR`.
- **Per-editor** — `style::EditorConfig` exposed as `MixedEditor::config` (font, `line_space`, `SyntaxColors`); mutate at runtime to retheme.

External-editor selection is overridden via the `$TERMINAL` env var (handled in `external_editor.rs`). Logging filter via `RUST_LOG` (default `info,vellum=debug`).

## Key Shortcuts

- `Ctrl+E` — open current note in Helix (works from any panel)
- `Ctrl+S` — save current note

`App::shortcut_actions` produces these from `ctx.input(...)` at the start of every frame; UI panels emit `AppAction` values handled by `App::perform` at the end of the frame.

## Implementation Notes

- `editor::preamble::wrap_for_render` wraps each snippet body with `#import "/asset/theme.typ": template, line-note\n#show: template.with(width: …pt, size: …pt)\n\n{body}\n` before handing it to `TypstEngine::render`. Width and size come from `style::CONTENT_WIDTH_PT` / `EDITOR_PT`, so the rendered image stays in lock-step with the surrounding egui layout. `line-note` is co-imported so `#line-note("X")` works without an explicit `#import` in user code.
- `comemo::evict(0)` is called before each compile to flush Typst's memoization cache.
- `typst-assets` provides bundled fonts including New Computer Modern Math (required for math rendering). System fonts are loaded via `fontdb` in addition.
- The render cache key is the *fully wrapped* source (template + preamble + body), so changing any of those parts invalidates the entry. Failed compiles are also cached (in `failed: HashMap<String, String>`) to avoid retrying every frame.
- **Caret quirk**: in egui 0.27 the built-in caret rect grows to match the galley row span (`cursor_rect` in `text_cursor_state.rs` uses `max.y.at_least(min.y + row_height)`), so widening `line_space` would stretch the caret. `mixed::show_editing` works around this by setting `visuals_mut().text_cursor.color = TRANSPARENT` for the duration of `TextEdit::show`, then painting a `font_size`-tall blinking caret manually in `paint_caret`. Highlighter sections use `valign: Align::Center` so the caret tracks the glyph centre.
- **Theme override**: `assets/default_theme.typ` is `include_str!`'d at compile time and rewritten to `~/vellum/asset/theme.typ` by `vault::ensure_theme` on every launch. The signature `template(doc, width, size)` is owned by the app — changing it on disk will be overwritten next start.
- Search uses regex; Tantivy is a future upgrade if zstd dependency conflicts are resolved.
- Obsidian-style `[[links]]` are parsed for backlink tracking.
- **Inter-note link extraction**: `typst_engine::collect_links` walks the compiled `page.frame` and only folds the translation component of `GroupItem::transform` (`tx`, `ty`) into the accumulated origin. Rotation/scale are ignored — fine for `#line-note` (plain inline text), but rectangles will drift if a future caller puts a `vellum://` link inside `rotate(…)` or non-uniform `scale(…)`.
- `typst::Library::default()` requires `use typst::LibraryExt` to be in scope (typst 0.14+).
