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
cd ../..
./scripts/run-gtk-manual-test.sh
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
- `hopd-linux-aarch64.tar.gz`
- `hop-hotkeyd-linux-x86_64.tar.gz`
- `hop-hotkeyd-linux-aarch64.tar.gz`
- `kde-hopd-query-linux-x86_64.tar.gz`
- `kde-hopd-query-linux-aarch64.tar.gz`
- `.deb` packages for `hopd` and `hop-hotkeyd`
- artifact manifest (`dist/ARTIFACT_MANIFEST.txt`) validated before upload/release
- `SHA256SUMS` checksum file for all release assets

Trigger options:

- automatic on semver tag pushes (`v*.*.*`)
- manual via `workflow_dispatch` with explicit `tag_name`

CI (`.github/workflows/ci.yml`) also builds `.deb` packages on PR/push and uploads them as artifact `hop-launcher-deb`.

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
4. Verify launcher parity behavior in GTK:
   - Query utility routes (`weather zurich`, `time in tokyo`, `emoji smile`).
   - Query non-utility routes (`terminal`, `w terminal`, `f readme`, `settings bluetooth`).
   - Confirm status transitions (`Searching...`, `<n> results`, `No results`) and Enter action execution.
5. (Optional staged rollout) If validating GNOME convergence path, enable:
   - `feature-hopd-enabled=true`
   - `feature-hopd-convergence-enabled=true`

## 5) Linux package publishing

For `.deb`/APT and cross-distro guidance, follow:

- `docs/LINUX_PACKAGING.md`
