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
      ];

      vellum = pkgs.rustPlatform.buildRustPackage {
        pname = "vellum";
        version = "0.1.0";
        src = ./.;
        cargoLock.lockFile = ./Cargo.lock;

        nativeBuildInputs = with pkgs; [
          pkg-config
          makeWrapper
        ];
        buildInputs = runtimeLibs;

        # The compiled binary doesn't link against the dlopen'd libs, so the
        # store path doesn't pull them in; wrap LD_LIBRARY_PATH at runtime.
        postFixup = ''
          wrapProgram $out/bin/vellum \
            --prefix LD_LIBRARY_PATH : ${pkgs.lib.makeLibraryPath runtimeLibs}
        '';

        meta = with pkgs.lib; {
          description = "Typst-native desktop note-taking app";
          mainProgram = "vellum";
          platforms = platforms.linux;
        };
      };
    in
    {
      packages.${system} = {
        default = vellum;
        vellum = vellum;
      };

      apps.${system}.default = {
        type = "app";
        program = "${vellum}/bin/vellum";
      };

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
      };
    };
}
