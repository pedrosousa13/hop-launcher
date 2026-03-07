---
title: Getting Started
description: Local setup and first-run checks for hopd, hop-hotkeyd, and GTK launcher.
order: 1
---

## Components

The current stack is split into three runtime parts:

- `hopd` daemon (`crates/hopd`)
- `hop-hotkeyd` shortcut/control bridge (`crates/hop-hotkeyd`)
- GTK launcher client (`apps/gtk-launcher`)

## Local manual run

From repository root:

```bash
./scripts/run-gtk-manual-test.sh
```

This installs/updates local user services, verifies daemon socket health, and launches the GTK UI.

## Quick health checks

```bash
~/.local/bin/hop-hotkeyd status
~/.local/bin/hop-hotkeyd doctor --strict
```

Use these before debugging frontend issues so transport and shortcut wiring are validated first.
