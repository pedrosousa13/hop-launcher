import Meta from 'gi://Meta';
import Shell from 'gi://Shell';

import * as Main from 'resource:///org/gnome/shell/ui/main.js';
import {Extension} from 'resource:///org/gnome/shell/extensions/extension.js';

import {LauncherOverlay} from './ui/launcherOverlay.js';
import {AppsProvider} from './lib/providers/apps.js';
import {WindowsProvider} from './lib/providers/windows.js';
import {RecentsProvider} from './lib/providers/recents.js';

const KEY_TOGGLE = 'toggle-launcher';

export default class HopLauncherExtension extends Extension {
    enable() {
        this._settings = this.getSettings('org.example.launcher');
        this._providers = [
            new WindowsProvider(),
            new AppsProvider(),
            new RecentsProvider(),
        ];

        this._overlay = new LauncherOverlay(this._settings, this._providers);
        Main.layoutManager.addChrome(this._overlay);
        this._positionOverlay();

        this._monitorsChangedId = Main.layoutManager.connect('monitors-changed', () => this._positionOverlay());

        Main.wm.addKeybinding(
            KEY_TOGGLE,
            this._settings,
            Meta.KeyBindingFlags.NONE,
            Shell.ActionMode.ALL,
            () => this._toggle()
        );

        this._applyBlur();
        this._blurChangedId = this._settings.connect('changed::blur-enabled', () => this._applyBlur());
    }

    disable() {
        if (this._blurChangedId) {
            this._settings.disconnect(this._blurChangedId);
            this._blurChangedId = null;
        }

        if (this._monitorsChangedId) {
            Main.layoutManager.disconnect(this._monitorsChangedId);
            this._monitorsChangedId = null;
        }

        Main.wm.removeKeybinding(KEY_TOGGLE);

        if (this._overlay) {
            this._overlay.destroyOverlay();
            this._overlay = null;
        }

        this._providers = [];
        this._settings = null;
    }

    _toggle() {
        if (!this._overlay)
            return;

        if (this._overlay.visible)
            this._overlay.close();
        else
            this._overlay.open();
    }

    _positionOverlay() {
        const monitor = Main.layoutManager.primaryMonitor;
        if (!monitor || !this._overlay)
            return;

        const width = Math.min(700, Math.floor(monitor.width * 0.8));
        const height = Math.floor(monitor.height * 0.6);

        this._overlay.set_size(width, height);
        this._overlay.set_position(
            monitor.x + Math.floor((monitor.width - width) / 2),
            monitor.y + Math.floor((monitor.height - height) / 5)
        );
    }

    _applyBlur() {
        if (!this._overlay)
            return;

        if (this._settings.get_boolean('blur-enabled'))
            this._overlay.add_style_class_name('blurred');
        else
            this._overlay.remove_style_class_name('blurred');
    }
}
