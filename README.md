# Hop Launcher (GNOME Wayland)

A polished command palette for GNOME Shell 45+ on Wayland.

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
- Prefix filters and inferred utility intents:
  - `w ` windows only
  - `a ` apps only
  - `f ` files only
  - `:emoji ` or `emoji ` emoji picker
  - `tz ` / `timezone ` / `time in <city>` / `<city> time` / bare tokens like `pst`, `tokyo`, or `zurich` for timezone lookup
  - `$100 usd to eur`, `100 usd to eur`, or `100usd to eur` currency conversion (supports common codes like `CHF`)
  - `weather <location>`, `wx <location>`, or `<location> weather` for current weather (`Open-Meteo` icons)
  - `=2+2` calculator
  - `>` action mode (minimal shell action)
- Smooth open/close animations with settings for duration and optional disable.
- Fast incremental updates with debounce and caching.
- Configurable web search actions (Google/DuckDuckGo defaults), appended at the end of non-empty result lists.

## Super-quick local test (recommended)

From this repo root:

```bash
./scripts/install-local.sh
```

That script:
- syncs your working tree to `~/.local/share/gnome-shell/extensions/hop-launcher@hoplauncher.app`
- compiles GSettings schemas
- disables + re-enables the extension

Then test instantly:
- press **Super+Space** to open
- type `chr` or `crome` and verify matching
- try `w ` for windows only and `a ` for apps only
- try `f report`, `emoji smile`, `time in zurich`, `zurich time`, `weather berlin`, `zurich weather`, `wx 94103`, `pst`, `100eur to chf`, `100usd to eur`, and `2+2`
- type any non-empty query and confirm trailing `Search <provider>` actions appear at the end

## Web search providers (URL templates)

`web-search-services-json` accepts an array of provider objects. Each entry uses a URL template with `%s` placeholder:

```json
[
  {
    "id": "google",
    "name": "Google",
    "urlTemplate": "https://www.google.com/search?q=%s",
    "enabled": true
  },
  {
    "id": "duckduckgo",
    "name": "DuckDuckGo",
    "urlTemplate": "https://duckduckgo.com/?q=%s",
    "enabled": true
  },
  {
    "id": "kagi",
    "name": "Kagi",
    "urlTemplate": "https://kagi.com/search?q=%s",
    "enabled": true
  }
]
```

Rules:
- Use `https://` URLs only.
- Include `%s` exactly where the query should be inserted.
- Queries are URL-encoded automatically.

## Fast dev loop

After any code change:

```bash
./scripts/install-local.sh
```

If you suspect GNOME cached stale extension modules, force reload without full logout:

```bash
./scripts/reload-shell.sh
```

Watch GNOME Shell logs in another terminal:

```bash
journalctl --user -f /usr/bin/gnome-shell
```

Open preferences:

```bash
gnome-extensions prefs hop-launcher@hoplauncher.app
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
- `stylesheet.css`
- `ui/launcherOverlay.js`
- `lib/fuzzy.js`
- `lib/providers/apps.js`
- `lib/providers/windows.js`
- `lib/providers/recents.js`
- `schemas/org.hoplauncher.app.gschema.xml`
- `scripts/install-local.sh`
- `scripts/package-extension.sh`
- `tests/fuzzy.test.mjs`
- `docs/PERFORMANCE_SECURITY_REVIEW.md`

## Keybinding

- Default: `Super+Space` (GSettings key `toggle-launcher`)
- Configurable in extension preferences or GNOME Keyboard settings.

## Settings pane highlights

- Behavior: keybinding, blur style, launcher-wide translucency, animations on/off
- Main features: one-click enable/disable for windows, apps, files, emoji, calculator, currency, timezone, weather, and web search
- Performance: debounce, max results, open/close animation durations
- Ranking: windows/apps/recents source weight controls
- Ranking: windows/apps/recents/files/emoji/utility source weight controls
- Smart providers: indexed folders and currency cache controls
- Web search providers: add/edit/reorder/remove providers, reset defaults, max actions, and URL-template config

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

## CI checks (GitHub Actions)

A CI workflow is included at `.github/workflows/ci.yml` and runs on push/PR:
- `npm test` (fuzzy matcher tests)
- `glib-compile-schemas --strict --dry-run schemas`
- `bash -n scripts/install-local.sh scripts/reload-shell.sh scripts/package-extension.sh`
