# Cross-Linux Full App Parity Design (Functional First)

Date: 2026-03-04
Status: Approved

## Goal

Evolve the Linux native implementation from a phase-1 scaffold into a full launcher app with functional parity to the GNOME implementation, while prioritizing behavior parity before visual refinement.

## Confirmed Product Decisions

1. Scope targets full GNOME feature parity now (apps, windows, files, recents, settings, utilities).
2. Priority is functional parity first; visual parity can follow.
3. Enter executes the selected result's primary action directly.

## Problem Statement

Current GTK behavior can show a connected state but limited/no actionable outcomes because:

1. `hopd` currently exposes utility-focused provider heuristics and stubbed execute behavior.
2. GTK status messaging is static and can imply success even when the result/action layer is thin.
3. GNOME has the richer provider and UX model today, creating cross-frontend capability gaps.

## Approach Options Considered

### Option 1 (Recommended): Backend-Centric Parity in `hopd`

Move provider aggregation, routing, ranking, and action dispatch into `hopd` so GTK and GNOME can converge on one backend behavior model.

Pros:
- Single source of truth for search/action behavior.
- Better cross-desktop consistency and maintainability.
- Easier future KDE/other frontend integration.

Cons:
- Larger initial implementation effort.
- Requires robust action abstraction per desktop/session.

### Option 2: GTK-Local Provider Logic

Implement full provider/action logic directly in GTK client.

Pros:
- Fast short-term GTK results.

Cons:
- Duplication with GNOME logic.
- Parity drift and long-term maintenance risk.

### Option 3: Hybrid JS Bridge in GTK

Reuse GNOME JS modules from GTK via runtime bridge.

Pros:
- Potential short-term reuse.

Cons:
- Packaging/runtime complexity.
- Brittle cross-language boundary.

Decision: Option 1.

## Target Architecture

### `hopd` responsibilities

1. Query routing (`all`, `weather`, `timezone`, `emoji`, `settings`, `files`, `apps`, `windows`, `recents`).
2. Provider aggregation across all supported result kinds.
3. Ranking and normalized result shaping for frontend consumption.
4. Action execution contract for primary enter behavior (`actions.execute`).

### GTK responsibilities

1. Thin presentation client over `hopd` IPC.
2. GNOME-like search lifecycle and keyboard UX.
3. Query-state status model (searching/results/empty/error/action result).
4. Execute selected row through standardized daemon contract.

### GNOME convergence strategy

1. Keep current GNOME providers initially for stability.
2. Add staged opt-in path for GNOME to consume expanded `hopd` routes.
3. Validate parity route-by-route before default switch.

## Functional UX Spec (GTK)

### Query lifecycle

1. Empty query shows top suggestions (not blank canvas).
2. Typing triggers debounced search; show loading row for slower responses.
3. No matches show explicit empty state plus example queries.

### Result model

Normalized rows from `hopd` include:
- `id`
- `kind` (`app`, `window`, `file`, `recent`, `setting`, `utility`)
- `title`
- `subtitle`
- `icon`
- `primary_action`
- `score`
- optional `hints`

### Input and actions

1. `Up`/`Down`: move selection.
2. `Enter`: execute selected primary action.
3. `Esc`: close launcher.
4. Row activate behavior mirrors Enter.

### Status messaging policy

Replace static “Connected to hopd” with contextual states:
- `Searching…`
- `<n> results`
- `No results`
- `Action failed: <reason>`

## Error Handling

1. Daemon unreachable: actionable transport error state in UI.
2. Execute failures: preserve selection/query, show failure reason.
3. Unsupported action/result kind: explicit non-fatal error path.

## Testing Strategy

1. IPC contract tests for all result kinds and enter-action execution payloads.
2. End-to-end daemon socket tests for multi-request search/execute flows.
3. GTK behavior tests for state transitions and keyboard execution paths where feasible.
4. Manual Linux smoke checks extended beyond utility routes.

## Milestones

1. Milestone A: IPC contract and result schema hardening.
2. Milestone B: `hopd` provider and execution expansion to full parity kinds.
3. Milestone C: GTK UI behavior parity (state machine, richer rows, keyboard flow).
4. Milestone D: GNOME staged convergence onto `hopd` routes.
5. Milestone E: performance and release quality gates.

## Non-Goals (This Iteration)

1. Pixel-perfect GNOME visual parity.
2. Broad packaging redesign beyond current release flow.
3. New feature categories outside existing GNOME parity set.
