import Meta from 'gi://Meta';
import Shell from 'gi://Shell';

import * as Main from 'resource:///org/gnome/shell/ui/main.js';
import {Extension} from 'resource:///org/gnome/shell/extensions/extension.js';

import {LauncherOverlay} from './ui/launcherOverlay.js';
import {AppsProvider} from './lib/providers/apps.js';
import {WindowsProvider} from './lib/providers/windows.js';
import {RecentsProvider} from './lib/providers/recents.js';
import {CalculatorProvider} from './lib/providers/calculator.js';
import {CurrencyProvider} from './lib/providers/currency.js';
import {TimezoneProvider} from './lib/providers/timezone.js';
import {EmojiProvider} from './lib/providers/emoji.js';
import {FilesProvider} from './lib/providers/files.js';

const KEY_TOGGLE = 'toggle-launcher';

export default class HopLauncherExtension extends Extension {
    enable() {
        this._settings = this.getSettings('org.example.launcher');
        this._providers = [
            new WindowsProvider(),
            new AppsProvider(),
            new RecentsProvider(),
            new FilesProvider(this._settings),
            new EmojiProvider(),
            new CalculatorProvider(),
            new TimezoneProvider(),
            new CurrencyProvider(this._settings),
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

        for (const provider of this._providers) {
            try {
                provider.destroy?.();
            } catch (error) {
                logError(error, '[hop-launcher] provider destroy failed');
            }
        }

        this._providers = [];
        this._settings = null;
    }

    _toggle() {
        if (!this._overlay)
            return;

        if (this._overlay.visible)
            this._overlay.close();
        else {
            // Recompute geometry at open time to keep overlay centered across dynamic layout changes.
            this._positionOverlay();
            this._overlay.open();
        }
    }

    _positionOverlay() {
        if (!this._overlay)
            return;

        const monitorIndex = Main.layoutManager.primaryIndex ?? global.display.get_primary_monitor();
        const monitor = Main.layoutManager.monitors?.[monitorIndex] ?? Main.layoutManager.primaryMonitor;
        if (!monitor)
            return;

        const workArea = Main.layoutManager.getWorkAreaForMonitor(monitorIndex);
        const width = Math.min(700, Math.floor(workArea.width * 0.8));
        const maxResultsHeight = Math.floor(workArea.height * 0.55);
        const topOffset = Math.floor(workArea.height * 0.18);

        this._overlay.set_width(width);
        this._overlay.set_height(-1);
        this._overlay.setMaxResultsHeight(maxResultsHeight);
        this._overlay.set_position(
            workArea.x + Math.floor((workArea.width - width) / 2),
            workArea.y + topOffset
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
