import Adw from 'gi://Adw';
import Gio from 'gi://Gio';
import Gtk from 'gi://Gtk';

import {ExtensionPreferences} from 'resource:///org/gnome/Shell/Extensions/js/extensions/prefs.js';

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
            settings.set_strv('toggle-launcher', value ? [value] : ['<Super>space']);
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

        page.add(rankingGroup);

        window.add(page);
    }
}
