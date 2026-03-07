---
title: Runtime Commands Cheatsheet
description: High-signal commands for runtime status, diagnostics, and compositor-specific testing.
order: 2
---

## Core health

- `~/.local/bin/hop-hotkeyd status`
- `~/.local/bin/hop-hotkeyd doctor --wait-seconds 5 --interval-ms 200`
- `~/.local/bin/hop-hotkeyd doctor --strict`

## Shortcut management

- `~/.local/bin/hop-hotkeyd config get`
- `~/.local/bin/hop-hotkeyd config set --shortcut '<Super>space'`
- `~/.local/bin/hop-hotkeyd setup-shortcut`
- `~/.local/bin/hop-hotkeyd repair-shortcut`

## Compositor bridge probes

- Sway: `swaymsg -q -t send_tick hop-launcher-toggle`
- Hyprland: `hyprctl dispatch event hop-launcher-toggle`
- KDE Wayland: `qdbus org.kde.kglobalaccel /component/hoplauncher org.kde.kglobalaccel.Component.invokeShortcut hop-launcher-toggle`
- GNOME Wayland: `gdbus emit --session --object-path /io/github/hop/Hotkeyd --signal io.github.hop.Hotkeyd.Toggle hop-launcher-toggle`
