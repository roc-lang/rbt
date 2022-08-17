{ ... }:
let
  sources = import ./nix/sources.nix;
  pkgs = import sources.nixpkgs { };
  niv = import sources.niv { };
  macosDeps = [ pkgs.darwin.apple_sdk.frameworks.CoreServices ];
  linuxDeps = [
    # only necessary until surgical linking is in Roc
    pkgs.glibc
  ];

  roc = pkgs.callPackage sources.roc {
    cargoSha256 = "sha256-jFN0Ne/0wCCZl/oNmZE5/Sw5l+qNxShI3xlP4ikFMlw=";
  };
in pkgs.mkShell {
  buildInputs = with pkgs;
    [
      niv.niv
      git

      # roc tools
      roc

      (pkgs.writeShellScriptBin "sync-roc-std" ''
        set -euo pipefail

        ROOT="$(${pkgs.git}/bin/git rev-parse --show-toplevel)"
        rm -rf "$ROOT/vendor/roc_std"
        mkdir -p "$ROOT/vendor/roc_std"
        ${pkgs.rsync}/bin/rsync --chmod 0644 -r ${sources.roc}/crates/roc_std/ "$ROOT/vendor/roc_std"
      '')

      (pkgs.writeShellScriptBin "sync-glue" ''
        set -euo pipefail

        ROOT="$(${pkgs.git}/bin/git rev-parse --show-toplevel)"
        ${roc}/bin/roc glue "$ROOT/main.roc" "$ROOT/src/glue.rs"
        ${rustfmt}/bin/rustfmt "$ROOT/src/glue.rs"
      '')

      # rust tools
      cargo
      cargo-edit
      rustPackages.clippy
      rustc
      rustfmt
      libiconv
    ] ++ pkgs.lib.optionals pkgs.stdenv.isDarwin macosDeps
    ++ pkgs.lib.optionals pkgs.stdenv.isLinux linuxDeps;

  NIXOS_GLIBC_PATH = if pkgs.stdenv.isLinux then pkgs.glibc else "";
}
