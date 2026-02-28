# Smart Providers Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add offline-first smart launcher capabilities (calculator, currency conversion with cache + optional refresh, timezone lookup, emoji picker, and indexed file search) while preserving fuzzy ranking and app/window priority.

**Architecture:** Implement provider-per-feature modules and a query router that chooses provider scope from prefixes and lightweight intent detection. Keep one ranked list via `rankResults`, adding new result kinds and configurable weights. Start with provider/route/ranking tests (TDD), then implement minimal provider logic incrementally.

**Tech Stack:** GNOME Shell extension JS (GJS), GLib/Gio, existing fuzzy ranker, Node test runner (`node --test`).

---

### Task 1: Add router and ranking tests for new kinds

**Files:**
- Create: `tests/query-router.test.mjs`
- Modify: `tests/fuzzy.test.mjs`
- Create: `lib/queryRouter.js`

**Step 1: Write the failing test**

```js
import test from 'node:test';
import assert from 'node:assert/strict';
import {extractQueryRoute} from '../lib/queryRouter.js';

test('prefix f routes to files mode', () => {
  assert.deepEqual(extractQueryRoute('f report'), {mode: 'files', query: 'report'});
});
```

Add failing assertions in `tests/fuzzy.test.mjs` for source weight ordering with `window`, `app`, `file`, `emoji`, `utility` kinds.

**Step 2: Run test to verify it fails**

Run: `node --test tests/query-router.test.mjs tests/fuzzy.test.mjs`
Expected: FAIL due to missing router module/new weight handling.

**Step 3: Write minimal implementation**

Implement `extractQueryRoute()` in `lib/queryRouter.js` for prefixes:
- `w ` -> windows
- `a ` -> apps
- `f ` -> files
- `:emoji ` -> emoji
- `tz ` -> timezone
- `$` -> currency
- `=` -> calculator
- fallback -> all

Extend `lib/fuzzy.js` `sourceWeight` map with `file`, `emoji`, `utility` keys and options.

**Step 4: Run test to verify it passes**

Run: `node --test tests/query-router.test.mjs tests/fuzzy.test.mjs`
Expected: PASS.

**Step 5: Commit**

```bash
git add lib/queryRouter.js lib/fuzzy.js tests/query-router.test.mjs tests/fuzzy.test.mjs
git commit -m "test: add routing and ranking coverage for smart providers"
```

### Task 2: Integrate router and provider dispatch in overlay

**Files:**
- Modify: `ui/launcherOverlay.js`
- Modify: `extension.js`

**Step 1: Write the failing test**

Add/extend `tests/query-router.test.mjs` with integration-like unit checks that expected modes map to provider kinds.

```js
test('implicit math query routes to calculator intent', () => {
  const route = extractQueryRoute('2+2');
  assert.equal(route.mode, 'calculator');
});
```

**Step 2: Run test to verify it fails**

Run: `node --test tests/query-router.test.mjs`
Expected: FAIL for unsupported intent route(s).

**Step 3: Write minimal implementation**

- Replace `_extractMode()` in overlay with router usage.
- Replace `_collectItems(mode)` with provider-aware filtering by `provider.kind` / result kinds.
- Keep existing action mode `>` support.
- Register new provider instances in `extension.js` in deterministic order (windows/apps first).

**Step 4: Run test to verify it passes**

Run: `node --test tests/query-router.test.mjs tests/fuzzy.test.mjs`
Expected: PASS.

**Step 5: Commit**

```bash
git add ui/launcherOverlay.js extension.js lib/queryRouter.js tests/query-router.test.mjs
git commit -m "feat: route queries and dispatch smart providers"
```

### Task 3: Implement calculator provider with safe expression evaluation

**Files:**
- Create: `lib/providers/calculator.js`
- Create: `tests/providers/calculator.test.mjs`

**Step 1: Write the failing test**

```js
import test from 'node:test';
import assert from 'node:assert/strict';
import {evaluateExpression} from '../../lib/providers/calculator.js';

test('evaluates basic arithmetic', () => {
  assert.equal(evaluateExpression('2+2'), '4');
});
```

Add tests for invalid expressions returning `null`.

**Step 2: Run test to verify it fails**

Run: `node --test tests/providers/calculator.test.mjs`
Expected: FAIL (module/function missing).

**Step 3: Write minimal implementation**

- Implement tokenizer + shunting-yard (or equivalent bounded parser) for `+ - * / ( )`.
- Expose provider that returns `kind: 'utility'` result row with copy action.

**Step 4: Run test to verify it passes**

Run: `node --test tests/providers/calculator.test.mjs`
Expected: PASS.

**Step 5: Commit**

```bash
git add lib/providers/calculator.js tests/providers/calculator.test.mjs
git commit -m "feat: add offline calculator provider"
```

### Task 4: Implement timezone provider with alias resolution

**Files:**
- Create: `lib/providers/timezone.js`
- Create: `lib/data/timezone-aliases.js`
- Create: `tests/providers/timezone.test.mjs`

**Step 1: Write the failing test**

```js
import test from 'node:test';
import assert from 'node:assert/strict';
import {resolveTimezoneQuery} from '../../lib/providers/timezone.js';

test('resolves pst alias', () => {
  assert.equal(resolveTimezoneQuery('tz pst')[0].iana, 'America/Los_Angeles');
});
```

**Step 2: Run test to verify it fails**

Run: `node --test tests/providers/timezone.test.mjs`
Expected: FAIL.

**Step 3: Write minimal implementation**

- Add compact alias map for common regions/abbreviations.
- Use `Intl.DateTimeFormat` with `timeZone` to format current time safely.
- Return `kind: 'utility'` rows with zone and current time subtitle.

**Step 4: Run test to verify it passes**

Run: `node --test tests/providers/timezone.test.mjs`
Expected: PASS.

**Step 5: Commit**

```bash
git add lib/providers/timezone.js lib/data/timezone-aliases.js tests/providers/timezone.test.mjs
git commit -m "feat: add offline timezone lookup provider"
```

### Task 5: Implement emoji provider

**Files:**
- Create: `lib/providers/emoji.js`
- Create: `lib/data/emoji-keywords.js`
- Create: `tests/providers/emoji.test.mjs`

**Step 1: Write the failing test**

```js
import test from 'node:test';
import assert from 'node:assert/strict';
import {searchEmoji} from '../../lib/providers/emoji.js';

test('finds smile emoji by keyword', () => {
  const results = searchEmoji('smile');
  assert.equal(results[0].emoji, '😀');
});
```

**Step 2: Run test to verify it fails**

Run: `node --test tests/providers/emoji.test.mjs`
Expected: FAIL.

**Step 3: Write minimal implementation**

- Add compact local emoji dataset (name + keywords + glyph).
- Return `kind: 'emoji'` rows with copy-to-clipboard execute action.

**Step 4: Run test to verify it passes**

Run: `node --test tests/providers/emoji.test.mjs`
Expected: PASS.

**Step 5: Commit**

```bash
git add lib/providers/emoji.js lib/data/emoji-keywords.js tests/providers/emoji.test.mjs
git commit -m "feat: add offline emoji picker provider"
```

### Task 6: Implement currency provider with cache and refresh policy

**Files:**
- Create: `lib/providers/currency.js`
- Create: `tests/providers/currency.test.mjs`
- Modify: `schemas/org.example.launcher.gschema.xml`
- Modify: `prefs.js`

**Step 1: Write the failing test**

```js
import test from 'node:test';
import assert from 'node:assert/strict';
import {parseCurrencyQuery, convertWithRates} from '../../lib/providers/currency.js';

test('parses conversion query', () => {
  assert.deepEqual(parseCurrencyQuery('100 usd to eur'), {amount: 100, from: 'USD', to: 'EUR'});
});
```

Add tests for stale cache behavior and offline fallback metadata.

**Step 2: Run test to verify it fails**

Run: `node --test tests/providers/currency.test.mjs`
Expected: FAIL.

**Step 3: Write minimal implementation**

- Parse `amount from to target` patterns.
- Read/write cached rates JSON in extension state/cache path.
- Add settings for refresh enabled + TTL hours.
- Return cached conversion row immediately; trigger async refresh when stale.

**Step 4: Run test to verify it passes**

Run: `node --test tests/providers/currency.test.mjs`
Expected: PASS.

**Step 5: Commit**

```bash
git add lib/providers/currency.js tests/providers/currency.test.mjs schemas/org.example.launcher.gschema.xml prefs.js
git commit -m "feat: add cached currency conversion provider"
```

### Task 7: Implement indexed file provider with user-managed folders

**Files:**
- Create: `lib/providers/files.js`
- Create: `lib/index/fileIndexer.js`
- Create: `tests/providers/files.test.mjs`
- Modify: `schemas/org.example.launcher.gschema.xml`
- Modify: `prefs.js`

**Step 1: Write the failing test**

```js
import test from 'node:test';
import assert from 'node:assert/strict';
import {scoreFileMatch} from '../../lib/providers/files.js';

test('filename score outranks path-only score', () => {
  const byName = scoreFileMatch('report', 'report-q1.pdf', '/docs/finance/report-q1.pdf');
  const byPath = scoreFileMatch('report', 'q1.pdf', '/docs/finance/report-q1.pdf');
  assert.ok(byName > byPath);
});
```

Add tests for folder inclusion/exclusion and incremental refresh.

**Step 2: Run test to verify it fails**

Run: `node --test tests/providers/files.test.mjs`
Expected: FAIL.

**Step 3: Write minimal implementation**

- Add settings for indexed folders list.
- Implement incremental index storing path/name/mtime.
- Search index with fuzzy filename-first scoring.
- Return `kind: 'file'` rows with open-file execute action.

**Step 4: Run test to verify it passes**

Run: `node --test tests/providers/files.test.mjs`
Expected: PASS.

**Step 5: Commit**

```bash
git add lib/providers/files.js lib/index/fileIndexer.js tests/providers/files.test.mjs schemas/org.example.launcher.gschema.xml prefs.js
git commit -m "feat: add indexed file search provider"
```

### Task 8: Wire all providers into extension and ranking settings

**Files:**
- Modify: `extension.js`
- Modify: `lib/fuzzy.js`
- Modify: `ui/launcherOverlay.js`
- Modify: `schemas/org.example.launcher.gschema.xml`
- Modify: `prefs.js`

**Step 1: Write the failing test**

Add assertions in router/ranking tests verifying windows/apps still outrank other kinds when fuzzy scores are equivalent.

**Step 2: Run test to verify it fails**

Run: `node --test tests/query-router.test.mjs tests/fuzzy.test.mjs`
Expected: FAIL.

**Step 3: Write minimal implementation**

- Register new providers in extension startup.
- Add weight settings: `weight-files`, `weight-emoji`, `weight-utility`.
- Expose settings controls in preferences.
- Keep default weights favoring windows/apps.

**Step 4: Run test to verify it passes**

Run: `node --test tests/query-router.test.mjs tests/fuzzy.test.mjs`
Expected: PASS.

**Step 5: Commit**

```bash
git add extension.js lib/fuzzy.js ui/launcherOverlay.js schemas/org.example.launcher.gschema.xml prefs.js tests/query-router.test.mjs tests/fuzzy.test.mjs
git commit -m "feat: wire smart providers with source-priority controls"
```

### Task 9: Verify and document

**Files:**
- Modify: `README.md`
- Optional Modify: `docs/PERFORMANCE_SECURITY_REVIEW.md`

**Step 1: Write the failing test**

Add regression assertions for any parser edge case discovered during manual testing.

**Step 2: Run test to verify it fails**

Run: `npm test`
Expected: FAIL until edge case handling is implemented.

**Step 3: Write minimal implementation**

- Fix failing edge case.
- Update README feature/prefix documentation and setup notes for currency refresh + file indexing.

**Step 4: Run test to verify it passes**

Run:
- `npm test`
- `glib-compile-schemas --strict --dry-run schemas`
- `bash -n scripts/install-local.sh scripts/package-extension.sh`

Expected: all PASS.

**Step 5: Commit**

```bash
git add README.md docs/PERFORMANCE_SECURITY_REVIEW.md tests
git commit -m "docs: cover smart provider usage and verification"
```
