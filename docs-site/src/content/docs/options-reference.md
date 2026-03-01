---
title: Options Reference
description: What each launcher setting controls.
order: 5
---

## Behavior

- `toggle-launcher`: keybinding list for opening/closing launcher.
- `blur-enabled`: enables blur-like overlay styling.
- `animations-enabled`: enables open/close animations.
- `open-animation-ms`: open animation duration.
- `close-animation-ms`: close animation duration.

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

- `web-search-enabled`: enable appended web search actions.
- `web-search-max-actions`: cap appended actions per query.
- `web-search-services-json`: provider templates and keywords.
