# Full Linux Parity, Polish, and Release Design

Date: 2026-03-05
Status: Proposed

## Goal

Deliver full Linux product readiness across functional parity, UI polish, and release/CI hardening so Hop Launcher can ship predictable desktop behavior with a repeatable release pipeline.

## Scope

1. Functional parity across `hopd`, GTK launcher, GNOME extension convergence, and KDE adapter surface.
2. UX polish for desktop quality (visual consistency, accessibility, copy/enter behaviors, interaction quality).
3. End-to-end CI/release maturity (versioned releases, signed/checksummed artifacts, multi-arch matrix, publish-ready package flow).

## Approaches Considered

### Approach A: Big-bang parity + polish + release in one branch

Pros:
- Single integration event.
- Fastest path to one final state.

Cons:
- High regression risk.
- Difficult review/debug when failures appear.
- Blocks release if any one stream slips.

### Approach B (Recommended): Single plan with gated phased execution

Pros:
- Maintains one end-to-end strategy while reducing risk.
- Each gate has explicit verification and rollback surface.
- Enables incremental user validation (functional first, then polish, then release hardening).

Cons:
- Slightly longer wall-clock timeline.
- Requires strict milestone discipline.

### Approach C: CI/release first, then parity and polish

Pros:
- Faster improvements to infrastructure confidence.

Cons:
- Ships hardened pipeline around incomplete product behavior.
- Creates rework when parity changes alter artifact structure.

## Decision

Adopt Approach B. Execute one integrated plan with 3 gated streams:
1. Parity completion.
2. UX polish completion.
3. CI/release hardening completion.

## Architecture and Delivery Model

### Stream 1: Functional Parity

- Finalize backend/provider parity deltas in `hopd` and convergence toggles.
- Promote KDE adapter from helper surface to integration-ready runner contract + packaging docs.
- Standardize action semantics (`enter`, `copy`, open settings, contextual execution metadata).

### Stream 2: UX Polish

- Consolidate GTK visual system (spacing, typography hierarchy, icon language, reduced noise defaults).
- Add accessibility and interaction quality pass (keyboard consistency, focus visibility, screen reader metadata where supported).
- Add visual/behavior regression checks for key states.

### Stream 3: CI/Release Process

- Move from rolling `main` prerelease to immutable semver tags.
- Generate checksums + provenance metadata, optional signing hooks.
- Add multi-arch build/test matrix and artifact manifest validation.
- Add publishable package workflow (APT-ready metadata + documented publish promotion steps).

## Risks and Mitigations

1. Provider parity drift between GNOME and `hopd`.
   - Mitigation: route-by-route convergence tests and feature-gated rollout.
2. KDE integration ambiguity without a full native shell component.
   - Mitigation: lock CLI/runner contract and package it as supported adapter surface.
3. Release hardening complexity (signing/provenance).
   - Mitigation: stage checksums/provenance first, signing optional but pluggable in same workflow.

## Validation Strategy

1. Contract/integration tests for each provider route and action kind.
2. Cross-frontend regression suite (`hopd`, GTK `gtk_ui`, GNOME extension tests).
3. Release workflow dry-run on tag candidate branch.
4. Artifact verification: checksums, expected files, package install smoke.

## Success Criteria

1. Functional: all target provider/action behaviors match defined parity matrix.
2. UX: launcher defaults are minimal, keyboard-first, and visually consistent.
3. Release: semver-tagged release emits validated artifacts and reproducible metadata.
