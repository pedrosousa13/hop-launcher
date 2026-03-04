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
6. On GNOME sessions, attempts to configure a custom keybinding for `hop-hotkeyd trigger`.
7. On KDE sessions, prints KGlobalAccel helper guidance and probe command hints.

## Useful options

```bash
# Preview commands without changing your system
cd apps/gnome-extension
./scripts/install-hopd-local.sh --dry-run

# Install unit without auto-starting service
./scripts/install-hopd-local.sh --no-enable

# Override default shortcut capture key
HOP_LAUNCHER_SHORTCUT='<Super>space' ./scripts/install-hopd-local.sh
```

## Verify daemons

```bash
systemctl --user status hopd.service
systemctl --user status hop-hotkeyd.service
```

Shortcut setup helper:

```bash
~/.local/bin/hop-hotkeyd setup-shortcut
~/.local/bin/hop-hotkeyd setup-shortcut --dry-run
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

For packaged distribution (`.deb`/APT and other Linux channels), see:

- `docs/LINUX_PACKAGING.md`
