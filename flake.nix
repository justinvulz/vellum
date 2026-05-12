{
  description = "Vellum — Typst note-taking desktop app";
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
  };

  outputs =
    { self, nixpkgs, ... }:
    let
      system = "x86_64-linux";
      pkgs = import nixpkgs { inherit system; };

      # Runtime libs that winit / eframe dlopen at startup.
      runtimeLibs = with pkgs; [
        wayland
        libxkbcommon
        libGL
        fontconfig
        # X11 fallback (used when WINIT_UNIX_BACKEND=x11)
        libX11
        libXcursor
        libXrandr
        libXi
        libxcb
        # PDF rendering (pdfium-render dlopens libpdfium.so)
        pdfium-binaries
      ];
    in
    {
      devShells.${system}.default = pkgs.mkShell {
        packages =
          with pkgs;
          [
            rustc
            cargo
            rust-analyzer
            pkg-config
            typst
            xdg-utils
          ]
          ++ runtimeLibs;

        # Make the dlopen()'d libs discoverable to `cargo run`.
        LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath runtimeLibs;
        # pdfium-render checks this env var to find libpdfium.so explicitly.
        PDFIUM_DYNAMIC_LIB_PATH = "${pkgs.pdfium-binaries}/lib";
      };
    };
}
