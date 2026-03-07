---
title: Status and Diagnostics Schema
description: What runtime status fields and diagnostics are expected to report.
order: 3
---

## `status` output focus

`hop-hotkeyd status` should report:

- backend mode and readiness
- control socket state
- native backend readiness (`native_backend_ready`, `native_backend_socket`, `native_backend_error`)
- `recommended_binding` guidance
- `capability_matrix` and `shortcut_drift`

## `doctor` usage

Use `doctor` during installation validation and release checks:

- normal mode for operator context
- `--strict` mode for CI and pre-release verification gates

See repository docs for full payload schema details:

- `docs/HOTKEYD_STATUS_SCHEMA.md`
