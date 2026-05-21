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
inputs.vellum.url = "github:justinChen/vellum";
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
inputs.vellum.url = "github:justinChen/vellum";
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
nix run github:justinChen/vellum
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

> `theme.typ` is rewritten on every launch. To customise it, edit `assets/default_theme.typ` in this repo and rebuild.

## Shortcuts

| Key      | Action                     |
|----------|----------------------------|
| `Ctrl+S` | Save current note          |
| `Ctrl+E` | Open current note in Helix |

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
