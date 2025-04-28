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

      trivialTypes = t.either t.nonEmptyStr (t.either t.bool t.number);
      settingsType = t.attrsOf (t.either trivialTypes (t.listOf trivialTypes));
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

      settings = lib.mkOption {
        description = "settings, passed as commandline arguments";
        type = t.submodule {
          freeformType = settingsType;

          options = {
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
          };
        };
        default = { };
      };

      dataDir = lib.mkOption {
        description = "directory to store uploaded files";
        type = t.nonEmptyStr;
        default = "/var/lib/fscs-website-server";
      };

      calendars = lib.mkOption {
        description = "ical calendars to make available under /api/calendar/<name>";
        type = t.attrsOf t.nonEmptyStr;
        default = { };
      };

      groups = lib.mkOption {
        description = "grant permissions to certain oauth groups";
        type = t.attrsOf (t.listOf t.nonEmptyStr);
        default = { };
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

      services.fscs-website-server.settings = {
        data-dir = cfg.dataDir;
        database-url = "postgresql:///${config.users.users.fscs-website-server.name}?port=${toString config.services.postgresql.settings.port}";

        calendar = lib.mapAttrsToList (name: url: "${name}=${url}") cfg.calendars;

        group = lib.mapAttrsToList (name: caps: "${name}=${lib.concatStringsSep "," caps}") cfg.groups;
      };

      systemd.services.fscs-website-server = {
        after = [ "network.target" ];
        wantedBy = [ "multi-user.target" ];
        serviceConfig = {
          EnvironmentFile = cfg.environmentFile;
          ExecStart = "${lib.getExe cfg.package} ${lib.cli.toGNUCommandLineShell { } cfg.settings}";
          Type = "exec";
          User = config.users.users.fscs-website-server.name;
          Restart = "always";
          RestartSec = 5;
          StateDirectory = cfg.dataDir;
          LimitNOFILE = "8192";
          AmbientCapabilities = [ "CAP_NET_BIND_SERVICE" ];
          CapabilityBoundingSet = [ "CAP_NET_BIND_SERVICE" ];
          DeviceAllow = [ "" ];
          DevicePolicy = "closed";
          LockPersonality = true;
          MemoryDenyWriteExecute = true;
          NoNewPrivileges = true;
          PrivateDevices = true;
          PrivateTmp = true;
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
