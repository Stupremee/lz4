{ pkgs ? import <nixpkgs> { } }:
pkgs.mkShell {
  name = "rust-shell";
  nativeBuildInputs = [
    pkgs.rust-analyzer
    (pkgs.latest.rustChannels.stable.rust.override {
      extensions = [ "rust-src" ];
    })
  ];
}
