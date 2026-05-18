# Vellum — Specification

Vellum is a desktop note-taking app inspired by Obsidian. Notes are Typst (`.typ`) documents stored in a local vault directory. The editor splits each note into block-level segments using Typst's syntax tree; every segment compiles in-process and renders as an image, flipping to a source `TextEdit` on click.

---

## Goals

- Store notes as plain `.typ` files (human-readable, git-friendly)
- Block-level inline editing: every segment renders through Typst and flips to source-edit on click
- Obsidian-style `[[wiki-links]]` and backlink tracking
- Typst-native inter-note links (`#line-note("name")`) that compile to real Typst `link`s and are click-navigable inside the app
- Minimal UI — egui, no Electron, no browser
- Helix as optional external editor; the app is self-sufficient for basic editing

---

## Vault

- Default location: `~/vellum/`
- Subdirectory structure:
  - `~/vellum/note/` — all `.typ` note files (may be nested in subdirectories)
  - `~/vellum/asset/` — images, theme template, and other shared assets
  - `~/vellum/typst.toml` — Typst package manifest for LSP root resolution
- `asset/theme.typ` is auto-generated on first run (dark theme template)
- Recursively scans `note/` for `.typ` files and subdirectories on start and after any file change; populates `notes: Vec<PathBuf>` and `folders: Vec<PathBuf>` (both sorted alphabetically)
- CRUD:
  - `create_note(name)` — accepts `"ideas/foo"` to create `note/ideas/foo.typ`; creates missing parent dirs
  - `delete_note(path)`
  - `create_folder(name)` — accepts nested paths
  - `delete_folder(path)` — empty-only (`fs::remove_dir`); recursive removal not exposed for safety
  - `move_note(from, to_folder)` — `fs::rename`; `to_folder: None` moves to root `note/`
- `display_name` strips the vault root prefix for display
- `default_vault_dir()` falls back to `./vellum` if home dir is unavailable

## Mixed Inline Editor

The core editing experience is a segment-based editor (`MixedEditor`). Every segment compiles through the same Typst pipeline; there is no "plain text" fast path.

### Segment Splitting

`editor::segment::parse_segments` walks the top-level children of Typst's `Markup` syntax tree and emits one segment per block-level construct:

- **Heading** (`= Title`) — always its own segment, even without a surrounding blank line.
- **Block math** (`$ … $` where the body has whitespace immediately inside the dollar signs, per Typst's block-math rule) — own segment.
- **Top-level `#` code** (`Hash` + following expression — `FuncCall`, `LetBinding`, `SetRule`, `ShowRule`, `ModuleImport`, …) — own segment **when the pair is alone on its source line**. This keeps inline uses like `Hello #strong[bold] world` and `Hello $x$ world` inside one text segment.
- **Other markup** (text, inline math, list items, inline calls, etc.) accumulates into text segments, separated only by blank lines (top-level `Parbreak`).

Blank lines that occur inside a multi-line function call or content block are *not* segment boundaries — the parser knows they are inner-paragraph breaks, not top-level ones.

### Editing Behavior

Each segment is in one of four states:

- **Rendered** — compiled Typst image at 1 egui pt ↔ 1 typst pt. Click to flip into edit mode.
- **Editing** — monospace `TextEdit` containing the segment's source, syntax-highlighted via `editor::highlight` (see below). A blue accent outline marks the active segment. Focus loss re-splits the buffer and re-renders.
- **Compile error** — red banner + the typst error message + the raw source. Click the source to edit.
- **Pending** — `⟳ rendering…` placeholder while the engine compiles.

### Syntax Highlighting

`editor::highlight::highlight` walks Typst's parse tree on every keystroke and produces an `egui::text::LayoutJob` with per-leaf colours. Coloured kinds include math `$` delimiters, `#` markup-to-code transitions, heading markers, comments, strings, numbers, keywords, identifiers, list/enum/term markers, and brackets/punctuation. The palette is configurable through `style::SyntaxColors` (purple `$`, teal `#`, yellow `=…`, etc., inspired by VS Code Dark+).

### Custom Caret

egui 0.27 ties caret height to the galley's row span, so any increase in row height stretches the caret. To keep a font-sized blinking caret while still allowing wide line spacing, `MixedEditor::show_editing` suppresses egui's built-in caret (`visuals_mut().text_cursor.color = TRANSPARENT`) and paints its own:

- Anchored at the row centre, `font_size` points tall.
- Blinks on a `CARET_BLINK_PERIOD = 0.53s` cycle, with `request_repaint_after` to keep ticking when idle.
- Glyph spans use `valign: Align::Center` so the caret tracks the text.

### Editor Config

`style::EditorConfig` (exposed as `MixedEditor::config`) collects the per-editor knobs:

- `font_size`, `font_family` — TextEdit font.
- `line_space: Option<f32>` — extra gap between baselines on top of `font_size`. `None` keeps egui's natural row height; `Some(x)` widens lines.
- `colors: SyntaxColors` — per-token-kind palette.

Mutate after `MixedEditor::new()` to retheme or resize at runtime; defaults match Typst's body line distance and a dark palette.

### Preamble Propagation

The leading run of "preamble-only" segments — lines containing only `#let` / `#import` / `#set` / `#show` / `//` comments / blanks — forms the **preamble**. The joined preamble text is prepended to every later segment before compilation so bindings and imports are in scope across all blocks. Detection and source-wrapping live in `editor::preamble`.

### Render Cache

Each segment is keyed on its fully-wrapped source (theme template + preamble + body). `MixedEditor` keeps two content-addressed caches: `renders: HashMap<String, RenderedPage>` for successful compiles (texture + link rectangles), and `failed: HashMap<String, String>` for compile errors (so we don't recompile a broken segment every frame). Both survive note reloads because the key is content-based.

### Progressive Rendering

`ensure_rendered` iterates segments in order and compiles those not yet in cache, but stops after `FRAME_COMPILE_BUDGET_MS` (16 ms) of wall-clock time. Segments not reached in a given frame remain **Pending** (`⟳ rendering…`); `ctx.request_repaint()` is called so the next frame continues where this one stopped. This keeps the UI responsive when opening long notes — segments appear progressively from top to bottom rather than the app freezing until all are compiled.

### Inter-note Links

`assets/default_theme.typ` defines `#let line-note(name, body: none) = link("vellum://" + name, …)` and `editor::preamble::wrap_for_render` co-imports it with `template`, so user code can write `#line-note("X")` without an explicit `#import`. The function compiles to a normal Typst `link`, so Typst records both the rectangle and the destination on the page frame.

After each successful compile, `TypstEngine::render` walks `page.frame` recursively (folding group transforms' translation component into the accumulated origin — rotation and scale are ignored, since `line-note` is plain inline text) and returns every `vellum://` link as a `LinkRect { rect: egui::Rect, target: String }` in typst points. 1 typst pt ↔ 1 egui pt by construction, so the rectangles need no scaling when overlaid on the rendered image.

`MixedEditor::show_rendered` hit-tests clicks against those rectangles in the response's local coordinates rather than allocating per-link `Sense::click()` widgets — that keeps the image as a single interaction target and sidesteps egui z-order quirks when a link straddles a row boundary. Matches return `SegmentClick::Link(target)`, which `MixedEditor::show` propagates up as the function's `Option<String>` return value. `ui::editor_view::show` wraps that into `AppAction::OpenNoteByName(name)`; `App::open_note_by_name` resolves it via `search::find_note_by_stem`:

- If `name` contains `/`, treated as a vault-relative path: `"ideas/foo"` matches `note/ideas/foo.typ` exactly.
- Otherwise, case-insensitive stem match; first in sorted order wins. Use the path-qualified form when two notes share the same stem in different folders.

Unresolved targets set the status line to `note not found: X`. A pointing-hand cursor (`CursorIcon::PointingHand`) is set whenever the pointer hovers over a link rectangle.

## Typst Engine

- In-process compilation using the `typst` crate (0.14); `TypstEngine` implements `typst::World`.
- Each snippet body is wrapped by `editor::preamble::wrap_for_render` with the theme template, threading the editor's column width and body size through `template.with(width: …pt, size: …pt)`.
- Fonts: bundled via `typst-assets` (includes New Computer Modern Math) + system fonts discovered via `fontdb`.
- Rendered to a `RenderedPage { texture: egui::TextureHandle, links: Vec<LinkRect> }` via `typst-render` at 2× pixel density.
- `comemo::evict(0)` flushes memoization between renders so each frame sees a fresh compile.

## External Editor (Helix)

- `open_in_helix(path)` spawns `$TERMINAL -e hx <file>`
- Terminal priority: `alacritty → kitty → foot → wezterm → ghostty → gnome-terminal → konsole → xterm`
- Override with `$TERMINAL` env var
- Buffer is saved before Helix launches
- File-watcher reloads buffer after Helix writes (only if buffer is clean)

## Search

- **Content search**: line-by-line substring scan returning `ContentHit { path, line, snippet }`; shown below the file tree when a query is active
- **Tree filter**: the file tree filters notes by stem match and force-opens ancestor folders of matching notes
- **Backlinks**: parses `[[link-name]]` from all notes into a `HashMap<String, Vec<PathBuf>>`; shown in backlinks panel for the current note
- **`find_note_by_stem(vault, name)`**: resolves `#line-note` click targets. If `name` contains `/`, matches by vault-relative path (`"ideas/foo"` → `note/ideas/foo.typ`). Otherwise, case-insensitive stem match; first alphabetically wins. Path-qualified form is needed to distinguish between two notes with the same stem in different folders.

## UI Layout

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

- Left sidebar (`ui::vault_explorer`) is foldable (animated); shows a VS Code-style recursive file tree:
  - Folders render with `▶`/`▼` chevron; clicking anywhere on the row toggles expansion. State stored in egui's ephemeral memory.
  - Notes are leaf nodes indented one chevron-width deeper than their folder.
  - Notes use `Sense::click_and_drag()` (via `ui.interact` layered over the `SelectableLabel`) so click-to-open and drag-to-folder both work. A drag-only outer widget would suppress the inner click per egui's hit-tester rules.
  - Each folder row is a `dnd_drop_zone<PathBuf>`; dropping a note emits `AppAction::MoveNote`. A `📂 (root)` drop zone appears at the top while dragging.
  - Search filters notes by stem; ancestor folders of matches are force-opened.
- Backlinks panel appears below the editor for the currently open note
- Editor column is fixed at `CONTENT_WIDTH_PT` (800pt) and centered when the viewport is wider; an outer `ScrollArea` handles horizontal overflow when narrower

## Key Shortcuts

| Key      | Action                          |
|----------|---------------------------------|
| `Ctrl+S` | Save current note               |
| `Ctrl+E` | Open current note in Helix      |

## Config

- No on-disk config file yet. Two layers of in-code tunables:
  - **Global style** — constants in `src/style.rs`: `UI_PT`, `EDITOR_PT`, `CONTENT_WIDTH_PT`, `SANS_FAMILIES`, `EDIT_OUTLINE_COLOR`.
  - **Per-editor** — `style::EditorConfig` (font, `line_space`, `SyntaxColors`) exposed as `MixedEditor::config`; mutate at runtime to retheme.
- **Logging**: `RUST_LOG` overrides the default `info,vellum=debug` filter (`env_logger`). Traces cover vault open/scan, note open/save/reload, segment parsing, render bursts, compile errors, file-watcher events, and Helix launches.
- **External editor**: `$TERMINAL` env var overrides the auto-detection order in `external_editor.rs`.

## Dependencies

| Crate              | Purpose                                          |
|--------------------|--------------------------------------------------|
| `eframe/egui`      | GUI framework                                    |
| `walkdir`          | Recursive vault directory scan                   |
| `notify`           | File-system watcher for external edits           |
| `typst`            | In-process Typst compilation                     |
| `typst-render`     | Rasterize compiled Typst pages to pixel data     |
| `typst-assets`     | Bundled fonts (incl. New Computer Modern Math)   |
| `fontdb`           | System font discovery                            |
| `comemo`           | Memoization cache management for Typst           |
| `regex`            | Wiki-link extraction and content search          |
| `dirs`             | Resolve `~/vellum` vault path                    |
| `anyhow`           | Error handling                                   |
| `log` + `env_logger` | Structured debug tracing                       |
| `serde/toml`       | (Reserved for future on-disk config)             |

## Future Work

- Tantivy full-text index (blocked on zstd dependency conflict)
- Optional git sync (init/commit/push/pull from the UI)
- `[[link]]` click-to-navigate in the editor
- Note rename propagates `[[links]]` across vault
- On-disk config (vault path, terminal, Helix theme, sizing knobs)
- Activity-reset for the caret blink (stay solid for ~500ms after a keystroke)
