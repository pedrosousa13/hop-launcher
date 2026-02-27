import Meta from 'gi://Meta';
import Shell from 'gi://Shell';

function getWorkspaceLabel(metaWindow) {
    const ws = metaWindow.get_workspace();
    if (!ws)
        return 'No workspace';
    return `Workspace ${ws.index() + 1}`;
}

export class WindowsProvider {
    getResults() {
        const tracker = Shell.WindowTracker.get_default();
        const windows = global.display.get_tab_list(Meta.TabList.NORMAL_ALL, null);

        return windows
            .filter(w => !w.skip_taskbar && !w.minimized)
            .map(metaWindow => {
                const app = tracker.get_window_app(metaWindow);
                const title = metaWindow.get_title() ?? 'Untitled window';
                const appName = app?.get_name() ?? 'Unknown app';
                const workspaceLabel = getWorkspaceLabel(metaWindow);

                return {
                    kind: 'window',
                    id: `window:${metaWindow.get_id()}`,
                    metaWindow,
                    primaryText: title,
                    secondaryText: `${appName} • ${workspaceLabel}`,
                    icon: app?.get_app_info()?.get_icon() ?? null,
                    execute: () => {
                        metaWindow.activate(global.get_current_time());
                    },
                    actions: [
                        {
                            id: 'focus',
                            label: 'Focus',
                            run: () => metaWindow.activate(global.get_current_time()),
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
                                const manager = global.workspace_manager;
                                const nextIndex = Math.min(current.index() + 1, manager.n_workspaces - 1);
                                const next = manager.get_workspace_by_index(nextIndex);
                                if (next)
                                    metaWindow.change_workspace(next);
                            },
                        },
                    ],
                };
            });
    }
}
