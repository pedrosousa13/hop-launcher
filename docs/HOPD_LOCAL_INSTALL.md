# hopd / hop-hotkeyd Local Install Guide

## Prerequisites

- Rust toolchain (`cargo`) installed
- `systemctl --user` available
- Optional: `socat` for quick socket checks

## Install / Update

From repository root:

```bash
cd apps/gnome-extension
npm run install:hopd:local
```

What this does:

1. Builds `hopd` in release mode.
2. Builds `hop-hotkeyd` in release mode.
3. Installs binaries to `~/.local/bin/hopd` and `~/.local/bin/hop-hotkeyd`.
4. Installs `~/.config/systemd/user/hopd.service` and `~/.config/systemd/user/hop-hotkeyd.service`.
5. Reloads user units and enables/starts both services.

## Useful options

```bash
# Preview commands without changing your system
cd apps/gnome-extension
./scripts/install-hopd-local.sh --dry-run

# Install unit without auto-starting service
./scripts/install-hopd-local.sh --no-enable
```

## Verify daemons

```bash
systemctl --user status hopd.service
systemctl --user status hop-hotkeyd.service
```

If `socat` is installed:

```bash
echo '{"id":"1","method":"health.ping"}' | socat - UNIX-CONNECT:${XDG_RUNTIME_DIR}/hopd.sock
```

Expected response contains `"ok":true`.

## Enable in GNOME extension

After installing and starting `hopd`, open Hop Launcher preferences and enable:

- `Main features` -> `hopd daemon`

`hopd` integration is off by default. Once enabled, utility-intent queries (weather/timezone/emoji routes) can be served by the daemon.
