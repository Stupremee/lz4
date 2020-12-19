{ pkgs ? import <nixpkgs> { } }:
pkgs.mkShell {
  name = "rust-shell";
  nativeBuildInputs = with pkgs; [
    lz4
    rust-analyzer
    python3
    hexyl
    ((rustChannelOf { rustToolchain = ./rust-toolchain; }).rust.override {
      extensions = [ "rust-src" ];
    })
  ];
}
