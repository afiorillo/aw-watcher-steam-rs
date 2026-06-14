{
  description = "aw-watcher-steam-rs — an ActivityWatch watcher for Steam activity";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs =
    {
      self,
      nixpkgs,
      flake-utils,
    }:
    # Per-system outputs (packages, devShells, apps).
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = import nixpkgs { inherit system; };

        # Reproducible build straight from the locked Cargo.lock. This is the
        # standard, well-supported way to package a Rust app with Nix; for heavier
        # caching needs `crane` is a common alternative, but buildRustPackage is
        # plenty for an app this size.
        aw-watcher-steam-rs = pkgs.rustPlatform.buildRustPackage {
          pname = "aw-watcher-steam-rs";
          version = "0.1.0";

          src = self;
          cargoLock.lockFile = ./Cargo.lock;

          # `pkg-config` locates `openssl`, needed by reqwest's native-tls (pulled
          # in by the default `steam-api` feature).
          nativeBuildInputs = [ pkgs.pkg-config ];
          buildInputs = [ pkgs.openssl ];

          meta = with pkgs.lib; {
            description = "An ActivityWatch watcher for Steam activity";
            homepage = "https://github.com/afiorillo/aw-watcher-steam-rs";
            license = licenses.agpl3Plus;
            mainProgram = "aw-watcher-steam-rs";
            platforms = platforms.unix ++ platforms.windows;
          };
        };
      in
      {
        packages = {
          default = aw-watcher-steam-rs;
          aw-watcher-steam-rs = aw-watcher-steam-rs;
        };

        apps.default = flake-utils.lib.mkApp { drv = aw-watcher-steam-rs; };

        devShells.default = pkgs.mkShell {
          # Pull in the package's build inputs (openssl, pkg-config, …) so the
          # dev shell can build it, plus the interactive toolchain.
          inputsFrom = [ aw-watcher-steam-rs ];
          packages = [
            pkgs.rustc
            pkgs.cargo
            pkgs.clippy
            pkgs.rustfmt
            pkgs.gcc
            pkgs.git
          ];
          env.CC = "cc";
        };

        formatter = pkgs.nixfmt;
      }
    )
    # System-independent outputs: overlay + modules.
    // {
      overlays.default = final: prev: {
        aw-watcher-steam-rs = self.packages.${final.system}.default;
      };

      # home-manager module — the recommended way to run the watcher, as a
      # per-user systemd service (it needs to see *your* processes and Steam, and
      # talk to *your* aw-server).
      homeManagerModules.default =
        {
          config,
          lib,
          pkgs,
          ...
        }:
        let
          cfg = config.services.aw-watcher-steam-rs;
        in
        {
          options.services.aw-watcher-steam-rs = {
            enable = lib.mkEnableOption "the ActivityWatch Steam watcher (aw-watcher-steam-rs)";
            package = lib.mkOption {
              type = lib.types.package;
              default = self.packages.${pkgs.system}.default;
              defaultText = lib.literalExpression "aw-watcher-steam-rs.packages.\${system}.default";
              description = "The aw-watcher-steam-rs package to run.";
            };
            logLevel = lib.mkOption {
              type = lib.types.nullOr lib.types.str;
              default = null;
              example = "debug";
              description = "Value for RUST_LOG (overrides the config file's log_level). Null leaves it unset.";
            };
          };

          config = lib.mkIf cfg.enable {
            systemd.user.services.aw-watcher-steam-rs = {
              Unit = {
                Description = "ActivityWatch Steam watcher (aw-watcher-steam-rs)";
                # Order after the network and (if present) the aw-server user
                # service. Ordering against a unit that isn't running is a no-op,
                # so this is safe even if you run ActivityWatch some other way.
                After = [
                  "network-online.target"
                  "aw-server.service"
                ];
                Wants = [ "network-online.target" ];
              };
              Service = {
                ExecStart = lib.getExe cfg.package;
                Restart = "on-failure";
                RestartSec = 10;
                Environment = lib.optional (cfg.logLevel != null) "RUST_LOG=${cfg.logLevel}";
              };
              Install.WantedBy = [ "default.target" ];
            };
          };
        };

      # NixOS module — installs the package system-wide and (optionally) defines
      # the same per-user systemd service for all users. For single-user setups
      # prefer the home-manager module above.
      nixosModules.default =
        {
          config,
          lib,
          pkgs,
          ...
        }:
        let
          cfg = config.services.aw-watcher-steam-rs;
        in
        {
          options.services.aw-watcher-steam-rs = {
            enable = lib.mkEnableOption "the ActivityWatch Steam watcher as a per-user service";
            package = lib.mkOption {
              type = lib.types.package;
              default = self.packages.${pkgs.system}.default;
              defaultText = lib.literalExpression "aw-watcher-steam-rs.packages.\${system}.default";
              description = "The aw-watcher-steam-rs package to run.";
            };
            logLevel = lib.mkOption {
              type = lib.types.nullOr lib.types.str;
              default = null;
              example = "debug";
              description = "Value for RUST_LOG (overrides the config file's log_level).";
            };
          };

          config = lib.mkIf cfg.enable {
            environment.systemPackages = [ cfg.package ];
            systemd.user.services.aw-watcher-steam-rs = {
              description = "ActivityWatch Steam watcher (aw-watcher-steam-rs)";
              after = [ "network-online.target" ];
              wants = [ "network-online.target" ];
              wantedBy = [ "default.target" ];
              serviceConfig = {
                ExecStart = lib.getExe cfg.package;
                Restart = "on-failure";
                RestartSec = 10;
              };
              environment = lib.mkIf (cfg.logLevel != null) { RUST_LOG = cfg.logLevel; };
            };
          };
        };
    };
}
