{
  lib,
  config,
  ...
}: {
  options.services.fscswebsite = let
    t = lib.types;
  in {
    enable = lib.mkEnableOption "enable the fscs website";

    package = lib.mkOption {
      description = "server package";
      type = t.package;
    };

    user = {
      name = lib.mkOption {
        description = "user to run the server as (be careful this also affects the database name)";
        type = t.str;
        default = "fscs-website";
      };
      create = lib.mkOption {
        description = "create the user";
        type = t.bool;
        default = true;
      };
    };

    environmentFile = lib.mkOption {
      description = "environment file to load into the systemd service";
      type = t.nullOr t.path;
      default = null;
    };

    content = lib.mkOption {
      description = "content folder to server. needs to contain public, hidden and protected subfolders";
      type = t.pathInStore;
    };
    authUrl = lib.mkOption {
      description = "url for oauth authorization";
      type = t.str;
    };
    tokenUrl = lib.mkOption {
      description = "url for getting tokens";
      type = t.str;
    };
    userInfoUrl = lib.mkOption {
      description = "url for getting user info";
      type = t.str;
    };
    extraFlags = lib.mkOption {
      description = "list of extra options to pass to the server";
      type = t.listOf t.str;
      default = [];
    };
  };

  config = let
    cfg = config.services.fscswebsite;
  in
    lib.mkIf cfg.enable {
      users.users = lib.mkIf cfg.user.create {
        ${cfg.user.name} = {
          isNormalUser = true;
        };
      };

      services.postgresql = {
        enable = true;
        ensureDatabases = lib.singleton cfg.user.name;
        ensureUsers = lib.singleton {
          name = cfg.user.name;
          ensureDBOwnership = true;
        };
      };

      systemd.services.fscs-website-serve = let
        argSet = {
          database-url = "postgresql://localhost:${config.services.postgresql.settings.port}/${cfg.user.name}";
          content-dir = cfg.content;
          auth-url = cfg.authUrl;
          token-url = cfg.tokenUrl;
          user-into = cfg.userInfoUrl;
        };

        args = (lib.toGNUCommandLineShell argSet) ++ cfg.extraFlags;
      in {
        description = "Serve FSCS website";
        after = ["network.target"];
        serviceConfig = {
          EnvironmentFile = lib.mkIf cfg.environmentFile != null cfg.environmentFile;
          Type = "exec";
          User = cfg.user.name;
          ExecStart = "${lib.getExe cfg.package} ${args}";
          Restart = "always";
          RestartSec = 5;
          StandardOutput = "append:/var/log/fscs-website/log.log";
          StandardError = "append:/var/log/fscs-website/log.log";
        };
        wantedBy = ["multi-user.target"];
      };
    };
}
