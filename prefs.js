import Adw from 'gi://Adw';
import Gdk from 'gi://Gdk';
import Gio from 'gi://Gio';
import Gtk from 'gi://Gtk';

import {ExtensionPreferences} from 'resource:///org/gnome/Shell/Extensions/js/extensions/prefs.js';
import {
    interpretKeybindingPress,
    resolveTypedAccelerator,
    sanitizeModifierState,
} from './lib/keybindingCapture.js';


function isValidAccelerator(accel) {
    const [keyval, mods] = Gtk.accelerator_parse(accel);
    return keyval !== 0 && mods !== 0;
}

const MODIFIER_KEYVALS = new Set([
    Gdk.KEY_Shift_L,
    Gdk.KEY_Shift_R,
    Gdk.KEY_Control_L,
    Gdk.KEY_Control_R,
    Gdk.KEY_Alt_L,
    Gdk.KEY_Alt_R,
    Gdk.KEY_Meta_L,
    Gdk.KEY_Meta_R,
    Gdk.KEY_Super_L,
    Gdk.KEY_Super_R,
    Gdk.KEY_Hyper_L,
    Gdk.KEY_Hyper_R,
    Gdk.KEY_ISO_Level3_Shift,
    Gdk.KEY_ISO_Level5_Shift,
]);

function addSpinRow(group, settings, key, title, subtitle, lower, upper, step = 1, page = 5) {
    const row = new Adw.SpinRow({
        title,
        subtitle,
        adjustment: new Gtk.Adjustment({
            lower,
            upper,
            step_increment: step,
            page_increment: page,
            value: settings.get_int(key),
        }),
    });
    settings.bind(key, row, 'value', Gio.SettingsBindFlags.DEFAULT);
    group.add(row);
}

function addStringArrayRow(group, settings, key, title, subtitle) {
    const row = new Adw.EntryRow({
        title,
        text: settings.get_strv(key).join(', '),
    });
    row.set_tooltip_text(subtitle);
    row.connect('changed', entry => {
        const values = entry.text
            .split(',')
            .map(v => v.trim())
            .filter(Boolean);
        settings.set_strv(key, values);
    });
    group.add(row);
}

export default class HopLauncherPreferences extends ExtensionPreferences {
    fillPreferencesWindow(window) {
        const settings = this.getSettings('org.example.launcher');

        const page = new Adw.PreferencesPage({title: 'Hop Launcher'});

        const behaviorGroup = new Adw.PreferencesGroup({
            title: 'Behavior',
            description: 'Core launcher interactions and visual behavior.',
        });

        const keybindRow = new Adw.EntryRow({
            title: 'Toggle keybinding',
            text: settings.get_strv('toggle-launcher').join(', '),
            editable: false,
        });
        keybindRow.set_tooltip_text('Click Capture, then press a key combo. Backspace/Delete clears.');

        const captureButton = new Gtk.Button({label: 'Capture'});
        captureButton.set_valign(Gtk.Align.CENTER);
        captureButton.connect('clicked', () => {
            const dialog = new Adw.MessageDialog({
                transient_for: window,
                modal: true,
                heading: 'Capture Shortcut',
                body: 'Press your key combo now. Escape cancels, Backspace/Delete clears.',
            });
            dialog.add_response('cancel', 'Cancel');
            dialog.set_default_response('cancel');
            dialog.set_close_response('cancel');

            const keyController = new Gtk.EventControllerKey();
            keyController.set_propagation_phase(Gtk.PropagationPhase.CAPTURE);
            keyController.connect('key-pressed', (_controller, keyval, _keycode, state) => {
                const mods = sanitizeModifierState(state, Gtk.accelerator_get_default_mod_mask());
                const action = interpretKeybindingPress({
                    keyval,
                    mods,
                    keyNames: {
                        escape: Gdk.KEY_Escape,
                        backSpace: Gdk.KEY_BackSpace,
                        delete: Gdk.KEY_Delete,
                        modifiers: MODIFIER_KEYVALS,
                    },
                });

                if (action.kind === 'cancel') {
                    dialog.close();
                    return true;
                }

                if (action.kind === 'clear') {
                    settings.set_strv('toggle-launcher', []);
                    dialog.close();
                    return true;
                }

                if (action.kind === 'ignore')
                    return true;

                const accel = Gtk.accelerator_name(action.keyval, action.mods);
                if (!isValidAccelerator(accel))
                    return true;

                settings.set_strv('toggle-launcher', [accel]);
                dialog.close();
                return true;
            });
            dialog.add_controller(keyController);
            dialog.present();
        });
        keybindRow.add_suffix(captureButton);

        settings.connect('changed::toggle-launcher', () => {
            const accel = resolveTypedAccelerator(
                settings.get_strv('toggle-launcher').join(', '),
                '',
                isValidAccelerator
            );
            keybindRow.set_text(accel ?? '');
        });
        behaviorGroup.add(keybindRow);

        const blurRow = new Adw.SwitchRow({
            title: 'Blur/translucency style',
            subtitle: 'Use a lighter translucent background when supported.',
        });
        settings.bind('blur-enabled', blurRow, 'active', Gio.SettingsBindFlags.DEFAULT);
        behaviorGroup.add(blurRow);

        const animationSwitchRow = new Adw.SwitchRow({
            title: 'Enable animations',
            subtitle: 'Animate opening and closing for a polished feel.',
        });
        settings.bind('animations-enabled', animationSwitchRow, 'active', Gio.SettingsBindFlags.DEFAULT);
        behaviorGroup.add(animationSwitchRow);

        page.add(behaviorGroup);

        const performanceGroup = new Adw.PreferencesGroup({
            title: 'Performance',
            description: 'Tune responsiveness and list density.',
        });

        addSpinRow(
            performanceGroup,
            settings,
            'debounce-ms',
            'Typing debounce (ms)',
            'Delay before rescoring while typing.',
            5,
            80,
            1,
            5
        );

        addSpinRow(
            performanceGroup,
            settings,
            'max-results',
            'Max visible results',
            'Limit rows for stable frame-time.',
            5,
            40,
            1,
            5
        );

        addSpinRow(
            performanceGroup,
            settings,
            'open-animation-ms',
            'Open animation (ms)',
            'Duration for opening transition.',
            60,
            350,
            5,
            20
        );

        addSpinRow(
            performanceGroup,
            settings,
            'close-animation-ms',
            'Close animation (ms)',
            'Duration for closing transition.',
            50,
            300,
            5,
            20
        );

        page.add(performanceGroup);

        const rankingGroup = new Adw.PreferencesGroup({
            title: 'Ranking weights',
            description: 'Source tie-break priorities on equal fuzzy scores.',
        });

        addSpinRow(rankingGroup, settings, 'weight-windows', 'Window weight', 'Prefer open windows when scores tie.', 0, 100, 1, 5);
        addSpinRow(rankingGroup, settings, 'weight-apps', 'App weight', 'Prefer apps over recents when scores tie.', 0, 100, 1, 5);
        addSpinRow(rankingGroup, settings, 'weight-recents', 'Recent weight', 'Boost for recents provider results.', 0, 100, 1, 5);
        addSpinRow(rankingGroup, settings, 'weight-files', 'File weight', 'Boost for indexed file matches.', 0, 100, 1, 5);
        addSpinRow(rankingGroup, settings, 'weight-emoji', 'Emoji weight', 'Boost for emoji matches.', 0, 100, 1, 5);
        addSpinRow(rankingGroup, settings, 'weight-utility', 'Utility weight', 'Boost for calculator/currency/time rows.', 0, 100, 1, 5);

        page.add(rankingGroup);

        const smartGroup = new Adw.PreferencesGroup({
            title: 'Smart providers',
            description: 'Configure indexed folders and currency cache behavior.',
        });

        addStringArrayRow(
            smartGroup,
            settings,
            'indexed-folders',
            'Indexed folders (comma-separated)',
            'Example: /home/user/Documents, /home/user/Downloads'
        );

        const currencyRefreshRow = new Adw.SwitchRow({
            title: 'Currency online refresh',
            subtitle: 'When enabled, currency rates may be refreshed online in future updates.',
        });
        settings.bind('currency-refresh-enabled', currencyRefreshRow, 'active', Gio.SettingsBindFlags.DEFAULT);
        smartGroup.add(currencyRefreshRow);

        addSpinRow(
            smartGroup,
            settings,
            'currency-rate-ttl-hours',
            'Currency cache TTL (hours)',
            'How long cached rates are treated as fresh.',
            1,
            168,
            1,
            6
        );

        page.add(smartGroup);

        window.add(page);
    }
}
