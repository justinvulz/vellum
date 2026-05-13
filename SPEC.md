# Vellum — Specification

Vellum is a desktop note-taking app inspired by Obsidian. Notes are Typst (`.typ`) documents stored in a local vault directory. The app provides a mixed inline editor where plain text is edited directly and Typst blocks (math, tables, lists, code) are rendered in-process and editable on click.

---

## Goals

- Store notes as plain `.typ` files (human-readable, git-friendly)
- Mixed inline editing: plain prose editable directly, Typst blocks rendered live
- Obsidian-style `[[wiki-links]]` and backlink tracking
- Minimal UI — egui, no Electron, no browser
- Helix as optional external editor; the app is self-sufficient for basic editing

---

## Vault

- Default location: `~/vellum/`
- Subdirectory structure:
  - `~/vellum/note/` — all `.typ` note files
  - `~/vellum/asset/` — images, theme template, and other shared assets
  - `~/vellum/typst.toml` — Typst package manifest for LSP root resolution
- `asset/theme.typ` is auto-generated on first run (dark theme template)
- Recursively scans `note/` for `.typ` files on start and after any file change
- CRUD: create, read, write, delete, rename notes
- `display_name` strips the vault root prefix for display
- `default_vault_dir()` falls back to `./vellum` if home dir is unavailable

## Mixed Inline Editor

The core editing experience is a segment-based mixed editor (`MixedEditor`):

### Segment Classification

Source is split on blank lines into paragraphs. Each paragraph is classified as:
- **`Typst`**: contains `#` or `$`, or starts with `=` / `-` / `+` / `/`
- **`Plain`**: everything else

### Editing Behavior

- **Plain segments**: rendered as inline `egui::TextEdit` — edit directly in place
- **Typst segments (idle)**: rendered as an image via in-process Typst compilation; click to edit
- **Typst segments (editing)**: shown as a `code_editor` `TextEdit`; focus loss re-parses and re-renders
- **Compile error**: shows error message + raw source text; click to edit

### Preamble Propagation

The first N contiguous Typst segments containing only `#let` / `#import` / `#set` / `#show` / blank / comment lines are the "preamble". The preamble is prepended to every subsequent Typst segment before compilation so that variable bindings and imports are visible across all blocks in the note.

### Render Cache

Rendered textures are cached by *effective source* (preamble + block body). Failed compiles are cached to avoid retrying every frame. The cache is content-addressed, so unchanged blocks survive note reloads.

## Typst Engine

- In-process compilation using the `typst` crate (0.14)
- Each snippet is wrapped: `#import "/asset/theme.typ": template\n#show: template\n\n{snippet}`
- Fonts: bundled via `typst-assets` (includes New Computer Modern Math) + system fonts via `fontdb`
- Rendered to `egui::TextureHandle` via `typst-render` at 2× pixel density
- `comemo::evict(0)` flushes memoization between renders

## External Editor (Helix)

- `open_in_helix(path)` spawns `$TERMINAL -e hx <file>`
- Terminal priority: `alacritty → kitty → foot → wezterm → ghostty → gnome-terminal → konsole → xterm`
- Override with `$TERMINAL` env var
- Buffer is saved before Helix launches
- File-watcher reloads buffer after Helix writes (only if buffer is clean)

## Search

- **Filename search**: case-insensitive substring match on note stems; shown in sidebar
- **Content search**: line-by-line substring scan returning `ContentHit { path, line, snippet }`
- **Backlinks**: parses `[[link-name]]` from all notes into a `HashMap<String, Vec<PathBuf>>`; shown in backlinks panel for the current note

## Git Sync

- Optional; `GitSync` struct wraps `git init`, `git add -A && git commit`, push/pull
- Not yet wired to UI; stub is in `src/git.rs`

## UI Layout

```
┌──────────────────────────────────────────────┐
│  [sidebar toggle]              [status]       │  ← topbar
├──────────┬───────────────────────────────────┤
│  Vault   │                                   │
│  Search  │    MixedEditor                    │
│  ──────  │    Plain segments: inline TextEdit │
│  Notes   │    Typst segments: rendered image  │
│          │      └─ click → source TextEdit    │
│          ├───────────────────────────────────┤
│          │   Backlinks panel                 │
└──────────┴───────────────────────────────────┘
```

- Left sidebar is foldable (animated); shows vault tree + search
- Backlinks panel appears below the editor for the currently open note

## Key Shortcuts

| Key      | Action                          |
|----------|---------------------------------|
| `Ctrl+S` | Save current note               |
| `Ctrl+E` | Open current note in Helix      |

## Config

- `~/.config/vellum/config.toml` (parsed by `src/git.rs`; not yet fully used)

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
| `serde/toml`       | Config file parsing                              |

## Future Work

- Tantivy full-text index (blocked on zstd dependency conflict)
- Git sync UI (commit/push/pull buttons)
- `[[link]]` click-to-navigate in the editor
- Note rename propagates `[[links]]` across vault
- Config UI for vault path, terminal, Helix theme
- Syntax highlighting in Typst source `TextEdit` blocks
