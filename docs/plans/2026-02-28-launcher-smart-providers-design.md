# Launcher Smart Providers Design

## Scope
Add offline-first smart search capabilities to Hop Launcher while preserving fuzzy-first ranking and app/window prioritization. New capabilities:
- Calculator from math input
- Currency conversion with cached rates and optional online refresh
- Time/timezone lookup
- Emoji picker
- File search across user-chosen indexed folders

## Architecture
Use modular providers plus a lightweight query router.

Providers:
- Existing: `windows`, `apps`, `recents`
- New: `calculator`, `currency`, `timezone`, `emoji`, `files`

Router behavior:
- Prefix routes for explicit intent (for example: `=`, `$`, `tz `, `:emoji `, `f `)
- Intent detection without prefix for utility queries (math/time/currency patterns)
- Default mixed mode includes all enabled providers

All provider outputs feed a single ranking pipeline (`rankResults`) with per-kind source weights. New result kinds get their own weight controls so apps/windows remain globally prioritized.

## Feature Behavior

### Calculator (offline)
- Detect math-like expressions while typing.
- Evaluate through a safe parser (no `eval`).
- Return a top utility row with value and copy-on-enter behavior.

### Currency conversion (online optional)
- Parse conversions like `100 usd to eur`.
- Return immediate result from cached rates.
- Refresh rates in background with TTL (e.g. 12h).
- On offline/error, keep stale cache and surface timestamp metadata.

### Time/timezone lookup (offline)
- Handle queries such as `time tokyo`, `tz pst`, `now in berlin`.
- Resolve common aliases to IANA zones.
- Return local current time for matching zones.

### Emoji picker (offline)
- Prefix `:emoji ` plus implicit keyword matching.
- Fuzzy match emoji names + keywords.
- Enter copies emoji to clipboard.

### File search (offline)
- User-managed indexed folders from preferences.
- Maintain local index (name/path/mtime).
- Fuzzy score filename first, path second.
- Enter opens file via default app.

## Data Flow
1. Text changes in overlay input.
2. Router derives mode, explicit prefix, and implied intent.
3. Relevant providers return candidate rows.
4. Unified fuzzy ranking + source weighting orders results.
5. UI renders ranked rows.

Async rules:
- Fast providers may run synchronously.
- Slower providers return cache first and update in next debounced pass.
- Existing generation ID guards prevent stale async result commits.

## Error Handling
- Provider failures are isolated from one another.
- Currency network failure keeps cached rates with stale/offline marker.
- File indexing skips inaccessible paths and reports compact warning row.
- Invalid calculator input quietly produces no result.

## Testing Strategy
- Extend ranking tests to include new kinds/weights.
- Provider parser tests:
  - Calculator parsing/evaluation
  - Currency parsing + cache staleness behavior
  - Timezone alias resolution
  - Emoji keyword matching
  - File query scoring helpers
- Router tests:
  - Explicit prefix handling
  - Implicit intent handling
  - Mixed-mode app/window priority under close fuzzy scores
