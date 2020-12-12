{ pkgs ? import <nixpkgs> { } }:
pkgs.mkShell {
  name = "rust-shell";
  nativeBuildInputs = with pkgs; [
    lz4
    rust-analyzer
    python3
    hexyl
    (latest.rustChannels.nightly.rust.override { extensions = [ "rust-src" ]; })
  ];
}
