import Meta from 'gi://Meta';
import Shell from 'gi://Shell';

function getWorkspaceLabel(metaWindow) {
    const ws = metaWindow.get_workspace();
    if (!ws)
        return 'All workspaces';
    return `Workspace ${ws.index() + 1}`;
}

function safeActivateWindow(metaWindow) {
    if (!metaWindow || metaWindow.is_override_redirect())
        return;
    metaWindow.activate(global.get_current_time());
}

export class WindowsProvider {
    constructor() {
        this.refreshOnOpen = true;
        this._cache = [];
    }

    refresh() {
        const tracker = Shell.WindowTracker.get_default();
        const windows = global.display.get_tab_list(Meta.TabList.NORMAL_ALL, null);
        this._cache = windows
            .filter(w => !w.skip_taskbar && !w.minimized && !w.is_override_redirect())
            .map(metaWindow => {
                const app = tracker.get_window_app(metaWindow);
                const title = metaWindow.get_title() ?? 'Untitled window';
                const appName = app?.get_name() ?? 'Unknown app';
                const workspaceLabel = getWorkspaceLabel(metaWindow);

                return {
                    kind: 'window',
                    id: `window:${metaWindow.get_id()}`,
                    metaWindow,
                    windowAppId: app?.get_id?.() ?? '',
                    primaryText: title,
                    secondaryText: `${appName} • ${workspaceLabel}`,
                    icon: app?.get_app_info()?.get_icon() ?? null,
                    execute: () => safeActivateWindow(metaWindow),
                    actions: [
                        {
                            id: 'focus',
                            label: 'Focus',
                            run: () => safeActivateWindow(metaWindow),
                        },
                        {
                            id: 'close',
                            label: 'Close',
                            run: () => metaWindow.delete(global.get_current_time()),
                        },
                        {
                            id: 'move-next-workspace',
                            label: 'Move to next workspace',
                            run: () => {
                                const current = metaWindow.get_workspace();
                                if (!current)
                                    return;

                                const manager = global.workspace_manager;
                                const nextIndex = Math.min(current.index() + 1, manager.n_workspaces - 1);
                                const next = manager.get_workspace_by_index(nextIndex);
                                if (next && next !== current)
                                    metaWindow.change_workspace(next);
                            },
                        },
                    ],
                };
            });
    }

    getResults() {
        this.refresh();
        return this._cache;
    }
}
