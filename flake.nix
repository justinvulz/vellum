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
        vulkan-loader
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
        version = (builtins.fromTOML (builtins.readFile ./Cargo.toml)).package.version;
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
          license = licenses.gpl3Only;
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

      # Overlay for use in nixpkgs-based configs.
      overlays.default = _final: _prev: { inherit vellum; };

      # NixOS module for declarative installation.
      nixosModules.default =
        { config, lib, pkgs, ... }:
        {
          options.programs.vellum.enable = lib.mkEnableOption "Vellum Typst note-taking app";

          config = lib.mkIf config.programs.vellum.enable {
            environment.systemPackages = [
              self.packages.${pkgs.stdenv.hostPlatform.system}.default
            ];
          };
        };

      # Home Manager module for per-user installation.
      homeManagerModules.default =
        { config, lib, pkgs, ... }:
        {
          options.programs.vellum.enable = lib.mkEnableOption "Vellum Typst note-taking app";

          config = lib.mkIf config.programs.vellum.enable {
            home.packages = [
              self.packages.${pkgs.stdenv.hostPlatform.system}.default
            ];
          };
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
            gh
          ]
          ++ runtimeLibs;

        # Make the dlopen()'d libs discoverable to `cargo run`.
        LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath runtimeLibs;
      };
    };
}
