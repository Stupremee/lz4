{ pkgs ? import <nixpkgs> { } }:
pkgs.mkShell {
  name = "rust-shell";
  nativeBuildInputs = [
    pkgs.rust-analyzer
    pkgs.python3
    (pkgs.latest.rustChannels.nightly.rust.override {
      extensions = [ "rust-src" ];
    })
  ];
}
