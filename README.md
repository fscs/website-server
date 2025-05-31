# Website Server

API Docs can be found under [fscs.hhu.de/api/docs](https://fscs.hhu.de/api/docs).

## Usage

### Serving Content

A folder to be served over HTTP must be specified using the `--content-dir` flag.

Content is separated into 3 Classes:
- Public 
    - served to everyone, who isnt in one of the other classes
- Hidden
    - intended for content that is public but should not be searchable
- Protected
    - content that requires login to be accessed

The folder should be structured like this.
```
folder/
  public/
  hidden/
  protected/
```

Under the Hood, different users just get served these different sub folders. There is no way to restrict access to a certain url, if you want this only place the file
in the folder you want it to be visible from

### ICalendar Mirroring

The server is capable of mirroring an ICalendar based calendar as JSON. The calendar is cached and only refreshed every 4 hours. This has the advantage that fetching a calendar (from e.g. nextcloud) can sometimes be a bit slow and JSON is generally a bit easier to parse than ICalendars. Only future events are included.

Calendars can be specified using the `--calendar` flag and will be made available under `/api/calendar/<name>`. Can be specified multiple times.

Example: `--calendar events=https://dav.domain.tld/calendar/events.ical?export`

### Access Control

The server supports OAuth2 based Authorization. 

Set it up using the following command line flags:

- `--oauth-source-name` Used to differentiate between different OAuth Providers (although this is unimplemented at the moment)
- `--auth-url` Url to authorize against
- `--token-url` Url to obtain tokens from
- `--user-info` Url to obtain user info from

Additionally, specify the following environment variables
- `CLIENT_SECRET` OAuth client secret
- `CLIENT_ID` OAuth client id
- `SIGNING_KEY` Signing Key for signing cookies

Now on to giving your Users capabilities. These are based on OAuth Groups and can be specified using the `--group` flag. Can be specified multiple times.

Example `--group siko=ManageSitzungen,ManageÄntrage`

#### Available Capabilities
- `Admin` 
    - Includes all other capabilities
- `ManageSitzungen`
    - Create, edit and delete sitzungen
    - Create, edit and delete tops and assign anträge to them
    - Create, edit and delete legislative periods
- `CreateAntrag`
    - Create anträge
    - Edit ones own anträge
    - Create and delete attachments on ones own anträge
- `ManageAnträge`
    - Edit and delete anträge created by others
    - Add and delete antrag attachments
    - Delete Anträge
- `ManagePersons`
    - Create and delete Roles
    - View extended Info about Persons
    - Create, edit and delete Persons
    - Create, delete and view Abmeldungen
- `ViewHidden`
    - view content in the `hidden` folder
- `ViewProtected`
    - view content in the `protected` folder


## NixOS Module

A NixOS Module is provided.

Example flake.nix

```nix
{
    inputs.nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    inputs.fscs-website-server.url = "github:fscs/website-server";
    
    outputs = {self, nixpkgs, fscs-website-server }: {
        nixosConfigurations.teefax = nixpkgs.lib.nixosSystem {
            modules = [
                ./configuration.nix
                fscs-website-server.nixosModules.fscs-website-server
            ];
        };
    };
}
```

### NixOS Options
```nix
services.fscs-website-server = {
    enable = true;

    # optional
    package = <website-server-package>;
    
    # Environment File passed to the systemd service. Specify your OAuth
    # configuration here!
    environmentFile = "path to environment file";
    
    # path to the data dir. stuff like uploaded attachments will be placed here.
    # 
    # optional, defaults to /var/lib/fscs-website-server
    dataDir = null;
    
    # calendars to mirror, the name and url are passed as key-value pairs
    #
    # optional
    calendars = {
        events = "https://dav.domain.tld/calendar/events.ical?export";
    };
    
    # groups specified as pairs of group names and lists of capability names
    # 
    # optional
    groups = {
        siko = [ "ManageSitzungen" "ManageÄntrage" ];
    };
    
    # extra command line flags to pass to the server, specified as key-value pairs
    # 
    # see below for reference
    settings = {
        port = 9000;
        oauth-source-name = "authentik";
    };
};
```

## Commandline Options

```
Usage: fscs-website-backend [OPTIONS] --content-dir <CONTENT_DIR> --oauth-source-name <OAUTH_SOURCE_NAME> --auth-url <AUTH_URL> --token-url <TOKEN_URL> --user-info <USER_INFO> --data-dir <DATA_DIR>

Options:
  -p, --port <PORT>
          Port of the Application [default: 8080]
      --host <HOST>
          The Host Interface [default: 127.0.0.1]
      --content-dir <CONTENT_DIR>
          Directory to serve. Needs to contain public, hidden and private subdirs
      --log-level <LOG_LEVEL>
          Log Level [default: Info]
  -d, --database-url <DATABASE_URL>
          Postgres Database Url to connect to
  -o, --oauth-source-name <OAUTH_SOURCE_NAME>
          Oauth Source Name
  -a, --auth-url <AUTH_URL>
          Oauth Url to authorize against
      --group <GROUPS>
          Specifiy a group and grant it capabilities.. Parameter should be formatted like 'GroupName=CapName[,CapName]'
  -t, --token-url <TOKEN_URL>
          Oauth Url to get tokens from
  -u, --user-info <USER_INFO>
          Oauth Url to get user info from
  -j, --workers <WORKERS>
          How many web workers to spawn. Default is the number of CPU cores
      --cors-allowed-origin <CORS_ALLOWED_ORIGIN>
          Cors origin to allow request from. Can be specified multiple times
  -C, --calendar <CALENDARS>
          Define an ical calender to fetch, formatted like name=calendar-url. The calendar will be available under /api/calendar/<name>. Can be specified multiple times
      --max-file-size <MAX_FILE_SIZE>
          Define the max file size for uploads in bytes [default: 10485760]
      --data-dir <DATA_DIR>
          Define the datadir for the uploads
  -h, --help
          Print help
```


