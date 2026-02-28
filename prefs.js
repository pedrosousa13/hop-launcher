import Adw from 'gi://Adw';
import Gio from 'gi://Gio';
import Gtk from 'gi://Gtk';

import {ExtensionPreferences} from 'resource:///org/gnome/Shell/Extensions/js/extensions/prefs.js';


function isValidAccelerator(accel) {
    const [keyval, mods] = Gtk.accelerator_parse(accel);
    return keyval !== 0 && mods !== 0;
}

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
        });
        keybindRow.connect('changed', row => {
            const value = row.text.trim();
            const accel = value || '<Super>space';
            if (isValidAccelerator(accel))
                settings.set_strv('toggle-launcher', [accel]);
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
