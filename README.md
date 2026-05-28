# Vellum
<p align="center">
  <img src="assets/vellum_logo.svg" alt="Vellum logo" width="200"/>
</p>

A desktop note-taking app for [Typst](https://typst.app) documents, inspired by Obsidian. Notes live as `.typ` files in a local vault. Every block compiles in-process and renders as an image — click any segment to edit it as source.

![status: alpha](https://img.shields.io/badge/status-alpha-orange)

## Features

- **Block editor.** Headings, block math, and top-level `#`-calls each become their own edit unit. Click a rendered block to flip it to a source `TextEdit`; click outside to re-render.
- **Syntax highlighting.** Edit mode colours Typst syntax — math delimiters, `#`-code, headings, comments, keywords, strings, and more.
- **In-process Typst 0.14.** No external compiler — the app implements `typst::World` directly.
- **Preamble propagation.** Leading `#let` / `#import` / `#set` / `#show` segments are in scope across the whole note.
- **Inter-note links.** `#line-note("note-name")` renders a clickable inline link. Supports vault-relative paths (`"ideas/foo"`) and case-insensitive stem lookup.
- **VS Code-style file tree.** Folders with expand/collapse chevrons, drag-to-move notes, and search that auto-opens ancestor folders of matches.
- **External Helix.** `Ctrl+E` opens the current file in Helix; the file watcher reloads when Helix writes.
- **Progressive rendering.** Long notes load top-to-bottom within a 16 ms per-frame budget — the UI stays responsive.

## Install

### NixOS (declarative)

Add Vellum to your NixOS flake configuration:

**`flake.nix`**
```nix
inputs.vellum.url = "github:justinvulz/vellum/latest";
```

**`configuration.nix`** (or any NixOS module)
```nix
{ inputs, ... }: {
  imports = [ inputs.vellum.nixosModules.default ];
  programs.vellum.enable = true;
}
```

### Home Manager (per-user declarative)

**`flake.nix`**
```nix
inputs.vellum.url = "github:justinvulz/vellum/latest";
```

**`home.nix`** (or any Home Manager module)
```nix
{ inputs, ... }: {
  imports = [ inputs.vellum.homeManagerModules.default ];
  programs.vellum.enable = true;
}
```

### Pre-built binary (GitHub Releases)

Download the latest tarball from [Releases](https://github.com/justinvulz/vellum/releases), then:

```sh
tar -xzf vellum-*-x86_64-linux.tar.gz
chmod +x vellum
./vellum
```

The binary dynamically links against system libraries. Install them first if missing:

```sh
# Ubuntu / Debian
sudo apt install libvulkan1 libwayland-client0 libxkbcommon0 libfontconfig1 libx11-6

# Fedora / RHEL
sudo dnf install vulkan-loader wayland-devel libxkbcommon fontconfig libX11

# Arch
sudo pacman -S vulkan-icd-loader wayland libxkbcommon fontconfig libx11
```

You also need a Vulkan-capable GPU driver (Mesa or proprietary). Most desktop Linux systems already have this.

### macOS

[nix-darwin](https://github.com/nix-darwin/nix-darwin) is the recommended path — same flake setup as NixOS:

```nix
{ inputs, ... }: {
  imports = [ inputs.vellum.nixosModules.default ];
  programs.vellum.enable = true;
}
```

Without nix-darwin, build from source (see [From source](#from-source-nix-dev-shell)). No pre-built macOS binaries are published.

### Windows

No pre-built Windows binaries are published — build from source with `cargo build --release` after installing the Rust toolchain. You will also need a Vulkan-capable GPU driver.

### Try without installing

```sh
nix run github:justinvulz/vellum/latest
```

### From source (Nix dev shell)

```sh
nix develop
cargo run
```


Requires a C toolchain and Linux desktop libs: `libxkbcommon`, `libGL`, `fontconfig`, and X11 or Wayland. On systems missing Wayland:

```sh
WINIT_UNIX_BACKEND=x11 cargo run
```

## Vault layout

On first launch Vellum creates `~/vellum/`:

```
~/vellum/
  typst.toml        ← package manifest (tinymist LSP support)
  note/             ← all .typ note files
  asset/
    theme.typ       ← dark theme (regenerated each launch)
```

> `theme.typ` is rewritten on every launch. To customise the template, edit `assets/default_theme.typ` in this repo and rebuild. The page fill, body text colour, and `#line-note` link colour come from your `[ui_colors]` config — those three values are substituted into the template at write time so rendered blocks match the surrounding UI.

## Shortcuts

| Key      | Action                     |
|----------|----------------------------|
| `Ctrl+S` | Save current note          |
| `Ctrl+E` | Open current note in Helix |

## Config

On first launch Vellum copies its bundled defaults (`assets/default_config.toml`) to `~/.config/vellum/config.toml`. Every field is optional — the bundled defaults are merged behind your file on every load, so you can omit any field (or any whole table) and the default fills it in.

- `vault_path` — override the default `~/vellum` location (leading `~/` is expanded).
- `terminal` — preferred terminal for `Ctrl+E` (falls back to `$TERMINAL`, then auto-detection).
- `ui_pt`, `editor_pt`, `content_width_pt` — sizing knobs, in typographic points.
- `sans_families` — sans-serif faces tried in priority order, both in egui and in the Typst theme.
- `cjk_families` — CJK fallback faces appended to both Proportional and Monospace.
- `[ui_colors]` — chrome palette (panel surfaces, hover/active fills, accent, body text). Hex strings.
- `[colors]` — syntax-highlighter palette. Hex strings like `"#d4d4d4"`.

A malformed file logs a warning and is treated as if absent — the app never refuses to start.

## Development

```sh
cargo build
cargo test
cargo run
```

Debug logging via `RUST_LOG` (default: `info,vellum=debug`):

```sh
RUST_LOG=trace cargo run
RUST_LOG=warn cargo run
```

Architecture and implementation notes live in [CLAUDE.md](./CLAUDE.md).

## License

GPL-3.0. See [LICENSE](./LICENSE).
