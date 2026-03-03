import Meta from 'gi://Meta';
import Shell from 'gi://Shell';
import Gio from 'gi://Gio';
import GLib from 'gi://GLib';
import Soup from 'gi://Soup?version=3.0';

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
import {WeatherProvider} from './lib/providers/weather.js';
import {WebSearchProvider} from './lib/providers/webSearch.js';
import {HopdProvider} from './lib/providers/hopd.js';
import {makeSettingsGatedProvider} from './lib/providerToggleWrapper.js';
import {buildProviderFeatureMap} from './lib/providerFeatureMap.js';

const KEY_TOGGLE = 'toggle-launcher';

// Promisification mutates GJS prototypes globally, so keep it at module scope and do it once.
Gio._promisify(Soup.Session.prototype, 'send_and_read_async', 'send_and_read_finish');

function createSoupRequestJson(session, cancellable, isDestroyed) {
    return async url => {
        if (isDestroyed())
            throw new Error('weather canceled');

        const message = Soup.Message.new('GET', url);
        const bytes = await session.send_and_read_async(
            message,
            GLib.PRIORITY_DEFAULT,
            cancellable
        );
        if (isDestroyed())
            throw new Error('weather canceled');

        const status = Number(message.status_code ?? message.get_status?.());
        if (!Number.isFinite(status) || status < 200 || status >= 300)
            throw new Error(`weather http ${status}`);

        const payload = bytes.get_data();
        const json = typeof payload === 'string'
            ? payload
            : new TextDecoder().decode(payload);
        return JSON.parse(json);
    };
}

function resolveHopdSocketPath() {
    const explicitPath = (GLib.getenv('HOPD_SOCKET') ?? '').trim();
    if (explicitPath)
        return explicitPath;

    const runtimeDir = (GLib.getenv('XDG_RUNTIME_DIR') ?? '').trim();
    if (runtimeDir)
        return GLib.build_filenamev([runtimeDir, 'hopd.sock']);

    return '/tmp/hopd.sock';
}

function createHopdRequestIpc(socketPath, isDestroyed) {
    return async payload => {
        if (isDestroyed())
            throw new Error('hopd canceled');

        const client = new Gio.SocketClient();
        const address = Gio.UnixSocketAddress.new(socketPath);
        const connection = client.connect(address, null);
        try {
            const request = JSON.stringify(payload);
            const output = connection.get_output_stream();
            output.write_all(new TextEncoder().encode(`${request}\n`), null);

            const input = new Gio.DataInputStream({
                base_stream: connection.get_input_stream(),
            });
            const [line] = input.read_line_utf8(null);
            if (!line)
                throw new Error('hopd empty response');
            return JSON.parse(line);
        } finally {
            try {
                connection.close(null);
            } catch (_) {
                // ignore close failures
            }
        }
    };
}

export default class HopLauncherExtension extends Extension {
    enable() {
        this._destroyed = false;
        this._requestCancellable = new Gio.Cancellable();
        this._soupSession = new Soup.Session({
            user_agent: 'hop-launcher/1.0',
        });

        this._settings = this.getSettings('org.gnome.shell.extensions.hop-launcher');
        const openUrl = url => {
            try {
                Gio.AppInfo.launch_default_for_uri(url, null);
            } catch (error) {
                logError(error, '[hop-launcher] open url failed');
            }
        };
        const providers = buildProviderFeatureMap({
            windows: new WindowsProvider(),
            apps: new AppsProvider(),
            recents: new RecentsProvider(),
            files: new FilesProvider(this._settings),
            emoji: new EmojiProvider(),
            calculator: new CalculatorProvider(),
            timezone: new TimezoneProvider(),
            currency: new CurrencyProvider(this._settings),
            weather: new WeatherProvider({
                requestJson: createSoupRequestJson(
                    this._soupSession,
                    this._requestCancellable,
                    () => this._destroyed
                ),
            }),
            webSearch: new WebSearchProvider(this._settings, {openUrl}),
            hopd: new HopdProvider({
                requestIpc: createHopdRequestIpc(resolveHopdSocketPath(), () => this._destroyed),
                limit: this._settings.get_int('max-results'),
            }),
        });
        this._providers = providers.map(([provider, key]) =>
            makeSettingsGatedProvider(provider, this._settings, key)
        );

        this._overlay = new LauncherOverlay(this._settings, this._providers, this.path);
        Main.layoutManager.addChrome(this._overlay);
        this._positionOverlay();

        this._monitorsChangedId = Main.layoutManager.connect('monitors-changed', () => this._positionOverlay());

        Main.wm.addKeybinding(
            KEY_TOGGLE,
            this._settings,
            Meta.KeyBindingFlags.NONE,
            Shell.ActionMode.NORMAL | Shell.ActionMode.OVERVIEW,
            () => this._toggle()
        );

        this._applyOverlayVisuals();
        this._overlayVisualSettingIds = [
            this._settings.connect('changed::blur-enabled', () => this._applyOverlayVisuals()),
            this._settings.connect('changed::overlay-translucency', () => this._applyOverlayVisuals()),
        ];
    }

    disable() {
        this._destroyed = true;
        this._requestCancellable?.cancel();
        this._soupSession?.abort();

        if (this._overlayVisualSettingIds?.length) {
            for (const id of this._overlayVisualSettingIds)
                this._settings.disconnect(id);
            this._overlayVisualSettingIds = [];
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

        for (const provider of this._providers ?? []) {
            try {
                provider.destroy?.();
            } catch (error) {
                logError(error, '[hop-launcher] provider destroy failed');
            }
        }

        this._providers = [];
        this._requestCancellable = null;
        this._soupSession = null;
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

    _applyOverlayVisuals() {
        if (!this._overlay)
            return;

        const blurEnabled = this._settings.get_boolean('blur-enabled');
        if (blurEnabled)
            this._overlay.add_style_class_name('blurred');
        else
            this._overlay.remove_style_class_name('blurred');

        this._overlay.applyVisualSettings({
            blurEnabled,
            translucencyPercent: this._settings.get_int('overlay-translucency'),
        });
    }
}
