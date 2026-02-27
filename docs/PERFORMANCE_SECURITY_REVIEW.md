# Performance & Security Review (Staff-level pass)

## Scope

Reviewed runtime paths in:
- `extension.js`
- `ui/launcherOverlay.js`
- `lib/fuzzy.js`
- `lib/providers/windows.js`
- `prefs.js`
- local install/package scripts

## High-impact findings and actions

### 1) Search computation responsiveness under larger result sets

**Finding:** Search ranking previously always executed synchronously on the shell thread.

**Risk:** UI frame drops during rapid typing when providers return larger sets.

**Mitigation implemented:**
- Added async ranking fallback via `GLib.idle_add` for result sets beyond a threshold (`ASYNC_SEARCH_THRESHOLD`).
- Added generation token cancellation to avoid stale updates applying after newer keystrokes.
- Added explicit cancellation of pending debounce/idle sources on close and destroy.

### 2) Ranking hot-path allocations

**Finding:** Every query rebuilt haystack strings for all items.

**Risk:** avoidable allocation/GC pressure at keystroke frequency.

**Mitigation implemented:**
- Added memoized `item._searchHaystack` in `rankResults`.
- Added fast-path for empty query to avoid fuzzy edit-distance work.

### 3) Action execution safety

**Finding:** Action mode used a generic signal emission (`global.display.emit('overlay-key')`).

**Risk:** brittle coupling to signal behavior and lower intent clarity.

**Mitigation implemented:**
- Switched to explicit `Main.overview.show()` call for action mode.

### 4) Window operation robustness

**Finding:** Workspace move logic assumed workspace exists; activation path had minimal validation.

**Risk:** edge-case errors for sticky windows / special window types.

**Mitigation implemented:**
- Added safety guards for override-redirect windows.
- Added null-check guards for workspace before move operations.

### 5) Keybinding input hardening in prefs

**Finding:** Preferences wrote arbitrary text directly into keybinding setting.

**Risk:** invalid accelerators degrade UX and can break toggle behavior.

**Mitigation implemented:**
- Added accelerator format validation using `Gtk.accelerator_parse` before write.

### 6) Local install script hardening

**Finding:** install script had no prerequisite checks and permissive umask.

**Risk:** confusing failures and broad file permissions in local extension folder.

**Mitigation implemented:**
- Added required tool checks (`rsync`, `gnome-extensions`).
- Added `umask 077` and excluded `dist/` during sync.

## Residual risks / next steps

- No end-to-end automated GNOME Shell integration tests in CI (manual QA still required).
- Recents provider remains placeholder; a future portal-based optional backend can be added with non-blocking behavior.
- For very large result spaces, consider incremental top-k scoring with chunked merges.

## Manual validation checklist

1. Run `./scripts/install-local.sh`.
2. Toggle launcher with `Super+Space` repeatedly (20+ cycles).
3. Type quickly in launcher and verify no visual stalls.
4. Verify `w ` and `a ` prefixes.
5. Validate window focus/close/move action on normal windows.
6. Set an invalid keybinding string in prefs and confirm it is ignored (previous valid shortcut remains effective).
7. Observe logs during testing via `journalctl --user -f /usr/bin/gnome-shell`.
