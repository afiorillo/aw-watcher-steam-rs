Environment notes:

- Running on a NixOS machine. The dev environment is defined by the Nix flake
  (`flake.nix`); enter it with `nix develop` (or rely on direnv, which uses
  `use flake`). It provides the Rust toolchain plus `openssl`/`pkg-config` needed
  to build the `steam-api` feature.
- Run commands inside the dev shell, e.g. `nix develop -c cargo test`,
  `nix develop -c cargo clippy --all-targets`.
