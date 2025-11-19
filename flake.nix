{
  description = "Fast and minimal program launcher for macOS";
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };
  outputs = {
    self,
    nixpkgs,
    rust-overlay,
  }: let
    forAllSystems = nixpkgs.lib.genAttrs ["x86_64-darwin" "aarch64-darwin"];
    nixpkgsFor = forAllSystems (system:
      import nixpkgs {
        inherit system;
        overlays = [rust-overlay.overlays.default];
      });
    cargoToml = builtins.fromTOML (builtins.readFile ./Cargo.toml);
  in {
    packages = forAllSystems (
      system: let
        pkgs = nixpkgsFor.${system};
        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          extensions = ["rust-src" "rust-analyzer"];
        };
      in {
        default = pkgs.rustPlatform.buildRustPackage {
          pname = cargoToml.package.name;
          version = cargoToml.package.version;
          src = self;
          cargoLock = {
            lockFile = ./Cargo.lock;
          };
          nativeBuildInputs = [rustToolchain];
          buildType = "release";
          doCheck = false;
          meta = with pkgs.lib; {
            description = cargoToml.package.description;
            license = licenses.gpl3Plus;
            platforms = platforms.darwin;
            mainProgram = "frisk";
          };
        };
        frisk = self.packages.${system}.default;
      }
    );
    devShells = forAllSystems (
      system: let
        pkgs = nixpkgsFor.${system};
        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          extensions = ["rust-src" "rust-analyzer"];
        };
      in {
        default = pkgs.mkShell {
          nativeBuildInputs = with pkgs; [
            rustToolchain
            cargo-watch
            cargo-edit
            rust-analyzer
            rustfmt
            clippy
          ];
          RUST_BACKTRACE = "1";
          RUST_LOG = "info";
          shellHook = ''
            echo "🚀 Frisk macOS Development Environment"
            echo "================================================"
            echo "Rust:    $(rustc --version)"
            echo "Cargo:   $(cargo --version)"
            echo ""
            echo "Commands:"
            echo "  cargo build              - Build debug binary"
            echo "  cargo build --release    - Build optimized binary"
            echo "  cargo run                - Run the launcher"
            echo "  cargo test               - Run tests"
            echo "  cargo clippy             - Run linter"
            echo "  cargo fmt                - Format code"
            echo "  cargo watch -x run       - Auto-rebuild on changes"
            echo ""
            echo "Nix commands:"
            echo "  nix build                - Build release binary"
            echo "  nix run                  - Run the launcher"
            echo ""
            echo "Binary locations:"
            echo "  Debug:   target/debug/frisk"
            echo "  Release: target/release/frisk"
            echo "================================================"
          '';
        };
      }
    );
    apps = forAllSystems (system: {
      default = {
        type = "app";
        program = "${self.packages.${system}.default}/bin/frisk";
      };
      frisk = self.apps.${system}.default;
    });
    formatter = forAllSystems (system: nixpkgsFor.${system}.nixpkgs-fmt);
  };
}
