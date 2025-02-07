{ outputs }:
{
  lib,
  pkgs,
  config,
  ...
}:
{
  options.services.fscs-website-server =
    let
      t = lib.types;
    in
    {
      enable = lib.mkEnableOption "enable the fscs website server";

      package = lib.mkOption {
        description = "server package";
        type = t.package;
        default = outputs.packages.${pkgs.stdenv.system}.default;
      };

      environmentFile = lib.mkOption {
        description = "environment file to load into the systemd service";
        type = t.nullOr t.nonEmptyStr;
        default = null;
      };

      host = lib.mkOption {
        description = "host address to bind to";
        type = t.nonEmptyStr;
        default = "0.0.0.0";
      };

      port = lib.mkOption {
        description = "port to bind to";
        type = t.port;
        default = 8080;
      };

      content = lib.mkOption {
        description = "content folder to server. needs to contain public, hidden and protected subfolders";
        type = t.nonEmptyStr;
      };
      authUrl = lib.mkOption {
        description = "url for oauth authorization";
        type = t.nonEmptyStr;
      };
      tokenUrl = lib.mkOption {
        description = "url for getting tokens";
        type = t.nonEmptyStr;
      };
      userInfoUrl = lib.mkOption {
        description = "url for getting user info";
        type = t.nonEmptyStr;
      };
      extraFlags = lib.mkOption {
        description = "list of extra options to pass to the server";
        type = t.listOf t.nonEmptyStr;
        default = [ ];
      };
    };

  config =
    let
      cfg = config.services.fscs-website-server;
    in
    lib.mkIf cfg.enable {
      users.groups.fscs-website-server = { };
      users.users.fscs-website-server = {
        isSystemUser = true;
        group = "fscs-website-server";
      };

      services.postgresql = {
        enable = true;
        ensureDatabases = [ config.users.users.fscs-website-server.name ];
        ensureUsers = lib.singleton {
          name = config.users.users.fscs-website-server.name;
          ensureDBOwnership = true;
        };
      };

      systemd.services.fscs-website-server =
        let
          argSet = {
            inherit (cfg) host port;
            database-url = "postgresql:///${config.users.users.fscs-website-server.name}?port=${toString config.services.postgresql.settings.port}";
            content-dir = cfg.content;
            auth-url = cfg.authUrl;
            token-url = cfg.tokenUrl;
            user-info = cfg.userInfoUrl;
          };

          args = lib.escapeShellArgs ((lib.cli.toGNUCommandLine { } argSet) ++ cfg.extraFlags);
        in
        {
          after = [ "network.target" ];
          wantedBy = [ "multi-user.target" ];
          serviceConfig = {
            EnvironmentFile = cfg.environmentFile;
            ExecStart = "${lib.getExe cfg.package} ${args}";
            Type = "exec";
            User = config.users.users.fscs-website-server.name;
            Restart = "always";
            RestartSec = 5;
            CapabilityBoundingSet = [ "" ];
            DeviceAllow = [ "" ];
            DevicePolicy = "closed";
            LockPersonality = true;
            MemoryDenyWriteExecute = true;
            NoNewPrivileges = true;
            PrivateDevices = true;
            PrivateTmp = true;
            PrivateUsers = true;
            ProcSubset = "pid";
            ProtectClock = true;
            ProtectControlGroups = true;
            ProtectHome = true;
            ProtectHostname = true;
            ProtectKernelLogs = true;
            ProtectKernelModules = true;
            ProtectKernelTunables = true;
            ProtectProc = "noaccess";
            ProtectSystem = "strict";
            RemoveIPC = true;
            RestrictAddressFamilies = [
              "AF_INET"
              "AF_INET6"
              "AF_UNIX"
            ];
            RestrictNamespaces = true;
            RestrictRealtime = true;
            RestrictSUIDSGID = true;
            SystemCallArchitectures = "native";
            SystemCallFilter = [
              "@system-service"
              "~@privileged"
            ];
            UMask = "0077";
          };
        };
    };
}
