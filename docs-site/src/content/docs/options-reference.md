---
title: Operational Reference
description: Runtime commands, control sockets, and where each control surface lives.
order: 5
---

## Services and binaries

- `~/.local/bin/hopd`
- `~/.local/bin/hop-hotkeyd`
- GTK client under `apps/gtk-launcher`

## Control commands

- `hop-hotkeyd status [--socket /path/to.sock]`
- `hop-hotkeyd doctor [--strict]`
- `hop-hotkeyd config get`
- `hop-hotkeyd config set --shortcut '<accelerator>'`
- `hop-hotkeyd print-bindings [--socket /path/to.sock]`

## Runtime docs map

- install + local testing: `docs/HOPD_LOCAL_INSTALL.md`
- status/doctor schema: `docs/HOTKEYD_STATUS_SCHEMA.md`
- release process: `docs/RELEASE.md`
- linux packaging: `docs/LINUX_PACKAGING.md`
