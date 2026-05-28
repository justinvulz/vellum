# Vellum Roadmap

## Phase 1 — Config & Usability

- ~~**On-disk config** (`~/.config/vellum/config.toml`) — vault path, terminal, sizing knobs, theme colors.~~ **Done.** `src/config.rs` loads at startup; commented sample written on first run.
- **More keyboard shortcuts** — `Ctrl+N` new note, `Ctrl+W` close/unload, `Ctrl+F` focus search, `Ctrl+[` sidebar toggle
- **Recently opened notes** — simple `Vec<PathBuf>` persisted to config
- **Scroll position preservation** — remember scroll offset per note when switching

## Phase 2 — Editor Completeness

- **Undo history across saves** — currently each save resets state; a simple `VecDeque<String>` per note
- **Image insert** — drag image file into editor → copy to `asset/`, insert `#image("/asset/foo.png")`
- **Note templates** — `~/vellum/template/` directory; "new note" dialog offers a picker
- **Theme customization without rebuild** — move theme edits to on-disk config; stop overwriting `theme.typ` every launch (currently flagged as a sharp edge in README)

## Phase 3 — Navigation & Discovery

- **Outline panel / table of contents** — sidebar tab showing heading tree for the current note (headings already parsed as segments)
- **`[[...]]` quick-insert** — typing `[[` opens a note-picker autocomplete that inserts `#line-note("...")`. Read-only; `#line-note` stays the canonical link format.
- **Keyboard navigation in file tree** — arrow keys, Enter to open, Delete to delete
- **Note graph view** — egui canvas showing notes as nodes, `#line-note` edges (backlink data already computed)

## Phase 4 — Search & Index

- **Tantivy full-text index** — blocked on zstd dependency conflict; worth revisiting or forking zstd linkage. Current regex scan is O(n·files) per keystroke.
- **Search results in main panel** — currently content hits show below the file tree; promote to a first-class search results view

## Phase 5 — Sync & Collaboration

- **Git sync UI** — `init / commit / push / pull` buttons in topbar; vault is already plain files, git-friendly
- **Multiple vaults** — vault switcher in topbar; each vault is independent state

## Phase 6 — Polish & Distribution

- **macOS/Windows CI builds** — current release infra is Linux-only
- **Light theme** — `SyntaxColors` already configurable; just needs a light palette + toggle
- **Export to PDF** — "Save as PDF" via Typst's own PDF output (already available in the `typst` crate)
- **Tinymist LSP integration** — launch `tinymist` as a subprocess; proxy diagnostics and completions into the `TextEdit`
