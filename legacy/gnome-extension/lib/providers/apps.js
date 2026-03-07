import Shell from 'gi://Shell';
import {launchOrFocusApp} from './appLaunch.js';

function buildAppSignature(app) {
    const id = app.get_id?.() ?? '';
    const name = app.get_name?.() ?? '';
    const executable = app.get_app_info?.()?.get_executable?.() ?? '';
    return `${name.toLowerCase()}|${(executable || id).toLowerCase()}`;
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
        const seen = new Set();
        this._cache = [];

        for (const app of installed) {
            const signature = buildAppSignature(app);
            if (seen.has(signature))
                continue;
            seen.add(signature);

            this._cache.push({
                kind: 'app',
                id: app.get_id(),
                app,
                primaryText: app.get_name(),
                secondaryText: app.get_description() ?? 'Application',
                icon: app.get_icon(),
                execute: () => launchOrFocusApp(app),
            });
        }
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
