# Release Checklist

This is the maintainer checklist for shipping Hop Launcher artifacts.

## 1) Pre-release verification

Run the full suite from repository root:

```bash
cd apps/gnome-extension && npm test
cd ../../crates/hopd && cargo test
cd ../hop-hotkeyd && cargo test
cd ../../apps/gtk-launcher && cargo test
cargo test --features gtk_ui
```

## 2) Prepare release metadata

1. Confirm changelog/release notes draft.
2. Confirm user-facing docs are current:
   - `README.md`
   - `docs/HOPD_LOCAL_INSTALL.md`
   - `docs/HOTKEYD_STATUS_SCHEMA.md`
   - `docs/LINUX_PACKAGING.md`
3. Ensure branch is merged to `main`.

## 3) GitHub release artifacts

The release workflow currently builds:

- GNOME extension zip (`apps/gnome-extension/dist/*.zip`)
- `hopd-linux-x86_64.tar.gz`
- `hop-hotkeyd-linux-x86_64.tar.gz`

Trigger options:

- automatic on push to `main`
- manual via `workflow_dispatch`

## 4) Post-release smoke checks

On a clean Linux user account:

1. Install local services (`npm run install:hopd:local`) or use release binaries.
2. Verify service status:
   - `systemctl --user status hopd.service`
   - `systemctl --user status hop-hotkeyd.service`
3. Verify shortcut and bridge behavior:
   - `~/.local/bin/hop-hotkeyd status`
   - `~/.local/bin/hop-hotkeyd doctor --strict`
   - `~/.local/bin/hop-hotkeyd setup-shortcut --dry-run`

## 5) Linux package publishing

For `.deb`/APT and cross-distro guidance, follow:

- `docs/LINUX_PACKAGING.md`
