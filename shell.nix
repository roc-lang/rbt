{ ... }:
let
  sources = import ./nix/sources.nix;
  pkgs = import sources.nixpkgs { };
  niv = import sources.niv { };
  macosDeps = [ pkgs.darwin.apple_sdk.frameworks.CoreServices ];
in pkgs.mkShell {
  buildInputs = with pkgs;
    [
      niv.niv
      git

      # rust tools
      cargo
      cargo-watch
      rustPackages.clippy
      rustc
      rustfmt
      libiconv

      # only necessary until surgical linking is in Roc
      glibc
    ] ++ pkgs.lib.optionals pkgs.stdenv.isDarwin macosDeps;

  NIXOS_GLIBC_PATH = pkgs.glibc;
}
