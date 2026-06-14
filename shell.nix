# Development shell for forge-agent.
#
#   nix-shell        # drops you into a shell with the toolchain below
#   cargo test --workspace
#
# Provides the Rust toolchain plus a C compiler/linker (`cc`), which Cargo needs to link
# even pure-Rust crates (serde_json's build scripts, the final binary).
{ pkgs ? import <nixpkgs> { } }:

pkgs.mkShell {
  packages = [
    pkgs.rustc
    pkgs.cargo
    pkgs.gcc # provides cc / the linker
    pkgs.rustfmt
    pkgs.clippy
    pkgs.git
  ];

  # `pkg-config` (nativeBuildInput) locates `openssl` (buildInput) so the `openssl-sys`
  # crate — pulled in transitively by reqwest's `native-tls` feature — can build and link.
  nativeBuildInputs = [ pkgs.pkg-config ];
  buildInputs = [ pkgs.openssl ];

  # Help Cargo/rustc find the C compiler explicitly.
  CC = "cc";
}
