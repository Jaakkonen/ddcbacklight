{
  description = "DDC Backlight - CLI tool to control monitor brightness using DDC/CI protocol";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs =
    {
      self,
      nixpkgs,
      rust-overlay,
      flake-utils,
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };

        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          extensions = [
            "rust-src"
            "rust-analyzer"
          ];
        };

        buildInputs = with pkgs; [
          pkg-config
          libudev-zero
        ];

        nativeBuildInputs = with pkgs; [
          rustToolchain
          pkg-config
        ];
      in
      {
        packages = {
          default = self.packages.${system}.ddcbacklight;

          ddcbacklight = pkgs.rustPlatform.buildRustPackage {
            pname = "ddcbacklight";
            version = "0.1.0";

            src = ./.;

            cargoLock = {
              lockFile = ./Cargo.lock;
            };

            inherit nativeBuildInputs buildInputs;

            meta = with pkgs.lib; {
              description = "CLI tool to control monitor brightness using DDC/CI protocol";
              longDescription = ''
                A command-line tool for controlling external monitor brightness using the DDC/CI protocol.
                Supports both Intel and AMD GPUs with automatic I2C device detection.

                Requires the i2c-dev kernel module to be loaded:
                  sudo modprobe i2c-dev

                Usage:
                  ddcbacklight get-brightness
                  ddcbacklight set-brightness 75
                  ddcbacklight --i2c-path /dev/i2c-N set-brightness 50
              '';
              homepage = "https://github.com/jaakko/ddcbacklight";
              license = licenses.mit;
              maintainers = [ ];
              platforms = platforms.linux;
            };
          };
        };

        devShells.default = pkgs.mkShell {
          inherit buildInputs;
          nativeBuildInputs =
            nativeBuildInputs
            ++ (with pkgs; [
              cargo-watch
              cargo-edit
              rust-analyzer
            ]);

          shellHook = ''
            echo "🦀 Rust development environment for ddcbacklight"
            echo "Available commands:"
            echo "  cargo build    - Build the project"
            echo "  cargo run      - Run the project"
            echo "  cargo test     - Run tests"
            echo "  cargo watch    - Watch for changes and rebuild"
            echo ""
            echo "📋 Kernel module requirements:"
            echo "  sudo modprobe i2c-dev    - Enable I2C device access"
            echo ""
            echo "🔧 Usage examples:"
            echo "  cargo run get-brightness"
            echo "  cargo run set-brightness 50"
            echo "  cargo run --i2c-path /dev/i2c-7 get-brightness"
          '';

          RUST_SRC_PATH = "${rustToolchain}/lib/rustlib/src/rust/library";
        };
      }
    );
}
