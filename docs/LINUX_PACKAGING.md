# Linux Packaging Guide

This guide covers practical distribution paths for Hop Launcher components.

## What to package

Current shippable pieces in this repository:

- `hopd` (Rust daemon)
- `kde-hopd-query` (KDE adapter helper CLI, shipped inside `hopd` package)
- `hop-hotkeyd` (Rust global hotkey agent)
- GNOME extension zip (`apps/gnome-extension/dist/*.zip`)

## Option A: Debian package (`.deb`) + APT repository

This is the path for Ubuntu/Debian users.

### A1) Build `.deb` packages (native Debian tooling)

This repository now includes `debian/` packaging skeletons in:

- `crates/hopd/debian`
- `crates/hop-hotkeyd/debian`

Build commands:

```bash
cd crates/hopd
dpkg-buildpackage -us -uc -b

cd ../hop-hotkeyd
dpkg-buildpackage -us -uc -b
```

Expected output: `.deb` files in the parent directory of each crate.

Required build dependencies on Debian/Ubuntu:

```bash
sudo apt update
sudo apt install -y build-essential debhelper dh-cargo cargo rustc dpkg-dev
```

### A2) Publish via APT (two common methods)

1. Launchpad PPA (Ubuntu-focused, hosted):
   - Best for Ubuntu users and minimal infra.
   - See: https://help.launchpad.net/Packaging/PPA

2. Self-hosted APT repo (Debian + Ubuntu):
   - Host signed repository metadata and `.deb` files.
   - Common tooling: `aptly` or `reprepro`.
   - See:
     - https://www.aptly.info/
     - https://wiki.debian.org/DebianRepository/SetupWithReprepro

Automation helper in this repo:

```bash
LAUNCHPAD_PPA='~owner/ubuntu/hop-launcher' RELEASE_TAG='v0.1.0' \
  bash scripts/release/publish-launchpad.sh
```

The script builds signed source packages and uploads with `dput` to Launchpad.

### A3) User installation (APT)

Once repo is published, users install with:

```bash
sudo apt update
sudo apt install hopd hop-hotkeyd
```

Then enable services for each user:

```bash
systemctl --user enable --now hopd.service
systemctl --user enable --now hop-hotkeyd.service
```

## Option B: Generic tarballs

Already produced by release workflow:

- `hopd-linux-x86_64.tar.gz`
- `hopd-linux-aarch64.tar.gz`
- `hop-hotkeyd-linux-x86_64.tar.gz`
- `hop-hotkeyd-linux-aarch64.tar.gz`
- `kde-hopd-query-linux-x86_64.tar.gz`
- `kde-hopd-query-linux-aarch64.tar.gz`

Good for early adopters and non-Debian distros.

Note: `.deb` packages are currently built for the runner host architecture in CI/release.

## Option C: Other distro channels

Depending on target audience:

- Fedora/RHEL: RPM (or COPR)
- Arch: AUR package
- openSUSE: OBS

Useful references:

- OBS: https://openbuildservice.org/help/
- COPR: https://docs.pagure.org/copr.copr/user_documentation.html
- AUR submission guidelines: https://wiki.archlinux.org/title/AUR_submission_guidelines

COPR automation helper in this repo:

```bash
COPR_OWNER='your-owner' COPR_PROJECT='hop-launcher' RELEASE_TAG='v0.1.0' \
  bash scripts/release/publish-copr.sh
```

The script expects RPM spec files at:

- `packaging/rpm/hopd.spec`
- `packaging/rpm/hop-hotkeyd.spec`

## Suggested release order

1. Release GitHub artifacts (zip + tarballs).
2. Publish APT packages for Debian/Ubuntu users.
3. Add RPM/AUR/OBS after APT flow is stable.

## Security and signing notes

- Sign packages/repositories with a dedicated release GPG key.
- Rotate keys on compromise and publish fingerprint in project docs.
- Keep CI secrets minimal and scoped to release only.

## Repository-maintainer note

If you prefer quicker local iteration over full Debian policy packaging,
`cargo-deb` is still useful. The `debian/` skeleton here is meant for
`dpkg-buildpackage` compatibility and long-term APT publication flow.
