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

      # roc tools
      (pkgs.callPackage sources.roc {
        cargoSha256 = "sha256-jFN0Ne/0wCCZl/oNmZE5/Sw5l+qNxShI3xlP4ikFMlw=";
      })

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
