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
- **In-process Typst 0.14.** No external `typst` invocation — the app
  implements `typst::World` and compiles directly.
- **Preamble propagation.** Leading `#let` / `#import` / `#set` / `#show`
  segments are prepended to every later segment before compilation, so
  bindings and imports are in scope across the whole note.
- **Backlinks.** `[[note-name]]` references are indexed; the bottom panel shows
  notes that link to the current one.
- **Filename + content search.** Fuzzy filename match and a regex content
  search over the vault.
- **External Helix.** `Ctrl+E` launches Helix in your terminal on the current
  file; a file watcher reloads the buffer when Helix writes.
- **Centered, fixed-width column.** The editor column is 800pt wide. All
  segments share the same column and font — when the window is wider, the
  column centers; when narrower, a single horizontal scrollbar appears at
  the bottom of the editor.
- **Sans-serif by default.** Egui and the Typst theme both resolve to the same
  system sans-serif (Inter / Noto Sans / DejaVu Sans / …), kept identical
  between source `TextEdit`s and rendered output.

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

Most knobs live in `src/style.rs`:

| Constant            | Default | Effect                                 |
|---------------------|---------|----------------------------------------|
| `UI_PT`             | `14.0`  | Chrome size (topbar, sidebar, buttons) |
| `EDITOR_PT`         | `20.0`  | Mixed-editor body size                 |
| `CONTENT_WIDTH_PT`  | `800.0` | Editor column width                    |
| `SANS_FAMILIES`     | …       | Sans-serif fallback list               |

`UI_PT` and `EDITOR_PT` are decoupled, so the chrome and the editor body can be
tuned independently.

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
    mixed.rs           MixedEditor — per-segment render/edit state machine
    typst_engine.rs    in-process Typst 0.14 compiler, render to texture
  ui/
    mod.rs
    topbar.rs          sidebar toggle + dirty marker + status
    vault_explorer.rs  left sidebar (new note, search, note list)
    editor_view.rs     central editor panel
    backlinks_panel.rs bottom backlinks panel
```

Data flow:

1. `Vault::open_or_init` scans `~/vellum/note/` and rewrites `asset/theme.typ`
   from the embedded default.
2. Selecting a note loads it into `MixedEditor::load`, which calls
   `segment::parse_segments` to walk Typst's syntax tree and produce a
   `Vec<String>` of block segments.
3. Each segment is wrapped by `preamble::wrap_for_render` with
   `#show: template.with(width: 800pt, size: 20pt)` (plus the preamble of the
   note) and compiled in-process by `TypstEngine`. The render cache is keyed
   on the wrapped source string, so the template, width, size, and preamble
   all participate in invalidation.
4. Rendered pages become `egui::TextureHandle`s, drawn at `pixels / PIXEL_PER_PT`
   logical points so 1 typst pt ↔ 1 egui pt on screen.
5. Clicking a rendered segment flips it to a monospace source `TextEdit` with
   a blue edit outline; focus loss re-splits the buffer with the parser and
   re-renders.
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

Project guidance for AI assistants lives in [CLAUDE.md](./CLAUDE.md).

## License

Not yet declared.
