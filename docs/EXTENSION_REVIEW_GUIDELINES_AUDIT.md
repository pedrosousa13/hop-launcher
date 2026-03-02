# GNOME Extensions Review Guidelines Audit (Hop Launcher)

Date: 2026-03-02

> Note: The environment could not fetch `https://gjs.guide/extensions/review-guidelines/review-guidelines.html` directly (HTTP tunnel 403). This audit is a best-effort review against the commonly enforced GNOME Extensions review requirements that the page documents.

## Verdict

**Partially aligned**. The repository is close on packaging, metadata, lifecycle cleanup, and avoiding dangerous APIs, but has review-risk gaps around localization and explicit user-facing disclosure for network-backed features.

## Checklist

### 1) Metadata and packaging basics — **PASS**

- Extension metadata includes UUID, name, description, version, shell compatibility, settings schema, and URL.  
- Packaging script ships expected extension assets and excludes test/dev artifacts.

Evidence:
- `metadata.json` fields are present and populated.  
- `scripts/package-extension.sh` zips extension runtime files and excludes `dist`, `scripts`, `tests`, and VCS artifacts.

### 2) No minified/obfuscated code, no dangerous dynamic execution — **PASS**

- Source is readable, non-minified JS modules.
- No usage of `eval`, `Function(...)`, or shell command execution from extension runtime paths.

### 3) Extension lifecycle and shell integration hygiene — **PASS**

- Keybinding is registered on enable and removed on disable.
- Signal handlers and providers are disconnected/destroyed during disable.

Evidence:
- `Main.wm.addKeybinding(...)` in `enable()` and `Main.wm.removeKeybinding(...)` in `disable()`.
- Overlay and provider cleanup is performed in `disable()`.

### 4) User data/network behavior clarity — **PARTIAL (review risk)**

- Weather provider performs network requests to Open-Meteo endpoints.
- Web search provider launches user queries to external search URLs.
- Provider toggles exist in settings schema, which is good.

Risk:
- Review often expects clear, explicit user-facing disclosure for network features and what data leaves the machine.

Evidence:
- Weather URLs and request path are defined in `lib/providers/weather.js`.
- Web search URL-template launch behavior is implemented in `lib/providers/webSearch.js`.
- Feature toggles exist for weather/web search in schema.

### 5) Internationalization (i18n/gettext) — **GAP (likely blocker)**

- User-facing strings in extension and preferences are hard-coded English.
- No gettext wiring (`gettext`, `_()`, `initTranslations`) was found.

Risk:
- Lack of translatable UI strings is commonly flagged during review.

### 6) Settings schema and prefs safety — **PASS**

- GSettings schema is structured and includes summaries/descriptions.
- Preferences validate accelerator input rather than writing arbitrary values.

## Remaining follow-ups

- Add full gettext-based localization plumbing and wrap user-visible strings across extension UI/providers/prefs.
- Ensure extensions.gnome.org listing text mirrors the network/privacy disclosure now present in README and Preferences.
- Re-run release checks before packaging:
  - `glib-compile-schemas --strict --dry-run schemas`
  - `npm test`

## Commands used for this audit

- `rg --files`
- `cat metadata.json`
- `sed -n '1,260p' extension.js`
- `sed -n '1,320p' lib/providers/weather.js`
- `sed -n '1,220p' lib/providers/webSearch.js`
- `sed -n '1,260p' schemas/org.hoplauncher.app.gschema.xml`
- `sed -n '1,220p' scripts/package-extension.sh`
- `rg -n "\\beval\\b|Function\\(|spawn_command_line|spawn\\(|child_process|imports\\.ui|byteArray|unsafe_mode|chmod|curl|wget|fetch\\(|Soup\\.Session|GLib\\.file_set_contents|Gio\\.File\\.new_for_path|Main\\.wm\\.addKeybinding|global\\.stage" extension.js lib prefs.js ui scripts metadata.json README.md`
- `rg -n "gettext|\\b_\\(|initTranslations|domain" extension.js prefs.js lib ui`
