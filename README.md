# Vellum

A desktop note-taking app for [Typst](https://typst.app) documents, inspired by
Obsidian. Notes live as `.typ` files in a local vault. Every block — prose,
heading, math, list, function call — compiles in-process and renders as an
image, flipping to a source `TextEdit` on click. No PDF preview pipeline, no
external compiler.

![status: alpha](https://img.shields.io/badge/status-alpha-orange)

## Why

Obsidian-style backlinks and quick navigation, but with Typst as the source
language: math, tables, scripting, custom show rules, and proper typography out
of the box. The editor splits a note into block-level segments using Typst's
syntax tree, so headings, block math, and top-level `#`-calls each become their
own edit unit without needing blank-line separators.

```
┌──────────────────────────────────────────────────┐
│  ◀                                       saved   │
├──────────┬───────────────────────────────────────┤
│  Vellum  │                                       │
│  search… │   paragraph text                      │
│  ──────  │                                       │
│  note A  │   $ E = m c^2 $       ← block math    │
│  note B  │                                       │
│          │   #table(columns: 2)[a][b]            │
│          │                                       │
│          │   - list                              │
│          │   - styled via theme template         │
│          ├───────────────────────────────────────┤
│          │   Backlinks                           │
└──────────┴───────────────────────────────────────┘
```

## Features

- **Tree-based block editor.** Each note is split via Typst's syntax tree:
  headings (`= …`), block math (`$ … $`), and `#`-calls alone on a line each
  become their own segment. Inline use (`Hello #strong[bold] world`,
  `Hello $x$ world`) stays inside a single text segment. Click any rendered
  segment to flip it to a source `TextEdit` with a blue edit outline; click
  outside to re-render.
- **Syntax-highlighted source view.** When a segment is in edit mode the
  source is coloured per Typst's syntax tree — `$` math delimiters,
  `#`-code transitions, heading markers, comments, keywords, numbers,
  strings, list markers, brackets — re-running on every keystroke. Palette
  lives in `style::SyntaxColors` and is fully retheme-able.
- **Configurable edit mode.** `MixedEditor::config` (a `style::EditorConfig`)
  exposes font size and family, optional extra line spacing
  (`line_space`), and the syntax-colour palette. Mutate at runtime to
  retheme the editor.
- **Custom blinking caret.** A font-sized caret painted by the app
  (egui 0.27 otherwise ties caret height to row height, so wider lines
  would stretch the caret). Blinks at ~1 Hz with a wake-up timer.
- **In-process Typst 0.14.** No external `typst` invocation — the app
  implements `typst::World` and compiles directly.
- **Preamble propagation.** Leading `#let` / `#import` / `#set` / `#show`
  segments are prepended to every later segment before compilation, so
  bindings and imports are in scope across the whole note.
- **Backlinks.** `[[note-name]]` references are indexed; the bottom panel shows
  notes that link to the current one.
- **Inter-note links.** `#line-note("note-name")` (or
  `#line-note("note-name", body: [Display label])`) renders an inline blue
  link in compiled segments. Click to navigate. Resolution: if the name
  contains `/` it matches by vault-relative path (`"ideas/foo"` →
  `note/ideas/foo.typ`); otherwise a case-insensitive stem match is used
  (first alphabetically wins). The theme defines the function and the
  preamble auto-imports it, so user code never needs an explicit `#import`.
- **Folder organisation.** Notes can live in subdirectories of `note/`.
  Create folders with the "Folder" input, drag notes onto folder rows to
  move them, and right-click to delete empty folders.
- **VS Code-style file tree.** The sidebar shows a recursive tree with
  `▶`/`▼` chevrons for expand/collapse, depth indentation, and persistent
  expand state. Ancestor folders of search matches auto-open.
- **Content search.** Regex content search over the vault; shown below the
  file tree when a query is active.
- **External Helix.** `Ctrl+E` launches Helix in your terminal on the current
  file; a file watcher reloads the buffer when Helix writes.
- **Centered, fixed-width column.** The editor column is 800pt wide. All
  segments share the same column and font — when the window is wider, the
  column centers; when narrower, a single horizontal scrollbar appears at
  the bottom of the editor.
- **Sans-serif by default.** Egui and the Typst theme both resolve to the same
  system sans-serif (Inter / Noto Sans / DejaVu Sans / …), kept identical
  between source `TextEdit`s and rendered output.
- **Debug tracing.** Structured `log` + `env_logger` traces over vault,
  note ops, segment parsing, render bursts, compile errors, file-watcher
  events, and Helix launches; filterable via `RUST_LOG`.

## Install

### Nix (recommended)

```sh
nix develop        # enters shell with rustc, cargo, rust-analyzer, typst CLI
cargo run
```

The flake includes the X11, Wayland, libGL, and fontconfig runtime libraries
that `winit`/`eframe` `dlopen` at startup.

### Cargo (without Nix)

```sh
cargo build --release
cargo run --release
```

Requires a system C toolchain plus the usual Linux desktop libs: `libxkbcommon`,
`libGL`, `fontconfig`, and X11 or Wayland. On Wayland-only systems, leave the
defaults; on systems missing Wayland libs, run with:

```sh
WINIT_UNIX_BACKEND=x11 cargo run
```

## Vault layout

On first launch Vellum creates `~/vellum/`:

```
~/vellum/
  typst.toml        ← package manifest so tinymist resolves /asset/* imports
  note/             ← all .typ note files
  asset/
    theme.typ       ← dark-theme template (regenerated each launch)
    (images, …)
```

`~/vellum/asset/theme.typ` is **rewritten on every launch** from the embedded
default, because the app injects `template.with(width: …, size: …)` into every
snippet and an out-of-date signature would break compilation. Customize the
template by editing `assets/default_theme.typ` in this repo and rebuilding.

## Keyboard shortcuts

| Key      | Action                              |
|----------|-------------------------------------|
| `Ctrl+S` | Save current note                   |
| `Ctrl+E` | Open current note in Helix          |

Click a rendered segment to flip it to source-edit mode; click outside (or
move focus) to re-render.

## Configuration

Global style constants in `src/style.rs`:

| Constant            | Default | Effect                                 |
|---------------------|---------|----------------------------------------|
| `UI_PT`             | `14.0`  | Chrome size (topbar, sidebar, buttons) |
| `EDITOR_PT`         | `20.0`  | Mixed-editor body size                 |
| `CONTENT_WIDTH_PT`  | `800.0` | Editor column width                    |
| `SANS_FAMILIES`     | …       | Sans-serif fallback list               |
| `EDIT_OUTLINE_COLOR`| …       | Blue accent stroke on the active segment |

`UI_PT` and `EDITOR_PT` are decoupled, so the chrome and the editor body can be
tuned independently.

Per-editor knobs in `style::EditorConfig` (exposed as
`MixedEditor::config`, mutable at runtime):

```rust
editor.config.font_size = 18.0;
editor.config.font_family = egui::FontFamily::Monospace;
editor.config.line_space = Some(13.0);              // extra gap between lines
editor.config.colors.dollar = Color32::from_rgb(0xff, 0x00, 0xff);
```

`SyntaxColors` has fields for every highlighted token kind (`dollar`,
`hash`, `heading_marker`, `comment`, `string`, `number`, `keyword`,
`ident`, `punct`, `emphasis`, `list_marker`, `default`).

External-editor selection: set `$TERMINAL` to override the auto-detection in
`external_editor.rs`. The search order is `alacritty`, `kitty`, `foot`,
`wezterm`, `ghostty`, `gnome-terminal`, `konsole`, `xterm`.

## Architecture

```
src/
  main.rs              entry point, eframe boot, style install
  app.rs               App struct, event loop, AppAction dispatch, shortcuts
  vault.rs             vault scan, file CRUD, theme bootstrap
  search.rs            filename + content search, [[wiki-link]] backlinks
  style.rs             fonts, text styles, sizing constants, edit-mode outline
  external_editor.rs   spawn Helix in a terminal
  file_watcher.rs      notify vault of external .typ changes
  editor/
    mod.rs
    segment.rs         syntax-tree splitter (parse_segments)
    preamble.rs        preamble detection + theme-template source wrapping
    highlight.rs       Typst syntax highlighter for the source TextEdit
    mixed.rs           MixedEditor — per-segment render/edit state machine,
                       custom blinking caret, EditorConfig
    typst_engine.rs    in-process Typst 0.14 compiler, render to texture
  ui/
    mod.rs
    topbar.rs          sidebar toggle + dirty marker + status
    vault_explorer.rs  left sidebar (new note, search, note list)
    editor_view.rs     central editor panel
    backlinks_panel.rs bottom backlinks panel
```

Data flow:

1. `Vault::open_or_init` scans `~/vellum/note/` recursively, populating
   `notes` and `folders`, and rewrites `asset/theme.typ` from the embedded default.
2. Selecting a note loads it into `MixedEditor::load`, which calls
   `segment::parse_segments` to walk Typst's syntax tree and produce a
   `Vec<String>` of block segments.
3. Each frame, `ensure_rendered` compiles uncached segments up to a 16 ms
   budget. Segments beyond the budget show `⟳ rendering…` and are compiled
   on the next repaint — long notes load progressively rather than freezing.
   The cache is keyed on the fully-wrapped source (template + preamble + body).
4. Rendered pages become `RenderedPage { texture, links }`, where the
   texture is drawn at `pixels / PIXEL_PER_PT` (1 typst pt ↔ 1 egui pt) and
   `links` is the set of `vellum://`-scheme link rectangles walked out of
   the compiled `page.frame`.
5. Clicking a rendered segment flips it to a monospace source `TextEdit` with
   a blue edit outline — unless the click landed on a link rectangle, in which
   case the editor emits `OpenNoteByName(target)` and navigates instead.
   Focus loss re-splits the buffer with the parser and re-renders.
6. `Ctrl+S` joins segments back with `\n\n` and writes to disk.
7. The file watcher notices external writes and reloads the buffer if it
   isn't dirty.

### Segment splitting

`segment::parse_segments` walks the top-level children of Typst's `Markup`
node and emits a segment for each:

- `Heading` (`= …`) — always its own segment.
- `Equation` where `ast::Equation::block()` is true (block math, `$ … $` with
  whitespace immediately inside the dollars) — own segment.
- `Hash` + following code expression (`FuncCall`, `LetBinding`, `SetRule`,
  `ShowRule`, `ModuleImport`, …) — own segment **only when the pair is alone
  on its source line**, so inline uses like `Hello #strong[bold] world` stay
  inside one text segment.
- `Parbreak` (blank line) — ends the current text segment.
- Anything else (`Text`, `Space`, list/enum/term items, inline math, inline
  `Strong`/`Emph`, …) — accumulates into the current text segment.

A blank line *inside* a function call's content block is not a top-level
`Parbreak`, so multi-line `#table(…)[…]` stays as one segment.

### Preamble propagation

`preamble::collect` walks the leading run of "preamble-only" segments — those
whose lines start only with `#let` / `#import` / `#set` / `#show`, `//`
comments, or are blank. The joined preamble text is prepended to every later
segment before compilation, so bindings and imports flow through every block.

### Inter-note links

`assets/default_theme.typ` defines `#let line-note(name, body: none) =
link("vellum://" + name, …)`. `preamble::wrap_for_render` adds it to the
auto-import alongside `template`, so user code can write `#line-note("X")`
without an explicit `#import`. The function compiles to a normal Typst
`link`, so Typst records the rectangle and destination as a
`FrameItem::Link(Destination::Url, Size)` on the page frame.

After each compile, `TypstEngine::render` walks `page.frame` recursively
(folding group translations into the accumulated origin) and pulls out every
`vellum://` link as a `LinkRect { rect, target }` in typst points — which is
also egui points by construction. `MixedEditor::show_rendered` hit-tests
clicks against those rectangles in the image's local coordinates; matches
emit `AppAction::OpenNoteByName(name)`, which `App::open_note_by_name`
resolves via `search::find_note_by_stem` and opens (or surfaces "note not
found" in the status line).

## Development

```sh
cargo build
cargo test
cargo clippy
cargo run
```

Single test:

```sh
cargo test segment::tests::heading_splits_without_blank_line
```

Debug logging is wired through `log` + `env_logger`. The default filter
is `info,vellum=debug`, so vault state, note open/save, segment counts,
render bursts, watcher events, and Helix launches appear on stderr.
Override with `RUST_LOG`:

```sh
RUST_LOG=trace cargo run                          # everything
RUST_LOG=vellum::editor=trace,info cargo run      # editor-only trace
RUST_LOG=warn cargo run                           # quiet — only warnings + errors
```

Project guidance for AI assistants lives in [CLAUDE.md](./CLAUDE.md).

## License

Not yet declared.
