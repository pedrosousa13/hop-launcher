---
title: Options Reference
description: What each launcher setting controls.
order: 5
---

## Behavior

- `toggle-launcher`: keybinding list for opening/closing launcher.
- `blur-enabled`: enables blur-like overlay styling.
- `overlay-translucency`: launcher-wide translucency percentage for overlay surfaces.
- `animations-enabled`: enables open/close animations.
- `open-animation-ms`: open animation duration.
- `close-animation-ms`: close animation duration.

## Main features

- `feature-windows-enabled`: include open windows provider results.
- `feature-apps-enabled`: include installed apps provider results.
- `feature-files-enabled`: include indexed/recent files provider results.
- `feature-emoji-enabled`: include emoji provider results.
- `feature-calculator-enabled`: include calculator provider results.
- `feature-currency-enabled`: include currency provider results.
- `feature-timezone-enabled`: include timezone provider results.
- `feature-weather-enabled`: include weather provider results.
- `feature-web-search-enabled`: include web search provider results.

## Search and ranking

- `max-results`: maximum displayed results.
- `debounce-ms`: delay before recalculating search.
- `min-fuzzy-score`: minimum fuzzy score threshold.
- `weight-windows`: ranking boost for windows.
- `weight-apps`: ranking boost for apps.
- `weight-recents`: ranking boost for recents.
- `weight-files`: ranking boost for files.
- `weight-emoji`: ranking boost for emoji.
- `weight-utility`: ranking boost for utility results.

## Smart providers and data

- `indexed-folders`: folders used for file indexer.
- `currency-refresh-enabled`: allow rate refresh.
- `currency-rate-ttl-hours`: exchange-rate cache lifetime.
- `custom-aliases-json`: alias rewrite/target config.

## Learning

- `learning-enabled`: enables interaction-based ranking boosts.
- `launch-learning-json`: stored query-app usage map.
- `learning-insights-limit`: max insights rows in preferences.
- `learning-insights-sort`: sort mode (`count` or `recent`).

## Web search actions

- `web-search-enabled`: global web action switch.
- `web-search-max-actions`: cap appended actions per query.
- `web-search-services-json`: provider templates and keywords.
