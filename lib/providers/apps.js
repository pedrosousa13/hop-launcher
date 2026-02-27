import Shell from 'gi://Shell';

export class AppsProvider {
    constructor() {
        this._appSystem = Shell.AppSystem.get_default();
        this._cache = [];
    }

    refresh() {
        this._cache = this._appSystem.get_installed().map(app => ({
            kind: 'app',
            id: app.get_id(),
            app,
            primaryText: app.get_name(),
            secondaryText: app.get_description() ?? 'Application',
            icon: app.get_icon(),
            execute: () => app.activate(),
        }));
    }

    getResults() {
        if (this._cache.length === 0)
            this.refresh();
        return this._cache;
    }
}
