{
  description = "datom — Datom text serialization and deserialization";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-build = {
      url = "github:LiGoldragon/rust-build";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, flake-utils, rust-build }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs { inherit system; };
        rust = rust-build.lib.${system}.fromPkgs pkgs;
        inherit (rust) craneLib toolchain;
        src = rust.cleanSource {
          root = ./.;
        };
        commonArguments = { inherit src; strictDeps = true; };
        cargoArtifacts = craneLib.buildDepsOnly commonArguments;
      in
      {
        packages.default = craneLib.buildPackage (commonArguments // { inherit cargoArtifacts; });
        checks = {
          build = craneLib.cargoBuild (commonArguments // { inherit cargoArtifacts; });
          test = craneLib.cargoTest (commonArguments // { inherit cargoArtifacts; });
          no-production-free-functions = pkgs.runCommand "datom-no-production-free-functions" { } ''
            if grep -R -n -E '^(pub(\([^)]*\))? )?fn ' ${src}/src; then
              echo "production Rust must not use module-level free functions" >&2
              exit 1
            fi
            touch $out
          '';
          no-production-inherent-methods = pkgs.runCommand "datom-no-production-inherent-methods" { } ''
            if grep -R -n -E '^[[:space:]]*impl[[:space:]]+[[:alpha:]_][[:alnum:]_:<>]*[[:space:]]*\{' ${src}/src; then
              echo "production Rust must home behavior in traits" >&2
              exit 1
            fi
            touch $out
          '';
          no-zst-behavior = pkgs.runCommand "datom-no-zst-behavior" { } ''
            if grep -R -n -E '^[[:space:]]*(pub[[:space:]]+)?struct[[:space:]]+[[:alpha:]_][[:alnum:]_]*[[:space:]]*;' ${src}/src; then
              echo "behavioral Rust nouns must carry data" >&2
              exit 1
            fi
            touch $out
          '';
          no-forbidden-vocabulary = pkgs.runCommand "datom-no-forbidden-vocabulary" { } ''
            if grep -R -n -i -E 'encode|decode|codec|transcode' ${src}/src; then
              echo "Datom names must use the ruled form vocabulary" >&2
              exit 1
            fi
            touch $out
          '';
          doc = craneLib.cargoDoc (commonArguments // {
            inherit cargoArtifacts;
            RUSTDOCFLAGS = "-D warnings";
          });
          fmt = craneLib.cargoFmt { inherit src; };
          clippy = craneLib.cargoClippy (commonArguments // {
            inherit cargoArtifacts;
            cargoClippyExtraArgs = "--all-targets -- -D warnings";
          });
        };
        devShells.default = pkgs.mkShell {
          name = "datom";
          packages = [ pkgs.jujutsu toolchain ];
        };
      });
}
