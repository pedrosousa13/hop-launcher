# Full Linux Parity, Polish, and Release Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Complete feature parity, UI polish, and production-grade CI/release workflow for Linux-native Hop Launcher.

**Architecture:** Deliver in gated phases: (1) parity behavior completion in `hopd` + adapters, (2) GTK/desktop polish and a11y consistency, (3) CI/release hardening with immutable versioned artifacts and verification metadata. Each phase must pass full repo verification before moving forward.

**Tech Stack:** Rust (`hopd`, `hop-hotkeyd`, GTK launcher), JS (GNOME extension), GitHub Actions, Debian packaging.

---

### Task 1: Define and lock parity matrix

**Files:**
- Modify: `docs/CROSS_LINUX_PLAN.md`
- Create: `docs/plans/2026-03-05-parity-matrix.md`
- Test: `crates/hopd/tests/providers_parity.rs`, `crates/hopd/tests/ipc_contract.rs`

**Step 1: Add explicit parity matrix document**

Create a table with provider/action rows (`apps`, `windows`, `files`, `recents`, `settings`, `weather`, `timezone`, `emoji`, `calculator`, `currency`, `copy`) and columns (`hopd`, `GTK`, `GNOME`, `KDE adapter`).

**Step 2: Add missing parity expectations in tests**

Extend existing `providers_parity` and IPC tests to assert behavior for each enabled mode and action kind.

**Step 3: Run targeted parity tests**

Run: `cd crates/hopd && cargo test --test providers_parity --test ipc_contract`
Expected: PASS with explicit parity assertions.

**Step 4: Commit**

```bash
git add docs/plans/2026-03-05-parity-matrix.md docs/CROSS_LINUX_PLAN.md crates/hopd/tests/providers_parity.rs crates/hopd/tests/ipc_contract.rs
git commit -m "docs(parity): define locked cross-frontend parity matrix"
```

### Task 2: Complete action parity (`enter`/`copy` semantics)

**Files:**
- Modify: `crates/hopd/src/actions.rs`
- Modify: `apps/gtk-launcher/src/lib.rs`
- Modify: `apps/gnome-extension/lib/resultAction.js`
- Test: `crates/hopd/tests/actions_execute.rs`, `apps/gnome-extension/tests/result-action.test.mjs`, `apps/gtk-launcher/src/lib.rs` tests

**Step 1: Write failing tests for copy-capable rows**

Add tests for utility/text/emoji results where Enter resolves to copy action metadata and returns success.

**Step 2: Implement action routing updates**

In `actions.rs`, support copy-oriented resolution metadata for applicable IDs while preserving spawn behavior for executable rows.

**Step 3: Wire client-side parsing**

Update GTK/GNOME action handling to consume standardized action metadata.

**Step 4: Run focused action tests**

Run:
- `cd crates/hopd && cargo test --test actions_execute`
- `cd apps/gnome-extension && npm test -- tests/result-action.test.mjs`
- `cd apps/gtk-launcher && cargo test`
Expected: PASS.

**Step 5: Commit**

```bash
git add crates/hopd/src/actions.rs crates/hopd/tests/actions_execute.rs apps/gtk-launcher/src/lib.rs apps/gnome-extension/lib/resultAction.js apps/gnome-extension/tests/result-action.test.mjs
git commit -m "feat(actions): align enter/copy semantics across frontends"
```

### Task 3: Finish KDE adapter surface contract

**Files:**
- Modify: `crates/hopd/src/bin/kde-hopd-query.rs`
- Modify: `docs/HOPD_LOCAL_INSTALL.md`
- Modify: `docs/LINUX_PACKAGING.md`
- Test: `crates/hopd/tests/kde_adapter.rs`

**Step 1: Add stable runner contract docs**

Document TSV fields, escaping rules, and exit codes for `--format runner`.

**Step 2: Add/expand contract tests**

Assert output format stability and error semantics for no-intent/mode/execute failures.

**Step 3: Verify release build includes contract binary**

Run: `cd crates/hopd && cargo build --release --bin kde-hopd-query`
Expected: binary builds and packaging paths remain valid.

**Step 4: Commit**

```bash
git add crates/hopd/src/bin/kde-hopd-query.rs crates/hopd/tests/kde_adapter.rs docs/HOPD_LOCAL_INSTALL.md docs/LINUX_PACKAGING.md
git commit -m "feat(kde-adapter): lock runner contract and docs"
```

### Task 4: Polish GTK visual defaults and noise budget

**Files:**
- Modify: `apps/gtk-launcher/src/main.rs`
- Modify: `apps/gtk-launcher/README.md`
- Test: `apps/gtk-launcher/src/lib.rs` tests (state text, behavior), manual smoke script

**Step 1: Write failing behavior tests for minimal UI defaults**

Assert default state expectations (no extra header noise, selection/enter behavior intact, copy actions preserved).

**Step 2: Implement minimal visual defaults**

Reduce persistent chrome/noise while preserving settings access and keyboard flow.

**Step 3: Add polish-focused manual checklist entries**

Update docs/manual script with visual acceptance checks.

**Step 4: Run GTK test suite**

Run: `cd apps/gtk-launcher && cargo test --features gtk_ui`
Expected: PASS.

**Step 5: Commit**

```bash
git add apps/gtk-launcher/src/main.rs apps/gtk-launcher/README.md scripts/run-gtk-manual-test.sh
git commit -m "polish(gtk): enforce minimal default chrome and interaction quality"
```

### Task 5: Accessibility and keyboard consistency pass

**Files:**
- Modify: `apps/gtk-launcher/src/main.rs`
- Modify: `apps/gnome-extension/ui/launcherOverlay.js`
- Test: `apps/gnome-extension/tests/launcher-overlay-layout.test.mjs`, GTK tests

**Step 1: Add failing tests for shortcut and focus consistency**

Cover settings shortcut behavior (`Ctrl+,`/platform equivalent), focus restoration, and accessible labels.

**Step 2: Implement consistency updates**

Ensure equivalent key paths and focus rules across GTK and GNOME surfaces.

**Step 3: Run frontend test suites**

Run:
- `cd apps/gtk-launcher && cargo test --features gtk_ui`
- `cd apps/gnome-extension && npm test`
Expected: PASS.

**Step 4: Commit**

```bash
git add apps/gtk-launcher/src/main.rs apps/gnome-extension/ui/launcherOverlay.js apps/gnome-extension/tests/launcher-overlay-layout.test.mjs
git commit -m "polish(a11y): align keyboard and focus behavior across launchers"
```

### Task 6: Harden CI quality gates

**Files:**
- Modify: `.github/workflows/ci.yml`
- Modify: `.github/workflows/release.yml`
- Create/Modify: `crates/hopd/scripts/*.sh` as needed

**Step 1: Ensure GTK CI runs `gtk_ui` feature tests**

Update CI job command to `cargo test --features gtk_ui`.

**Step 2: Add artifact manifest verification step**

Validate expected artifact filenames exist before upload in CI/release jobs.

**Step 3: Keep benchmark guardrails + trend as required checks**

Fail CI on script errors or missing benchmark outputs.

**Step 4: Validate workflow syntax**

Run local workflow lint if available (`act` or static yaml checks), and run repo tests.

**Step 5: Commit**

```bash
git add .github/workflows/ci.yml .github/workflows/release.yml crates/hopd/scripts
git commit -m "ci: harden quality gates and artifact manifest checks"
```

### Task 7: Productionize release versioning and artifacts

**Files:**
- Modify: `.github/workflows/release.yml`
- Modify: `docs/RELEASE.md`
- Create: `scripts/release/generate-checksums.sh`
- Create: `dist/manifest` generation logic in workflow

**Step 1: Switch release to semver-tag driven flow**

Trigger only on `v*.*.*` tags for production release job; keep manual dispatch for dry-runs.

**Step 2: Generate checksums and manifest**

Produce `SHA256SUMS` for zip/tar/deb assets and upload with artifacts.

**Step 3: Add optional provenance/signing hook points**

Use workflow conditionals/secrets gates so signing can be enabled without altering logic.

**Step 4: Dry-run release pipeline on test tag**

Expected: artifacts + checksums produced; release notes generated.

**Step 5: Commit**

```bash
git add .github/workflows/release.yml docs/RELEASE.md scripts/release/generate-checksums.sh
git commit -m "release: move to semver tags with checksum manifest"
```

### Task 8: Multi-arch and packaging publish readiness

**Files:**
- Modify: `.github/workflows/release.yml`
- Modify: `docs/LINUX_PACKAGING.md`
- Create/Modify: packaging publish helper docs/scripts under `scripts/release/`

**Step 1: Add architecture matrix plan**

Add release matrix stubs/implementation for at least `x86_64` and `aarch64` build artifacts where feasible.

**Step 2: Document APT promotion process**

Define explicit post-build publish steps and required signatures.

**Step 3: Add release checklist gates**

Include verification for `.deb` install smoke and adapter binary presence.

**Step 4: Commit**

```bash
git add .github/workflows/release.yml docs/LINUX_PACKAGING.md scripts/release
git commit -m "release(packaging): add multi-arch and publish-readiness flow"
```

### Task 9: Final full verification gate

**Files:**
- Modify: `docs/RELEASE.md` (final checklist update)

**Step 1: Run full verification matrix**

Run:
- `cd crates/hopd && cargo test`
- `cd crates/hop-hotkeyd && cargo test`
- `cd apps/gtk-launcher && cargo test --features gtk_ui`
- `cd apps/gnome-extension && npm test`
- benchmark scripts dry-run
- packaging build dry-run (`dpkg-buildpackage` for both crates)

Expected: all pass.

**Step 2: Update release checklist with observed commands and outputs**

Record concrete “last known good” verification commands/dates.

**Step 3: Commit**

```bash
git add docs/RELEASE.md
git commit -m "docs(release): finalize verification gates for production readiness"
```
