---
title: Shortcut Setup
description: Configure launcher toggles through hop-hotkeyd.
order: 6
---

## Configure the shortcut

```bash
~/.local/bin/hop-hotkeyd config set --shortcut '<Super>space'
```

Verify what is currently active:

```bash
~/.local/bin/hop-hotkeyd config get
```

## Apply native wiring

```bash
~/.local/bin/hop-hotkeyd setup-shortcut
~/.local/bin/hop-hotkeyd repair-shortcut
```

Use `status` and `doctor --strict` after setup to ensure compositor bridge and control socket health are both green.
