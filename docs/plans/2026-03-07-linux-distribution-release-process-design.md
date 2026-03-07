# Linux Distribution Release Process Design

## Context

Hop Launcher already ships:

- Release tarballs and checksums in GitHub workflows
- Debian package skeletons for `hopd` and `hop-hotkeyd`
- A release checklist and Linux packaging documentation

The missing piece is a production release process that automatically publishes distro packages after tests pass.

## Goals

1. Ship official Linux packages for Ubuntu/Debian users via APT and Fedora users via RPM.
2. Keep release execution automated and test-gated from semver tags.
3. Preserve immutable artifacts and checksum verification.
4. Defer AUR to a later phase after distro publishing is stable.

## Non-Goals (Phase 1)

- AUR automation and maintenance
- openSUSE OBS publication
- Self-hosted repository infrastructure for APT/RPM

## Approaches Considered

1. Managed-hosting first (Launchpad PPA + COPR) with GitHub tarballs as canonical fallback.
2. Fully self-hosted apt/rpm repositories from day one.
3. Third-party package SaaS as canonical source.

### Recommendation

Use approach 1.

Reasoning:

- Free/low-cost path for open-source distribution
- Fastest time-to-release with low infrastructure burden
- Best fit for current repository maturity and existing Debian metadata

## Architecture

Release source of truth:

- Semver Git tags on `main` (`vX.Y.Z`)

Delivery channels:

- GitHub Releases: tarballs (`x86_64`, `aarch64`) + `SHA256SUMS`
- Ubuntu: Launchpad PPA publication
- Fedora: COPR publication

Release gating:

- Full test matrix from `docs/RELEASE.md`
- Artifact manifest and checksum verification scripts
- Publish jobs only after verification passes

## Pipeline/Data Flow

1. Maintainer updates version/changelogs and pushes `vX.Y.Z`.
2. CI runs full validation.
3. On success, workflow:
   - builds release artifacts
   - publishes GitHub Release assets
   - publishes to Launchpad PPA
   - triggers COPR builds
4. Post-publish smoke checks validate service startup and CLI behavior.
5. Channel failures produce a partial-release status and logs; retry is fix-forward via next patch tag.

## Failure Policy

- No mutable/overwrite behavior for published artifacts.
- If one channel fails (`ppa` or `copr`), keep successful channels published and mark release as partial.
- Recover by shipping a new semver patch release (`vX.Y.Z+1` semantics via normal semver patch).

## Security and Trust

- Dedicated release signing key for package/repository metadata.
- Published key fingerprint in docs.
- Minimal CI secret scope to publishing jobs only.

## Testing Strategy

Pre-publish checks:

- Existing Rust/Node test suites
- Packaging dry-runs for Debian source/binary generation
- Artifact manifest + checksums validation

Post-publish checks:

- Install from PPA and COPR in clean environments
- Verify `hopd` and `hop-hotkeyd` user services and control commands
- Validate search/action behavior parity with existing smoke checklist

## Operational Runbook (Maintainer)

1. Merge release-ready changes to `main`.
2. Update package metadata/changelog/version files.
3. Push signed tag `vX.Y.Z`.
4. Monitor CI publish jobs.
5. Execute smoke checks and publish release notes.

## Phase 2 Placeholder

After 2-3 stable managed releases, add Arch AUR automation:

- Introduce `PKGBUILD` and `.SRCINFO`
- Add CI update automation for Arch package metadata
- Define maintainer policy (official vs community)
