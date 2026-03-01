import Clutter from 'gi://Clutter';
import Gio from 'gi://Gio';
import GLib from 'gi://GLib';
import GObject from 'gi://GObject';
import St from 'gi://St';

import * as Main from 'resource:///org/gnome/shell/ui/main.js';

import {rankResults} from '../lib/fuzzy.js';
import {extractQueryRoute} from '../lib/queryRouter.js';
import {collectProviderItems} from '../lib/providerAggregator.js';
import {combineRankedWithTail, splitTailItems} from '../lib/searchResultsLayout.js';
import {buildAliasContext, parseAliasesConfig} from '../lib/aliases.js';
import {
    applyLearningBoosts,
    parseLearningStore,
    recordAppLaunch,
    serializeLearningStore,
} from '../lib/learning.js';
import {resolveEnterAction} from '../lib/resultAction.js';
import {getResultHintActionLabel, getResultHintIconSpec} from '../lib/resultKindHint.js';

const SLIDE_Y = 14;
const ASYNC_SEARCH_THRESHOLD = 180;
const KEY_ALIASES = 'custom-aliases-json';
const KEY_LEARNING_STORE = 'launch-learning-json';
const KEY_LEARNING_ENABLED = 'learning-enabled';
const COPY_HINT_ICON = {
    relativePath: 'assets/icons/lucide/copy.svg',
    fallbackIconName: 'edit-copy-symbolic',
};

export const LauncherOverlay = GObject.registerClass(
class LauncherOverlay extends St.BoxLayout {
    _init(settings, providers, extensionPath = '') {
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
        this._extensionPath = (extensionPath ?? '').toString();
        this._signals = [];
        this._debounceSourceId = null;
        this._idleSearchSourceId = null;
        this._stageKeyFocusChangedId = null;
        this._stageCapturedEventId = null;
        this._stageButtonPressId = null;
        this._selectedIndex = 0;
        this._results = [];
        this._renderedResultKeys = [];
        this._searchGeneration = 0;
        this._maxResultsHeight = 420;
        this._hintGIconCache = new Map();
        this._settingsSignals = [];
        this._typedQuery = '';
        this._aliases = parseAliasesConfig(this._settings.get_string(KEY_ALIASES));
        this._learningStore = parseLearningStore(this._settings.get_string(KEY_LEARNING_STORE));
        this._learningEnabled = this._settings.get_boolean(KEY_LEARNING_ENABLED);

        this._header = new St.BoxLayout({
            style_class: 'hop-launcher-header',
            x_expand: true,
        });
        this.add_child(this._header);

        this._input = new St.Entry({
            style_class: 'hop-launcher-input',
            hint_text: 'Search apps, windows, files, emoji, utilities…',
            can_focus: true,
            x_expand: true,
        });
        this._header.add_child(this._input);

        this._scroll = new St.ScrollView({
            style_class: 'hop-launcher-scrollview',
            overlay_scrollbars: true,
            x_expand: true,
            y_expand: false,
            visible: false,
        });
        this._scroll.set_policy(St.PolicyType.NEVER, St.PolicyType.AUTOMATIC);
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

        this._settingsSignals.push(
            this._settings.connect(`changed::${KEY_ALIASES}`, () => {
                this._aliases = parseAliasesConfig(this._settings.get_string(KEY_ALIASES));
            }),
            this._settings.connect(`changed::${KEY_LEARNING_STORE}`, () => {
                this._learningStore = parseLearningStore(this._settings.get_string(KEY_LEARNING_STORE));
            }),
            this._settings.connect(`changed::${KEY_LEARNING_ENABLED}`, () => {
                this._learningEnabled = this._settings.get_boolean(KEY_LEARNING_ENABLED);
            })
        );

        for (const provider of this._providers) {
            if (!provider?.setUpdateCallback)
                continue;
            provider.setUpdateCallback(() => {
                if (!this.visible)
                    return;
                this._queueSearch();
            });
        }
    }

    open() {
        this._refreshProviders();
        this._input.set_text('');
        this._results = [];
        this._selectedIndex = 0;
        this._clearList();
        this._setResultsVisible(false);

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
        this._ensureAutoCloseWatch();
    }

    close() {
        this._cancelPendingSearch();
        this._disconnectAutoCloseWatch();

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
            if (!provider.refresh || !provider.refreshOnOpen)
                continue;
            try {
                provider.refresh();
            } catch (error) {
                logError(error, '[hop-launcher] provider refresh failed');
            }
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
        const normalizedQuery = query.trim();
        const generation = ++this._searchGeneration;

        if (!normalizedQuery) {
            this._typedQuery = '';
            if (this._idleSearchSourceId) {
                GLib.source_remove(this._idleSearchSourceId);
                this._idleSearchSourceId = null;
            }
            this._results = [];
            this._selectedIndex = 0;
            this._renderResults();
            return;
        }

        const items = this._collectItems(mode, query);
        const {rankedItems, tailItems} = splitTailItems(items);
        const aliasContext = buildAliasContext(normalizedQuery, this._aliases, rankedItems);
        const rankingQuery = aliasContext.rankingQuery.trim();
        const learningBoosts = this._learningEnabled
            ? applyLearningBoosts(this._learningStore, normalizedQuery, rankedItems)
            : new Map();
        const scoreBoost = item =>
            (aliasContext.boosts.get(item) ?? 0) +
            (learningBoosts.get(item) ?? 0);
        this._typedQuery = normalizedQuery;
        const maxResults = this._settings.get_int('max-results');

        if (rankedItems.length < ASYNC_SEARCH_THRESHOLD) {
            const ranked = this._rank(rankingQuery, rankedItems, scoreBoost);
            this._results = combineRankedWithTail(ranked, tailItems, maxResults);
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

            const ranked = this._rank(rankingQuery, rankedItems, scoreBoost);
            this._results = combineRankedWithTail(ranked, tailItems, maxResults);
            this._selectedIndex = 0;
            this._renderResults();
            return GLib.SOURCE_REMOVE;
        });
    }

    _rank(query, items, scoreBoost = null) {
        return rankResults(query, items, {
            weightWindows: this._settings.get_int('weight-windows'),
            weightApps: this._settings.get_int('weight-apps'),
            weightRecents: this._settings.get_int('weight-recents'),
            weightFiles: this._settings.get_int('weight-files'),
            weightEmoji: this._settings.get_int('weight-emoji'),
            weightUtility: this._settings.get_int('weight-utility'),
            maxResults: this._settings.get_int('max-results'),
            minFuzzyScore: this._settings.get_int('min-fuzzy-score'),
            scoreBoost,
        });
    }

    _collectItems(mode, query) {
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

        const all = collectProviderItems(this._providers, query, mode, error => {
            logError(error, '[hop-launcher] provider getResults failed');
        });
        if (mode === 'windows')
            return all.filter(i => i.kind === 'window');
        if (mode === 'apps')
            return all.filter(i => i.kind === 'app');
        if (mode === 'files')
            return all.filter(i => i.kind === 'file');
        if (mode === 'emoji')
            return all.filter(i => i.kind === 'emoji');
        if (mode === 'currency' || mode === 'timezone' || mode === 'calculator' || mode === 'weather')
            return all.filter(i => i.kind === 'utility');
        return all;
    }

    _renderResults() {
        const hasQuery = this._input.get_text().trim().length > 0;
        if (!hasQuery || this._results.length === 0) {
            this._clearList();
            this._setResultsVisible(false);
            return;
        }

        this._setResultsVisible(true);
        const nextKeys = this._results.map(result =>
            `${result.kind}:${result.id ?? result.primaryText ?? ''}:${result.secondaryText ?? ''}`
        );
        const unchanged = nextKeys.length === this._renderedResultKeys.length &&
            nextKeys.every((key, index) => key === this._renderedResultKeys[index]);

        if (unchanged) {
            this._updateSelectionStyles();
            this._ensureSelectionVisible();
            this._updateResultsHeight();
            return;
        }

        this._clearList();
        this._renderedResultKeys = nextKeys;

        this._results.forEach((result, index) => {
            const enterAction = resolveEnterAction(result);
            const isCopyAction = enterAction.type === 'copy';
            const actionLabel = getResultHintActionLabel(result.kind, enterAction.type);
            const row = new St.BoxLayout({
                style_class: `hop-launcher-row${index === this._selectedIndex ? ' selected' : ''}`,
                x_expand: true,
            });

            const icon = new St.Icon({style_class: 'hop-launcher-icon'});
            if (result.icon !== null && result.icon !== undefined)
                icon.gicon = result.icon;
            else if (isCopyAction) {
                icon.icon_name = COPY_HINT_ICON.fallbackIconName;
                icon.add_style_class_name('hop-launcher-icon-copy');
            } else {
                icon.icon_name = 'system-search-symbolic';
            }

            const secondaryText = (result.secondaryText ?? '').toString();
            const hasSecondaryText = secondaryText.trim().length > 0;
            const text = new St.BoxLayout({vertical: true, x_expand: true});
            text.add_child(new St.Label({text: result.primaryText ?? ''}));
            if (hasSecondaryText) {
                text.add_child(new St.Label({text: secondaryText, style_class: 'dim-label'}));
            } else {
                row.add_style_class_name('hop-launcher-row-singleline');
                text.add_style_class_name('hop-launcher-text-singleline');
            }

            const hint = new St.BoxLayout({style_class: 'hop-launcher-hint-box'});
            const hintIcon = isCopyAction
                ? this._createHintIconFromSpec(COPY_HINT_ICON, true)
                : this._createHintIcon(result.kind);
            if (hintIcon)
                hint.add_child(hintIcon);
            hint.add_child(new St.Label({
                text: actionLabel,
                style_class: isCopyAction ? 'hop-launcher-copy-label' : 'dim-label',
            }));
            const hintContainer = new St.BoxLayout({
                style_class: 'hop-launcher-hint-container',
                x_expand: false,
                y_expand: true,
                vertical: true,
                x_align: Clutter.ActorAlign.END,
                y_align: hasSecondaryText ? Clutter.ActorAlign.END : Clutter.ActorAlign.CENTER,
            });
            hintContainer.add_child(hint);

            row.add_child(icon);
            row.add_child(text);
            row.add_child(hintContainer);
            this._list.add_child(row);
        });

        this._updateResultsHeight();
        this._ensureSelectionVisible();
    }

    _createHintIcon(kind) {
        const spec = getResultHintIconSpec(kind);
        return this._createHintIconFromSpec(spec, false);
    }

    _createHintIconFromSpec(spec, forceWhite = false) {
        if (!spec)
            return null;

        const icon = new St.Icon({style_class: 'hop-launcher-hint-icon dim-label'});
        if (forceWhite)
            icon.add_style_class_name('hop-launcher-hint-icon-copy');
        const fileIcon = this._loadHintGIcon(spec.relativePath);
        if (fileIcon)
            icon.gicon = fileIcon;
        else
            icon.icon_name = spec.fallbackIconName;
        return icon;
    }

    _loadHintGIcon(relativePath) {
        if (!relativePath)
            return null;
        if (this._hintGIconCache.has(relativePath))
            return this._hintGIconCache.get(relativePath);
        if (!this._extensionPath) {
            this._hintGIconCache.set(relativePath, null);
            return null;
        }

        let icon = null;
        try {
            const fullPath = GLib.build_filenamev([this._extensionPath, relativePath]);
            const file = Gio.File.new_for_path(fullPath);
            if (file.query_exists(null))
                icon = new Gio.FileIcon({file});
        } catch (_) {
            icon = null;
        }

        this._hintGIconCache.set(relativePath, icon);
        return icon;
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
        this._renderedResultKeys = [];
    }

    setMaxResultsHeight(height) {
        const value = Number.isFinite(height) ? Math.floor(height) : 420;
        this._maxResultsHeight = Math.max(120, value);
        if (this._scroll.visible)
            this._updateResultsHeight();
    }

    _setResultsVisible(visible) {
        this._scroll.visible = visible;
        this._scroll.y_expand = false;
        if (!visible) {
            this._scroll.set_height(0);
            return;
        }
        this._updateResultsHeight();
    }

    _updateResultsHeight() {
        const [, naturalHeight] = this._list.get_preferred_height(-1);
        const targetHeight = Math.max(1, Math.min(this._maxResultsHeight, naturalHeight + 4));
        this._scroll.set_height(targetHeight);
    }

    _updateSelectionStyles() {
        const children = this._list.get_children();
        for (let index = 0; index < children.length; index++) {
            const row = children[index];
            if (index === this._selectedIndex)
                row.add_style_class_name('selected');
            else
                row.remove_style_class_name('selected');
        }
    }

    _moveSelection(delta) {
        if (this._results.length === 0)
            return Clutter.EVENT_PROPAGATE;

        const next = Math.max(0, Math.min(this._results.length - 1, this._selectedIndex + delta));
        if (next === this._selectedIndex)
            return Clutter.EVENT_STOP;

        this._selectedIndex = next;
        this._updateSelectionStyles();
        this._ensureSelectionVisible();
        return Clutter.EVENT_STOP;
    }

    _ensureAutoCloseWatch() {
        if (this._stageKeyFocusChangedId)
            return;

        this._stageKeyFocusChangedId = global.stage.connect('notify::key-focus', () => {
            if (!this.visible)
                return;

            const focus = global.stage.get_key_focus();
            if (!focus || !this.contains(focus))
                this.close();
        });

        this._stageCapturedEventId = global.stage.connect('captured-event', (_stage, event) => {
            if (!this.visible)
                return Clutter.EVENT_PROPAGATE;

            const type = event.type();
            if (type !== Clutter.EventType.BUTTON_PRESS && type !== Clutter.EventType.TOUCH_BEGIN)
                return Clutter.EVENT_PROPAGATE;

            if (this._isEventInsideOverlay(event))
                return Clutter.EVENT_PROPAGATE;

            this.close();
            return Clutter.EVENT_PROPAGATE;
        });

        this._stageButtonPressId = global.stage.connect('button-press-event', (_stage, event) => {
            if (!this.visible)
                return Clutter.EVENT_PROPAGATE;

            if (this._isEventInsideOverlay(event))
                return Clutter.EVENT_PROPAGATE;

            this.close();
            return Clutter.EVENT_PROPAGATE;
        });
    }

    _disconnectAutoCloseWatch() {
        if (this._stageKeyFocusChangedId) {
            global.stage.disconnect(this._stageKeyFocusChangedId);
            this._stageKeyFocusChangedId = null;
        }

        if (this._stageCapturedEventId) {
            global.stage.disconnect(this._stageCapturedEventId);
            this._stageCapturedEventId = null;
        }

        if (this._stageButtonPressId) {
            global.stage.disconnect(this._stageButtonPressId);
            this._stageButtonPressId = null;
        }
    }

    _isEventInsideOverlay(event) {
        const source = event.get_source?.() ?? global.stage.get_event_actor?.(event);
        if (source && this.contains(source))
            return true;

        const [x, y] = event.get_coords?.() ?? [null, null];
        if (x === null || y === null)
            return false;

        const [overlayX, overlayY] = this.get_transformed_position();
        const [overlayWidth, overlayHeight] = this.get_transformed_size();
        return x >= overlayX &&
            y >= overlayY &&
            x <= overlayX + overlayWidth &&
            y <= overlayY + overlayHeight;
    }

    _onKeyPress(event) {
        const symbol = event.get_key_symbol();

        if (symbol === Clutter.KEY_Escape) {
            this.close();
            return Clutter.EVENT_STOP;
        }

        if (symbol === Clutter.KEY_Down) {
            return this._moveSelection(1);
        }

        if (symbol === Clutter.KEY_Up) {
            return this._moveSelection(-1);
        }

        if (this._results.length === 0)
            return Clutter.EVENT_PROPAGATE;

        if (symbol === Clutter.KEY_Return || symbol === Clutter.KEY_KP_Enter) {
            const selected = this._results[this._selectedIndex];
            const action = resolveEnterAction(selected);
            if (action.type === 'copy')
                this._copyText(action.text);
            else if (action.type === 'execute')
                selected.execute();

            if (selected?.kind === 'app' && this._learningEnabled && selected.id) {
                this._learningStore = recordAppLaunch(this._learningStore, this._typedQuery, selected.id);
                this._settings.set_string(KEY_LEARNING_STORE, serializeLearningStore(this._learningStore));
            }

            this.close();
            return Clutter.EVENT_STOP;
        }

        return Clutter.EVENT_PROPAGATE;
    }

    destroyOverlay() {
        this._cancelPendingSearch();
        this._disconnectAutoCloseWatch();

        for (const id of this._signals)
            this._input.clutter_text.disconnect(id);
        this._signals = [];
        for (const id of this._settingsSignals)
            this._settings.disconnect(id);
        this._settingsSignals = [];
        this._hintGIconCache.clear();

        this.destroy();
    }

    _copyText(text) {
        const value = (text ?? '').toString().trim();
        if (!value)
            return;
        const clipboard = St.Clipboard.get_default();
        clipboard.set_text(St.ClipboardType.CLIPBOARD, value);
    }
});
