{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixpkgs-unstable";
    flake-parts.url = "github:hercules-ci/flake-parts";
    systems.url = "github:nix-systems/default";
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    crane.url = "github:ipetkov/crane";
    pre-commit-hooks = {
      url = "github:cachix/pre-commit-hooks.nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    treefmt-nix = {
      url = "github:numtide/treefmt-nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = inputs:
    inputs.flake-parts.lib.mkFlake {inherit inputs;} {
      systems = import inputs.systems;
      imports = [
        inputs.pre-commit-hooks.flakeModule
        inputs.treefmt-nix.flakeModule
      ];
      perSystem = {
        system,
        config,
        self',
        pkgs,
        lib,
        ...
      }: let
        toolchain = pkgs.fenix.fromToolchainFile {
          file = ./rust-toolchain.toml;
          sha256 = "sha256-X/4ZBHO3iW0fOenQ3foEvscgAPJYl2abspaBThDOukI=";
        };
        craneLib =
          (inputs.crane.mkLib pkgs).overrideToolchain toolchain;
        commonArgs = {
          src = lib.cleanSource ./.;
          strictDeps = true;
        };
        cargoArtifacts = craneLib.buildDepsOnly commonArgs;
      in {
        _module.args.pkgs = import inputs.nixpkgs {
          inherit system;
          overlays = [
            inputs.fenix.overlays.default
          ];
        };

        treefmt = {
          programs = {
            rustfmt = {
              enable = true;
            };
            clang-format.enable = true;
            alejandra.enable = true;
            taplo.enable = true;
            yamlfmt = {
              enable = true;
              settings = {
                formatter = {
                  include_document_start = true;
                  pad_line_comments = 2;
                };
              };
            };
          };
        };

        pre-commit.settings = {
          settings = {
            rust.check.cargoDeps = pkgs.rustPlatform.importCargoLock {lockFile = ./Cargo.lock;};
          };
          hooks = {
            yamllint.enable = true;
            treefmt = {
              enable = true;
              package = config.treefmt.build.wrapper;
            };
            clippy = {
              enable = true;
              packageOverrides.cargo = toolchain;
              packageOverrides.clippy = toolchain;
              extraPackages = self'.packages.default.buildInputs ++ self'.packages.default.nativeBuildInputs;
            };
            cargo-machete = {
              enable = true;
              name = "cargo-machete";
              description = "Remove unused Rust dependencies with this one weird trick!";
              language = "rust";
              pass_filenames = false;
              entry = lib.getExe pkgs.cargo-machete;
            };
            zizmor = {
              name = "zizmor";
              description = "Find security issues in GitHub Actions CI/CD setups";
              language = "python";
              types = ["yaml"];
              files = "(\.github/workflows/.*)|(action\.ya?ml)$";
              require_serial = true;
              entry = lib.getExe pkgs.zizmor;
            };
            deadnix = {
              enable = true;
              args = ["--edit"];
            };
            statix = {
              enable = true;
              settings = {
                format = "stderr";
              };
            };
            nil.enable = true;
            ripsecrets.enable = true;
          };
        };

        devShells.default = let
          buildInputs = with pkgs; [
            expat
            fontconfig
            freetype
            freetype.dev
            libGL
            pkg-config
            xorg.libX11
            xorg.libXcursor
            xorg.libXi
            xorg.libXrandr
            wayland
            vulkan-loader
            libxkbcommon
            xdg-desktop-portal-gtk
          ];
        in
          craneLib.devShell {
            packages =
              [
                pkgs.trunk
                pkgs.bacon
                pkgs.just
                pkgs.cargo-watch
                pkgs.nix-output-monitor
                config.treefmt.build.wrapper
                pkgs.git-cliff
              ]
              ++ self'.packages.default.buildInputs
              ++ self'.packages.default.nativeBuildInputs
              ++ buildInputs;
            LD_LIBRARY_PATH = lib.makeLibraryPath buildInputs;
            shellHook = config.pre-commit.installationScript;
          };

        packages = {
          paper-age-gui = craneLib.buildPackage (commonArgs
            // {
              inherit cargoArtifacts;
              doCheck = false;
              postFixup = lib.optionalString pkgs.stdenv.isLinux ''
                patchelf $out/bin/paper-age-gui \
                  --add-rpath ${lib.makeLibraryPath (with pkgs; [vulkan-loader xorg.libX11 libxkbcommon wayland])}
              '';
            });
          default = self'.packages.paper-age-gui;
        };
      };
    };
}
