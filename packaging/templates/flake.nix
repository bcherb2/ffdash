{
  description = "Nix flake for ffdash";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs = { self, nixpkgs }:
    let
      systems = [ "x86_64-linux" "aarch64-linux" "aarch64-darwin" "x86_64-darwin" ];
      forAllSystems = f: nixpkgs.lib.genAttrs systems (system:
        f system (import nixpkgs { inherit system; }));
    in {
      packages = forAllSystems (system: pkgs:
        let
          rustPlatform = pkgs.rustPlatform;
          version =
            let env = builtins.getEnv "FFDASH_VERSION";
            in if env != "" then env else "dev";
        in {
          ffdash = rustPlatform.buildRustPackage {
            pname = "ffdash";
            version = version;
            src = ./.;
            cargoLock = {
              lockFile = ./Cargo.lock;
            };
            nativeBuildInputs = [ pkgs.pkg-config ];
            buildInputs = [ pkgs.ffmpeg ];
            meta = with pkgs.lib; {
              description = "Fast VP9 video encoder with live TUI dashboard";
              homepage = "https://github.com/bcherb2/ffdash";
              license = licenses.mit;
              mainProgram = "ffdash";
              maintainers = [ ];
            };
          };
        });

      packages.x86_64-linux.default = self.packages.x86_64-linux.ffdash;
      apps = forAllSystems (system: pkgs: {
        ffdash = {
          type = "app";
          program = "${self.packages.${system}.ffdash}/bin/ffdash";
        };
        default = self.apps.${system}.ffdash;
      });
    };
}
