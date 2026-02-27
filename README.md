# Hop Launcher (GNOME Wayland)

A polished **Raycast-lite** command palette for GNOME Shell 45+ on Wayland.

## Features (v1)

- Single centered overlay (`Super+Space`) for:
  - Open windows
  - Installed apps
  - Optional recents provider placeholder
- Fuzzy matching with typo tolerance and ranking boosts for:
  - ordered matches
  - contiguous runs
  - word/camel boundaries
  - early matches
- Built-in window switching and window actions:
  - focus
  - close
  - move to next workspace
- Prefix filters:
  - `w ` windows only
  - `a ` apps only
  - `>` action mode (minimal shell action)
- Smooth open/close animations with settings for duration and optional disable.
- Fast incremental updates with debounce and caching.

## Super-quick local test (recommended)

From this repo root:

```bash
./scripts/install-local.sh
```

That script:
- syncs your working tree to `~/.local/share/gnome-shell/extensions/hop-launcher@example.org`
- compiles GSettings schemas
- disables + re-enables the extension

Then test instantly:
- press **Super+Space** to open
- type `chr` or `crome` and verify matching
- try `w ` for windows only and `a ` for apps only

## Fast dev loop

After any code change:

```bash
./scripts/install-local.sh
```

Watch GNOME Shell logs in another terminal:

```bash
journalctl --user -f /usr/bin/gnome-shell
```

Open preferences:

```bash
gnome-extensions prefs hop-launcher@example.org
```

## Packaging for release

Create a zip file in `dist/`:

```bash
./scripts/package-extension.sh
```

## Repository layout

- `metadata.json`
- `extension.js`
- `prefs.js`
- `style.css`
- `ui/launcherOverlay.js`
- `lib/fuzzy.js`
- `lib/providers/apps.js`
- `lib/providers/windows.js`
- `lib/providers/recents.js`
- `schemas/org.example.launcher.gschema.xml`
- `scripts/install-local.sh`
- `scripts/package-extension.sh`
- `tests/fuzzy.test.mjs`
- `docs/PERFORMANCE_SECURITY_REVIEW.md`

## Keybinding

- Default: `Super+Space` (GSettings key `toggle-launcher`)
- Configurable in extension preferences or GNOME Keyboard settings.

## Settings pane highlights

- Behavior: keybinding, blur/translucency, animations on/off
- Performance: debounce, max results, open/close animation durations
- Ranking: windows/apps/recents source weight controls

## Compatibility notes

- Designed for GNOME Shell **45+**.
- Uses GNOME Shell APIs for window listing/focus and avoids wlroots/KDE protocols.
- On non-GNOME compositors, this project is not expected to work.

## Performance/lifecycle notes

- Debounced search updates (`debounce-ms`, default 15ms).
- Provider results cached where sensible.
- `disable()` removes keybindings, disconnects signals, and destroys overlay actors/timeouts.

## Test fuzzy matcher

```bash
node --test tests/fuzzy.test.mjs
```


## Engineering review

A staff-level performance and security review is documented in `docs/PERFORMANCE_SECURITY_REVIEW.md`.
