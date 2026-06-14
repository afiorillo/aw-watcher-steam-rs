# Example home-manager configuration that runs a full ActivityWatch stack as
# systemd *user* services — the server, the official AFK/window watchers, and
# this Steam watcher — so your whole time-tracking setup is declarative.
#
# home-manager has no built-in ActivityWatch module, so the server and official
# watchers are defined here as plain user services. Remove that block if you
# already run ActivityWatch another way (e.g. `aw-qt`).
#
# Usage
# -----
# 1. Add the flake input to your flake.nix:
#
#      inputs.aw-watcher-steam-rs.url = "github:afiorillo/aw-watcher-steam-rs";
#
# 2. Make `inputs` available to your home-manager modules, e.g.:
#
#      home-manager.extraSpecialArgs = { inherit inputs; };
#
# 3. Import this file from your home configuration:
#
#      imports = [ ./path/to/activitywatch.nix ];
#
#    (or just copy the bits you want into your own home.nix).

{ pkgs, inputs, ... }:

{
  imports = [ inputs.aw-watcher-steam-rs.homeManagerModules.default ];

  # --- This Steam watcher ----------------------------------------------------
  services.aw-watcher-steam-rs = {
    enable = true;
    # logLevel = "debug"; # optional; sets RUST_LOG

    # To also track friends' games (and Proton titles via the API), generate a
    # config with `aw-watcher-steam-rs config --generate` and fill in the
    # [steam_api] section — see the project README.
  };

  # --- ActivityWatch server + official watchers ------------------------------
  # Delete this whole section if you run ActivityWatch some other way.
  home.packages = [ pkgs.activitywatch ];

  systemd.user.services.aw-server = {
    Unit.Description = "ActivityWatch server";
    Service = {
      ExecStart = "${pkgs.activitywatch}/bin/aw-server";
      Restart = "on-failure";
      RestartSec = 5;
    };
    Install.WantedBy = [ "default.target" ];
  };

  systemd.user.services.aw-watcher-afk = {
    Unit = {
      Description = "ActivityWatch AFK watcher";
      After = [ "aw-server.service" ];
    };
    Service = {
      ExecStart = "${pkgs.activitywatch}/bin/aw-watcher-afk";
      Restart = "on-failure";
      RestartSec = 5;
    };
    Install.WantedBy = [ "default.target" ];
  };

  systemd.user.services.aw-watcher-window = {
    Unit = {
      Description = "ActivityWatch window watcher";
      After = [ "aw-server.service" ];
    };
    Service = {
      ExecStart = "${pkgs.activitywatch}/bin/aw-watcher-window";
      Restart = "on-failure";
      RestartSec = 5;
    };
    Install.WantedBy = [ "default.target" ];
  };
}
