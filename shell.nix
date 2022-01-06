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
in pkgs.mkShell {
  buildInputs = with pkgs;
    [
      niv.niv
      git

      # rust tools
      cargo
      rustPackages.clippy
      rustc
      rustfmt
      libiconv
    ] ++ pkgs.lib.optionals pkgs.stdenv.isDarwin macosDeps
    ++ pkgs.lib.optionals pkgs.stdenv.isLinux linuxDeps;

  NIXOS_GLIBC_PATH = if pkgs.stdenv.isLinux then pkgs.glibc else "";
}
