# Linux Packaging Guide

This guide covers practical distribution paths for Hop Launcher components.

## What to package

Current shippable pieces in this repository:

- `hopd` (Rust daemon)
- `hop-hotkeyd` (Rust global hotkey agent)
- GNOME extension zip (`apps/gnome-extension/dist/*.zip`)

## Option A: Debian package (`.deb`) + APT repository

This is the path for Ubuntu/Debian users.

### A1) Build `.deb` packages

The simplest maintainer path for Rust binaries is `cargo-deb`.

```bash
cargo install cargo-deb

cd crates/hopd
cargo deb

cd ../hop-hotkeyd
cargo deb
```

Expected output: `target/debian/*.deb` in each crate.

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
- `hop-hotkeyd-linux-x86_64.tar.gz`

Good for early adopters and non-Debian distros.

## Option C: Other distro channels

Depending on target audience:

- Fedora/RHEL: RPM (or COPR)
- Arch: AUR package
- openSUSE: OBS

Useful references:

- OBS: https://openbuildservice.org/help/
- COPR: https://docs.pagure.org/copr.copr/user_documentation.html
- AUR submission guidelines: https://wiki.archlinux.org/title/AUR_submission_guidelines

## Suggested release order

1. Release GitHub artifacts (zip + tarballs).
2. Publish APT packages for Debian/Ubuntu users.
3. Add RPM/AUR/OBS after APT flow is stable.

## Security and signing notes

- Sign packages/repositories with a dedicated release GPG key.
- Rotate keys on compromise and publish fingerprint in project docs.
- Keep CI secrets minimal and scoped to release only.
