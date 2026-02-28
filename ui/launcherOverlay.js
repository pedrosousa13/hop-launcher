import Clutter from 'gi://Clutter';
import GLib from 'gi://GLib';
import GObject from 'gi://GObject';
import St from 'gi://St';

import * as Main from 'resource:///org/gnome/shell/ui/main.js';

import {rankResults} from '../lib/fuzzy.js';
import {extractQueryRoute} from '../lib/queryRouter.js';

const SLIDE_Y = 14;
const ASYNC_SEARCH_THRESHOLD = 180;

export const LauncherOverlay = GObject.registerClass(
class LauncherOverlay extends St.BoxLayout {
    _init(settings, providers) {
        super._init({
            style_class: 'hop-launcher-overlay',
            reactive: true,
            can_focus: true,
            track_hover: true,
            vertical: true,
            visible: false,
            opacity: 0,
            scale_x: 0.98,
            scale_y: 0.98,
            translation_y: -SLIDE_Y,
        });

        this._settings = settings;
        this._providers = providers;
        this._signals = [];
        this._debounceSourceId = null;
        this._idleSearchSourceId = null;
        this._selectedIndex = 0;
        this._results = [];
        this._searchGeneration = 0;

        this._input = new St.Entry({
            style_class: 'hop-launcher-input',
            hint_text: 'Search apps, windows, recents…',
            can_focus: true,
            x_expand: true,
        });
        this.add_child(this._input);

        this._scroll = new St.ScrollView({
            style_class: 'hop-launcher-scrollview',
            overlay_scrollbars: true,
            x_expand: true,
            y_expand: true,
        });
        this._list = new St.BoxLayout({
            style_class: 'hop-launcher-results',
            vertical: true,
            x_expand: true,
        });
        this._scroll.set_child(this._list);
        this.add_child(this._scroll);

        this._signals.push(
            this._input.clutter_text.connect('text-changed', () => this._queueSearch()),
            this._input.clutter_text.connect('key-press-event', (_, event) => this._onKeyPress(event))
        );
    }

    open() {
        this._refreshProviders();
        this._runSearch();

        this.visible = true;
        this.remove_all_transitions();
        const animate = this._settings.get_boolean('animations-enabled');
        this.ease({
            opacity: 255,
            scale_x: 1,
            scale_y: 1,
            translation_y: 0,
            duration: animate ? this._settings.get_int('open-animation-ms') : 1,
            mode: Clutter.AnimationMode.EASE_OUT_CUBIC,
        });
        global.stage.set_key_focus(this._input.clutter_text);
    }

    close() {
        this._cancelPendingSearch();

        this.remove_all_transitions();
        const animate = this._settings.get_boolean('animations-enabled');
        this.ease({
            opacity: 0,
            scale_x: 0.985,
            scale_y: 0.985,
            translation_y: -SLIDE_Y,
            duration: animate ? this._settings.get_int('close-animation-ms') : 1,
            mode: Clutter.AnimationMode.EASE_OUT_QUAD,
            onComplete: () => {
                this.visible = false;
                this._clearList();
                this._input.set_text('');
            },
        });
    }

    _refreshProviders() {
        for (const provider of this._providers) {
            if (provider.refresh)
                provider.refresh();
        }
    }

    _cancelPendingSearch() {
        if (this._debounceSourceId) {
            GLib.source_remove(this._debounceSourceId);
            this._debounceSourceId = null;
        }

        if (this._idleSearchSourceId) {
            GLib.source_remove(this._idleSearchSourceId);
            this._idleSearchSourceId = null;
        }
    }

    _queueSearch() {
        if (this._debounceSourceId) {
            GLib.source_remove(this._debounceSourceId);
            this._debounceSourceId = null;
        }

        const delay = this._settings.get_int('debounce-ms');
        this._debounceSourceId = GLib.timeout_add(GLib.PRIORITY_DEFAULT_IDLE, delay, () => {
            this._debounceSourceId = null;
            this._runSearch();
            return GLib.SOURCE_REMOVE;
        });
    }

    _runSearch() {
        const rawQuery = this._input.get_text();
        const {query, mode} = extractQueryRoute(rawQuery);
        const items = this._collectItems(mode);
        const generation = ++this._searchGeneration;

        if (items.length < ASYNC_SEARCH_THRESHOLD) {
            this._results = this._rank(query, items);
            this._selectedIndex = 0;
            this._renderResults();
            return;
        }

        if (this._idleSearchSourceId)
            GLib.source_remove(this._idleSearchSourceId);

        this._idleSearchSourceId = GLib.idle_add(GLib.PRIORITY_DEFAULT_IDLE, () => {
            this._idleSearchSourceId = null;
            if (generation !== this._searchGeneration)
                return GLib.SOURCE_REMOVE;

            this._results = this._rank(query, items);
            this._selectedIndex = 0;
            this._renderResults();
            return GLib.SOURCE_REMOVE;
        });
    }

    _rank(query, items) {
        return rankResults(query, items, {
            weightWindows: this._settings.get_int('weight-windows'),
            weightApps: this._settings.get_int('weight-apps'),
            weightRecents: this._settings.get_int('weight-recents'),
            maxResults: this._settings.get_int('max-results'),
        });
    }

    _collectItems(mode) {
        const all = this._providers.flatMap(p => p.getResults());
        if (mode === 'windows')
            return all.filter(i => i.kind === 'window');
        if (mode === 'apps')
            return all.filter(i => i.kind === 'app');
        if (mode === 'files')
            return all.filter(i => i.kind === 'file');
        if (mode === 'emoji')
            return all.filter(i => i.kind === 'emoji');
        if (mode === 'currency' || mode === 'timezone' || mode === 'calculator')
            return all.filter(i => i.kind === 'utility');
        if (mode === 'actions') {
            return [
                {
                    kind: 'action',
                    id: 'show-overview',
                    primaryText: 'Show Overview',
                    secondaryText: 'Shell action',
                    execute: () => Main.overview.show(),
                },
            ];
        }
        return all;
    }

    _renderResults() {
        this._clearList();

        if (this._results.length === 0) {
            const empty = new St.Label({
                text: 'No results',
                style_class: 'dim-label hop-launcher-empty',
                x_align: Clutter.ActorAlign.START,
            });
            this._list.add_child(empty);
            return;
        }

        this._results.forEach((result, index) => {
            const row = new St.BoxLayout({
                style_class: `hop-launcher-row${index === this._selectedIndex ? ' selected' : ''}`,
                x_expand: true,
            });

            const icon = new St.Icon({
                gicon: result.icon ?? null,
                icon_name: result.icon ? undefined : 'system-search-symbolic',
                style_class: 'hop-launcher-icon',
            });

            const text = new St.BoxLayout({vertical: true, x_expand: true});
            text.add_child(new St.Label({text: result.primaryText ?? ''}));
            text.add_child(new St.Label({text: result.secondaryText ?? '', style_class: 'dim-label'}));

            const hint = new St.Label({text: 'Enter', style_class: 'dim-label'});

            row.add_child(icon);
            row.add_child(text);
            row.add_child(hint);
            this._list.add_child(row);
        });

        this._ensureSelectionVisible();
    }

    _ensureSelectionVisible() {
        const vadj = this._scroll.vadjustment;
        if (!vadj)
            return;

        const row = this._list.get_child_at_index(this._selectedIndex);
        if (!row)
            return;

        const pageSize = vadj.page_size;
        const viewTop = vadj.value;
        const viewBottom = viewTop + pageSize;
        const rowTop = row.y;
        const rowBottom = row.y + row.height;

        if (rowTop < viewTop)
            vadj.value = Math.max(vadj.lower, rowTop - 10);
        else if (rowBottom > viewBottom)
            vadj.value = Math.min(vadj.upper - pageSize, rowBottom - pageSize + 10);
    }

    _clearList() {
        this._list.destroy_all_children();
    }

    _onKeyPress(event) {
        const symbol = event.get_key_symbol();

        if (symbol === Clutter.KEY_Escape) {
            this.close();
            return Clutter.EVENT_STOP;
        }

        if (this._results.length === 0)
            return Clutter.EVENT_PROPAGATE;

        if (symbol === Clutter.KEY_Down) {
            this._selectedIndex = Math.min(this._selectedIndex + 1, this._results.length - 1);
            this._renderResults();
            return Clutter.EVENT_STOP;
        }

        if (symbol === Clutter.KEY_Up) {
            this._selectedIndex = Math.max(this._selectedIndex - 1, 0);
            this._renderResults();
            return Clutter.EVENT_STOP;
        }

        if (symbol === Clutter.KEY_Return || symbol === Clutter.KEY_KP_Enter) {
            const selected = this._results[this._selectedIndex];
            if (selected?.execute)
                selected.execute();
            this.close();
            return Clutter.EVENT_STOP;
        }

        return Clutter.EVENT_PROPAGATE;
    }

    destroyOverlay() {
        this._cancelPendingSearch();

        for (const id of this._signals)
            this._input.clutter_text.disconnect(id);
        this._signals = [];

        this.destroy();
    }
});
