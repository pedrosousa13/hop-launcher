# Cross-Frontend Parity Matrix (Locked)

Date: 2026-03-05
Scope: Linux-native Hop Launcher surfaces (`hopd`, GTK launcher, GNOME extension convergence, KDE adapter helper)

## Providers and Query Modes

| Capability | hopd mode/provider | GTK launcher | GNOME extension | KDE adapter helper |
| --- | --- | --- | --- | --- |
| Apps | `mode=apps`, kind=`app` | Supported via routed query (`a ...`) | Supported (native + convergence path) | Supported via `--mode apps` |
| Windows | `mode=windows`, kind=`window` | Supported via routed query (`w ...`) | Supported (native + convergence path) | Supported via `--mode windows` |
| Files | `mode=files`, kind=`file` | Supported via routed query (`f ...`) | Supported (native + convergence path) | Supported via `--mode files` |
| Recents | `mode=recents`, kind=`recent` | Supported via routed query (`r ...`) | Supported (native + convergence path) | Supported via `--mode recents` |
| Settings | `mode=settings`, kind=`setting` | Supported (`settings ...`/`prefs ...`) | Supported (native + convergence path) | Supported via `--mode settings` |
| Calculator | `mode=calculator` or all-mode utility intent, kind=`calculator` | Supported (`calc ...` or expression) | Supported | Supported via `--mode calculator` |
| Currency | `mode=currency` or all-mode utility intent, kind=`currency` | Supported (`currency ...`/`fx ...`) | Supported | Supported via `--mode currency` |
| Weather | `mode=weather` or all-mode utility intent, kind=`weather` | Supported (`weather ...`, `<city> weather`) | Supported | Supported via `--mode weather` |
| Timezone | `mode=timezone` or all-mode utility intent, kind=`timezone` | Supported (`time ...`, `tz ...`, `<city> time`) | Supported | Supported via `--mode timezone` |
| Emoji | `mode=emoji` or all-mode utility intent, kind=`emoji` | Supported (`emoji ...`, `:...`) | Supported | Supported via `--mode emoji` |

## Action Behavior

| Action path | hopd contract | GTK launcher | GNOME extension | KDE adapter helper |
| --- | --- | --- | --- | --- |
| Enter execute | `actions.execute` with `result_id`, `action=enter` | Enter triggers execute | Enter triggers execute/copy path | `--execute <result_id>` |
| Execute metadata | returns `action_resolved`, `resolved_command`, `resolved_args`, `execution_status`, `success` | Consumed for status/errors | Consumed for action feedback | Available in JSON output |
| Utility/copy-like rows | utility IDs resolve deterministically (calculator/currency/weather/timezone/emoji) | Enter behavior mapped by kind | Enter behavior mapped by kind | Command metadata exposed for runner integration |

## Validation Gates

1. `crates/hopd/tests/providers_parity.rs` must assert mode-to-kind parity including utility modes.
2. `crates/hopd/tests/ipc_contract.rs` must assert utility intent rows and action-resolution metadata shape.
3. Frontend suites must continue passing:
   - `apps/gtk-launcher` (`--features gtk_ui`)
   - `apps/gnome-extension` (`npm test`)
