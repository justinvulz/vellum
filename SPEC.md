# Vellum — Specification

Vellum is a desktop note-taking app inspired by Obsidian. Notes are Typst (`.typ`) documents stored in a local vault directory and previewed as rendered PDFs inside the app. Real editing is done in Helix, launched as an external process.

---

## Goals

- Store notes as plain `.typ` files (human-readable, git-friendly)
- Live PDF preview compiled from Typst source
- Obsidian-style `[[wiki-links]]` and backlink tracking
- Minimal UI — egui, no Electron, no browser
- Helix as the editor; the app is a launcher + preview shell

---

## Vault

- Default location: `~/vellum/`
- Recursively scans for `.typ` files on start and after any file change
- CRUD: create, read, write, delete, rename notes
- `display_name` strips the vault root prefix for display
- `default_vault_dir()` falls back to `./vellum` if home dir is unavailable

## Editor

- Plain egui `TextEdit` buffer for lightweight in-app editing
- `Ctrl+S` saves and triggers recompile
- `Ctrl+E` opens the current note in Helix (external)
- File-watcher detects external writes and auto-reloads (if buffer is clean)

## External Editor (Helix)

- `open_in_helix(path)` spawns `$TERMINAL -e hx <file>`
- Terminal priority: `alacritty → kitty → foot → wezterm → ghostty → gnome-terminal → konsole → xterm`
- Override with `$TERMINAL` env var
- Buffer is saved before Helix launches
- File-watcher reloads buffer after Helix writes

## Typst Preview

- Pipeline: Typst CLI → PDF → pdfium rasterization → egui textures
- `typst compile --root <vault> <source> <tmp>.pdf`
- pdfium rasterizes each page at `target_width = 2400px`
- Pages stored as `egui::TextureHandle`; rendered in a scroll area
- On compile error: shows error message + raw source fallback
- "Open PDF" button launches the PDF in the system viewer (`xdg-open`)
- pdfium requires `PDFIUM_DYNAMIC_LIB_PATH` pointing to `libpdfium.so` (set by Nix dev shell)

## Search

- **Filename search**: case-insensitive substring match on note stems; shown in sidebar
- **Content search**: line-by-line substring scan returning `ContentHit { path, line, snippet }`
- **Backlinks**: parses `[[link-name]]` from all notes into a `HashMap<String, Vec<PathBuf>>`; shown in backlinks panel for the current note

## Git Sync

- Optional; `GitSync` struct wraps `git init`, `git add -A && git commit`, push/pull
- Not yet wired to UI; stub is in `src/git.rs`

## UI Layout

```
┌─────────────────────────────────────────────────────┐
│  Split | Tabs          [mode]         [status] [●]  │  ← topbar
├──────────┬──────────────────────────┬───────────────┤
│  Vault   │                          │               │
│  Search  │    TextEdit (editor)     │  PDF Preview  │
│  ──────  │    [Save] [Helix] [Rel.] │  [Helix][PDF] │
│  Notes   │                          │               │
│          ├──────────────────────────┤               │
│          │   Backlinks panel        │               │
└──────────┴──────────────────────────┴───────────────┘
```

- **Split mode**: sidebar + editor + preview + backlinks (default)
- **Tab mode**: sidebar + toggle between editor and preview

## Key Shortcuts

| Key      | Action                          |
|----------|---------------------------------|
| `Ctrl+S` | Save current note               |
| `Ctrl+E` | Open current note in Helix      |

## Config

- `~/.config/vellum/config.toml` (parsed by `src/git.rs`; not yet fully used)

## Dependencies

| Crate           | Purpose                                      |
|-----------------|----------------------------------------------|
| `eframe/egui`   | GUI framework                                |
| `walkdir`       | Recursive vault directory scan               |
| `notify`        | File-system watcher for external edits       |
| `pdfium-render` | In-process PDF rasterization                 |
| `regex`         | Wiki-link extraction and content search      |
| `dirs`          | Resolve `~/vellum` vault path                |
| `anyhow`        | Error handling                               |
| `serde/toml`    | Config file parsing                          |

## Future Work

- Tantivy full-text index (blocked on zstd dependency conflict)
- Git sync UI (commit/push/pull buttons)
- `[[link]]` click-to-navigate in the editor
- Note rename propagates `[[links]]` across vault
- Config UI for vault path, terminal, Helix theme
