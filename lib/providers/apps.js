import Shell from 'gi://Shell';

function focusExistingAppWindow(app) {
    const windows = app.get_windows?.() ?? [];
    const candidate = windows.find(window =>
        window &&
        !window.skip_taskbar &&
        !window.minimized &&
        !window.is_override_redirect?.()
    );

    if (!candidate)
        return false;

    candidate.activate(global.get_current_time());
    return true;
}

export class AppsProvider {
    constructor() {
        this._appSystem = Shell.AppSystem.get_default();
        this._cache = [];
        this.refreshOnOpen = true;
        this._installedChangedId = this._appSystem.connect('installed-changed', () => this.refresh());
        this.refresh();
    }

    refresh() {
        const installed = this._appSystem.get_installed();
        this._cache = installed.map(app => ({
            kind: 'app',
            id: app.get_id(),
            app,
            primaryText: app.get_name(),
            secondaryText: app.get_description() ?? 'Application',
            icon: app.get_icon(),
            execute: () => {
                if (!focusExistingAppWindow(app))
                    app.activate();
            },
        }));
    }

    getResults() {
        if (this._cache.length === 0)
            this.refresh();
        return this._cache;
    }

    destroy() {
        if (this._installedChangedId) {
            this._appSystem.disconnect(this._installedChangedId);
            this._installedChangedId = null;
        }
    }
}
