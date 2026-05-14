# Vellum

A desktop note-taking app for [Typst](https://typst.app) documents, inspired by
Obsidian. Notes live as `.typ` files in a local vault and render in-process
alongside editable plain text — no PDF preview pipeline, no external compiler.

![status: alpha](https://img.shields.io/badge/status-alpha-orange)

## Why

Obsidian-style backlinks and quick navigation, but with Typst as the source
language: math, tables, scripting, custom show rules, and proper typography out
of the box. The editor is **mixed inline**: plain paragraphs are editable
directly with a normal `TextEdit`, Typst blocks render as images and flip to
source-editing on click.

```
┌──────────────────────────────────────────────────┐
│  ◀                                       saved   │
├──────────┬───────────────────────────────────────┤
│  Vellum  │                                       │
│  search… │   plain paragraph, editable inline    │
│  ──────  │                                       │
│  note A  │   $ E = m c^2 $   ← rendered Typst    │
│  note B  │                                       │
│          │   - list items                        │
│          │   - render through theme template     │
│          ├───────────────────────────────────────┤
│          │   Backlinks                           │
└──────────┴───────────────────────────────────────┘
```

## Features

- **Mixed inline editor.** Paragraphs without Typst markup are edited as plain
  text; paragraphs containing `#`, `$`, or starting with `=`, `-`, `+`, `/`
  render through the Typst compiler. Click a rendered block to edit its source.
- **In-process Typst 0.14.** No external `typst` invocation — the app
  implements `typst::World` and compiles directly.
- **Backlinks.** `[[note-name]]` references are indexed; the bottom panel shows
  notes that link to the current one.
- **Filename + content search.** Fuzzy filename match and a regex content
  search over the vault.
- **External Helix.** `Ctrl+E` launches Helix in your terminal on the current
  file; a file watcher reloads the buffer when Helix writes.
- **Centered, fixed-width column.** The editor column is 800pt wide. Plain text
  and rendered blocks share the same column and font — when the window is
  wider, the column centers; when narrower, a single horizontal scrollbar
  appears at the bottom of the editor.
- **Sans-serif by default.** Egui and the Typst theme both resolve to the same
  system sans-serif (Inter / Noto Sans / DejaVu Sans / …), kept identical
  between plain text and rendered output.

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

Click a rendered Typst block to flip it to source-edit mode; click outside (or
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
  app.rs               App struct, event loop, keyboard, panel coordination
  vault.rs             vault scan, file CRUD, theme bootstrap
  search.rs            filename + content search, [[wiki-link]] backlinks
  style.rs             font install, text-style sizing, layout constants
  external_editor.rs   spawn Helix in a terminal
  file_watcher.rs      notify vault of external .typ changes
  editor/
    mod.rs
    segment.rs         paragraph parser, Plain vs Typst classification
    mixed.rs           mixed inline editor (MixedEditor)
    typst_engine.rs    in-process Typst 0.14 compiler, render to texture
  ui/
    mod.rs
    vault_explorer.rs  left sidebar
    editor_view.rs     central editor panel
    backlinks_panel.rs bottom backlinks panel
```

Data flow:

1. `Vault::open_or_init` scans `~/vellum/note/` and rewrites `asset/theme.typ`.
2. Selecting a note loads it into `MixedEditor::load`, which parses paragraphs
   into `Plain` / `Typst` segments.
3. Typst segments are wrapped with `#show: template.with(width: 800pt, size:
   20pt)` and compiled in-process by `TypstEngine`. The render cache is keyed
   on the wrapped source string (so preamble propagation, width, and size all
   participate).
4. Rendered pages become `egui::TextureHandle`s, drawn at `pixels / PIXEL_PER_PT`
   logical points so 1 typst pt ↔ 1 egui pt on screen.
5. `Ctrl+S` serializes segments back to source and writes to disk.
6. The file watcher notices external writes and reloads the buffer if it isn't
   dirty.

### Segment classification

A paragraph (blank-line-separated block) is `Typst` if it contains `#` or `$`
anywhere, or any line begins with `=`, `- `, `+ `, `/ `. Otherwise it's
`Plain`.

### Preamble propagation

The initial run of `#let` / `#import` / `#set` / `#show` / comment-only Typst
segments is the "preamble". Each subsequent Typst block is compiled with the
preamble prepended, so bindings and imports flow through every block.

## Development

```sh
cargo build
cargo test
cargo clippy
cargo run
```

Single test:

```sh
cargo test segment::tests::round_trip
```

Project guidance for AI assistants lives in [CLAUDE.md](./CLAUDE.md).

## License

Not yet declared.
