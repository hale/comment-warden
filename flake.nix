{
  description = "comment-warden: an AST comment warden that strips comments which stop no edit";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-26.05";

    crane.url = "github:ipetkov/crane";

    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.rust-analyzer-src.follows = "";
    };

    flake-utils.url = "github:numtide/flake-utils";

    advisory-db = {
      url = "github:rustsec/advisory-db";
      flake = false;
    };
  };

  outputs = { self, nixpkgs, crane, fenix, flake-utils, advisory-db, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs { inherit system; };
        lib = pkgs.lib;

        craneLib = (crane.mkLib pkgs).overrideToolchain
          (fenix.packages.${system}.stable.toolchain);

        # TRIPWIRE: the fixture tests `include_str!` non-.rs files under
        # tests/fixtures; cleanCargoSource drops those, so the build fails to
        # compile the tests unless they are added back to the filtered source.
        fixtureFilter = path: _type:
          builtins.match ".*/tests/fixtures/.*" path != null;
        srcFilter = path: type:
          (fixtureFilter path type) || (craneLib.filterCargoSources path type);
        src = lib.cleanSourceWith {
          src = ./.;
          filter = srcFilter;
          name = "source";
        };

        commonArgs = {
          inherit src;
          strictDeps = true;

          # tree-sitter grammar crates compile bundled C through the `cc` crate
          nativeBuildInputs = [ pkgs.stdenv.cc ];

          buildInputs = lib.optionals pkgs.stdenv.isDarwin [
            pkgs.libiconv
          ];
        };

        cargoArtifacts = craneLib.buildDepsOnly commonArgs;

        comment-warden = craneLib.buildPackage (commonArgs // {
          inherit cargoArtifacts;
        });
      in
      {
        packages.default = comment-warden;

        apps.default = flake-utils.lib.mkApp { drv = comment-warden; };

        checks = {
          inherit comment-warden;

          comment-warden-clippy = craneLib.cargoClippy (commonArgs // {
            inherit cargoArtifacts;
            cargoClippyExtraArgs = "--all-targets -- --deny warnings";
          });

          comment-warden-fmt = craneLib.cargoFmt { inherit src; };

          comment-warden-audit = craneLib.cargoAudit {
            inherit src advisory-db;
          };

          comment-warden-nextest = craneLib.cargoNextest (commonArgs // {
            inherit cargoArtifacts;
            partitions = 1;
            partitionType = "count";
          });

          comment-warden-self-check =
            pkgs.runCommand "comment-warden-self-check"
              { nativeBuildInputs = [ comment-warden ]; }
              ''
                comment-warden check ${src}/src
                touch $out
              '';
        };

        devShells.default = craneLib.devShell {
          packages = [
            pkgs.rust-analyzer
            pkgs.bacon
            pkgs.cargo-nextest
            pkgs.cargo-machete
            pkgs.taplo
          ];

          # TRIPWIRE: keyed on the repo root, not $PWD — a sibling checkout that
          # shares one target dir links this crate against another tree's
          # artifacts, so builds pass or fail on code that isn't in front of you.
          shellHook = ''
            export CARGO_TARGET_DIR="$HOME/.cargo-target/$(basename "$(${pkgs.git}/bin/git rev-parse --show-toplevel 2>/dev/null || echo "$PWD")")"
            mkdir -p "$CARGO_TARGET_DIR"
          '';
        };
      });
}
