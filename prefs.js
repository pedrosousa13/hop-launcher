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
import {parseAliasesConfig} from './lib/aliases.js';
import {buildLearningInsights} from './lib/learningInsights.js';
import {
    DEFAULT_WEB_SEARCH_SERVICES,
    parseWebSearchServices,
    serializeWebSearchServices,
    validateWebSearchService,
} from './lib/webSearchConfig.js';


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

const ALIAS_TYPES = ['rewrite', 'app', 'window'];

function normalizeAlias(value) {
    return (value ?? '')
        .toString()
        .trim()
        .toLowerCase();
}

function formatAliasTarget(rule) {
    if (rule.type === 'rewrite')
        return rule.target?.query ?? '';
    if (rule.type === 'app')
        return rule.target?.appId ?? '';
    if (rule.type === 'window') {
        const appId = (rule.target?.appId ?? '').trim();
        const titleContains = (rule.target?.titleContains ?? '').trim();
        if (appId && titleContains)
            return `${appId}|${titleContains}`;
        return appId || titleContains;
    }
    return '';
}

function parseAliasTarget(type, rawTarget) {
    const target = (rawTarget ?? '').toString().trim();
    if (type === 'rewrite')
        return target ? {query: target} : null;
    if (type === 'app')
        return target ? {appId: target} : null;
    if (type === 'window') {
        const [left = '', right = ''] = target.split('|', 2);
        const appId = left.trim();
        const titleContains = right.trim();
        if (!appId && !titleContains)
            return null;
        return {appId, titleContains};
    }
    return null;
}

function aliasTargetPlaceholder(type) {
    if (type === 'rewrite')
        return 'expanded query text';
    if (type === 'app')
        return 'desktop app id (example: org.gnome.Terminal.desktop)';
    return 'appId|titleContains (example: org.gnome.Terminal.desktop|standup)';
}

function aliasTypeDescription(type) {
    if (type === 'rewrite')
        return 'Replaces the search text for ranking.';
    if (type === 'app')
        return 'Boosts one app when alias matches exactly.';
    return 'Boosts open windows by app id and/or title match.';
}

function aliasTargetExample(type) {
    if (type === 'rewrite')
        return 'Example: youtube';
    if (type === 'app')
        return 'Example: org.gnome.Terminal.desktop';
    return 'Examples: org.gnome.Calendar.desktop|standup, |standup, org.gnome.Calendar.desktop|';
}

function formatLearningTimestamp(ms) {
    if (!Number.isFinite(ms) || ms <= 0)
        return 'unknown';
    return new Date(ms).toISOString().replace('T', ' ').replace('Z', ' UTC');
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

        const searchGroup = new Adw.PreferencesGroup({
            title: 'Search',
            description: 'Main search behavior and providers.',
        });

        addSpinRow(
            searchGroup,
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
            searchGroup,
            settings,
            'min-fuzzy-score',
            'Fuzzy match threshold',
            'Higher value means stricter matching.',
            0,
            120,
            1,
            5
        );

        const webSearchEnabledRow = new Adw.SwitchRow({
            title: 'Web search actions',
            subtitle: 'Append search actions at the end for non-empty queries.',
        });
        settings.bind('web-search-enabled', webSearchEnabledRow, 'active', Gio.SettingsBindFlags.DEFAULT);
        searchGroup.add(webSearchEnabledRow);

        addSpinRow(
            searchGroup,
            settings,
            'web-search-max-actions',
            'Max web actions',
            'Maximum number of providers appended for each query.',
            1,
            8,
            1,
            1
        );

        const providersHeaderRow = new Adw.ActionRow({
            title: 'Web providers',
            subtitle: 'Each row uses a provider name and an HTTPS URL template with %s.',
        });
        const addProviderButton = new Gtk.Button({label: 'Add provider'});
        providersHeaderRow.add_suffix(addProviderButton);
        searchGroup.add(providersHeaderRow);

        const providerStatusRow = new Adw.ActionRow({
            title: 'Provider status',
            subtitle: 'Checking configured providers…',
        });
        providerStatusRow.set_activatable(false);
        searchGroup.add(providerStatusRow);

        const defaultsRow = new Adw.ActionRow({
            title: 'Reset web search defaults',
            subtitle: 'Restore built-in Google and DuckDuckGo templates.',
        });
        const defaultsButton = new Gtk.Button({label: 'Reset defaults'});
        defaultsButton.connect('clicked', () => {
            settings.set_string('web-search-services-json', JSON.stringify(DEFAULT_WEB_SEARCH_SERVICES));
        });
        defaultsRow.add_suffix(defaultsButton);
        searchGroup.add(defaultsRow);

        addStringArrayRow(
            searchGroup,
            settings,
            'indexed-folders',
            'Indexed folders (comma-separated)',
            'Example: /home/user/Documents, /home/user/Downloads'
        );

        let webSearchServices = parseWebSearchServices(
            settings.get_string('web-search-services-json'),
            {fallbackToDefaults: false}
        );
        let providerRows = [];

        const providerErrorLabel = reason => {
            if (reason === 'name-missing')
                return 'Provider name is required.';
            if (reason === 'template-missing-placeholder')
                return 'URL must contain %s placeholder.';
            if (reason === 'template-invalid-url')
                return 'URL must be a valid template.';
            if (reason === 'template-non-https')
                return 'URL must use HTTPS.';
            return 'Invalid provider settings.';
        };

        const previewSearchUrl = (urlTemplate, sampleQuery = 'hop launcher') => {
            const template = (urlTemplate ?? '').toString().trim();
            if (!template)
                return 'Enter a URL template to preview.';
            if (!template.includes('%s'))
                return 'Add %s in the template to insert the search query.';
            try {
                return template.replaceAll('%s', encodeURIComponent(sampleQuery));
            } catch (_) {
                return 'Template preview unavailable.';
            }
        };

        const updateProviderStatus = () => {
            if (webSearchServices.length === 0) {
                providerStatusRow.set_subtitle('No providers configured. Add at least one row.');
                return;
            }

            const names = webSearchServices.map(service => service.name).join(', ');
            providerStatusRow.set_subtitle(`Loaded ${webSearchServices.length} provider(s): ${names}`);
        };

        const saveWebSearchServices = nextServices => {
            const json = serializeWebSearchServices(nextServices, {fallbackToDefaults: false});
            settings.set_string('web-search-services-json', json);
        };

        const rebuildProviderRows = () => {
            for (const row of providerRows)
                searchGroup.remove(row);
            providerRows = [];

            webSearchServices.forEach((service, index) => {
                const row = new Adw.ExpanderRow({
                    title: `Provider ${index + 1}: ${service.name}`,
                    subtitle: service.urlTemplate,
                });
                row.set_enable_expansion(true);
                row.set_expanded(false);

                const nameEntry = new Gtk.Entry({text: service.name});
                nameEntry.set_placeholder_text('Provider name');
                nameEntry.set_hexpand(true);
                const nameRow = new Adw.ActionRow({title: 'Name'});
                nameRow.set_activatable(false);
                nameRow.add_suffix(nameEntry);

                const urlEntry = new Gtk.Entry({text: service.urlTemplate});
                urlEntry.set_placeholder_text('https://example.com/search?q=%s');
                urlEntry.set_hexpand(true);
                urlEntry.set_width_chars(32);
                const urlRow = new Adw.ActionRow({title: 'URL template'});
                urlRow.set_activatable(false);
                urlRow.add_suffix(urlEntry);

                const keywordEntry = new Gtk.Entry({text: service.keyword ?? ''});
                keywordEntry.set_placeholder_text('Optional quick keyword (example: g)');
                keywordEntry.set_hexpand(true);
                const keywordRow = new Adw.ActionRow({title: 'Keyword (optional)'});
                keywordRow.set_activatable(false);
                keywordRow.add_suffix(keywordEntry);

                const helperRow = new Adw.ActionRow({
                    title: 'Template helper',
                    subtitle: 'Insert %s as query placeholder.',
                });
                helperRow.set_activatable(false);
                const insertPlaceholderButton = new Gtk.Button({label: 'Insert %s'});
                insertPlaceholderButton.connect('clicked', () => {
                    const current = urlEntry.get_text();
                    if (current.includes('%s'))
                        return;
                    if (!current.trim()) {
                        urlEntry.set_text('https://example.com/search?q=%s');
                        return;
                    }
                    urlEntry.set_text(`${current}${current.includes('?') ? '&' : '?'}q=%s`);
                });
                helperRow.add_suffix(insertPlaceholderButton);

                const previewRow = new Adw.ActionRow({
                    title: 'Preview URL',
                    subtitle: previewSearchUrl(service.urlTemplate),
                });
                previewRow.set_activatable(false);
                urlEntry.connect('changed', entry => {
                    previewRow.set_subtitle(previewSearchUrl(entry.get_text()));
                });

                const moveUpButton = new Gtk.Button({label: 'Up'});
                moveUpButton.set_sensitive(index > 0);
                moveUpButton.connect('clicked', () => {
                    if (index <= 0)
                        return;
                    const next = [...webSearchServices];
                    const [item] = next.splice(index, 1);
                    next.splice(index - 1, 0, item);
                    saveWebSearchServices(next);
                });

                const moveDownButton = new Gtk.Button({label: 'Down'});
                moveDownButton.set_sensitive(index < webSearchServices.length - 1);
                moveDownButton.connect('clicked', () => {
                    if (index >= webSearchServices.length - 1)
                        return;
                    const next = [...webSearchServices];
                    const [item] = next.splice(index, 1);
                    next.splice(index + 1, 0, item);
                    saveWebSearchServices(next);
                });

                const saveButton = new Gtk.Button({label: 'Save'});
                saveButton.connect('clicked', () => {
                    const checked = validateWebSearchService({
                        ...service,
                        name: nameEntry.get_text(),
                        urlTemplate: urlEntry.get_text(),
                        keyword: keywordEntry.get_text(),
                    });
                    if (!checked.valid) {
                        providerStatusRow.set_subtitle(providerErrorLabel(checked.reason));
                        return;
                    }

                    const next = webSearchServices.map((entry, entryIndex) =>
                        entryIndex === index ? checked.value : entry
                    );
                    saveWebSearchServices(next);
                });

                const deleteButton = new Gtk.Button({label: 'Delete'});
                deleteButton.connect('clicked', () => {
                    const next = webSearchServices.filter((_, entryIndex) => entryIndex !== index);
                    saveWebSearchServices(next);
                });

                const actionsRow = new Adw.ActionRow({title: 'Actions'});
                actionsRow.set_activatable(false);
                actionsRow.add_suffix(moveUpButton);
                actionsRow.add_suffix(moveDownButton);
                actionsRow.add_suffix(saveButton);
                actionsRow.add_suffix(deleteButton);

                row.add_row(nameRow);
                row.add_row(urlRow);
                row.add_row(keywordRow);
                row.add_row(helperRow);
                row.add_row(previewRow);
                row.add_row(actionsRow);
                providerRows.push(row);
                searchGroup.add(row);
            });

            updateProviderStatus();
        };

        addProviderButton.connect('clicked', () => {
            const next = [
                ...webSearchServices,
                {
                    name: 'New provider',
                    urlTemplate: 'https://example.com/search?q=%s',
                    enabled: true,
                    keyword: '',
                },
            ];
            saveWebSearchServices(next);
        });

        settings.connect('changed::web-search-services-json', () => {
            webSearchServices = parseWebSearchServices(
                settings.get_string('web-search-services-json'),
                {fallbackToDefaults: false}
            );
            rebuildProviderRows();
        });

        rebuildProviderRows();
        page.add(searchGroup);

        const integrationsGroup = new Adw.PreferencesGroup({
            title: 'Integrations',
            description: 'Online provider behavior and cache freshness.',
        });

        const currencyRefreshRow = new Adw.SwitchRow({
            title: 'Currency online refresh',
            subtitle: 'When enabled, currency rates may be refreshed online in future updates.',
        });
        settings.bind('currency-refresh-enabled', currencyRefreshRow, 'active', Gio.SettingsBindFlags.DEFAULT);
        integrationsGroup.add(currencyRefreshRow);

        addSpinRow(
            integrationsGroup,
            settings,
            'currency-rate-ttl-hours',
            'Currency cache TTL (hours)',
            'How long cached rates are treated as fresh.',
            1,
            168,
            1,
            6
        );

        page.add(integrationsGroup);

        const personalizationGroup = new Adw.PreferencesGroup({
            title: 'Personalization',
            description: 'Configure aliases and usage-based ranking.',
        });

        const learningRow = new Adw.SwitchRow({
            title: 'Learn from app launches',
            subtitle: 'Boost apps you pick often for similar searches.',
        });
        settings.bind('learning-enabled', learningRow, 'active', Gio.SettingsBindFlags.DEFAULT);
        personalizationGroup.add(learningRow);

        const resetLearningRow = new Adw.ActionRow({
            title: 'Reset learned ranking',
            subtitle: 'Clear launch history used by personalized ranking.',
        });
        const resetLearningButton = new Gtk.Button({label: 'Reset'});
        resetLearningButton.connect('clicked', () => {
            settings.set_string('launch-learning-json', '{"version":1,"entries":{}}');
        });
        resetLearningRow.add_suffix(resetLearningButton);
        personalizationGroup.add(resetLearningRow);

        const learningSortModel = Gtk.StringList.new(['Most used', 'Most recent']);
        const learningSortDropDown = new Gtk.DropDown({model: learningSortModel});
        learningSortDropDown.set_hexpand(false);
        learningSortDropDown.set_halign(Gtk.Align.END);
        const learningSortRow = new Adw.ActionRow({
            title: 'Learning insights sort',
            subtitle: 'Choose how learned query-app pairs are ordered.',
        });
        learningSortRow.set_activatable(false);
        learningSortRow.add_suffix(learningSortDropDown);
        personalizationGroup.add(learningSortRow);

        addSpinRow(
            personalizationGroup,
            settings,
            'learning-insights-limit',
            'Learning insights limit',
            'How many learned pairs to show below.',
            1,
            50,
            1,
            5
        );

        let learningInsightRows = [];

        const getSortSelection = () =>
            settings.get_string('learning-insights-sort') === 'recent' ? 1 : 0;

        const getSortKey = () =>
            learningSortDropDown.get_selected() === 1 ? 'recent' : 'count';

        learningSortDropDown.set_selected(getSortSelection());
        learningSortDropDown.connect('notify::selected', () => {
            const selectedSort = getSortKey();
            if (settings.get_string('learning-insights-sort') !== selectedSort)
                settings.set_string('learning-insights-sort', selectedSort);
        });

        const rebuildLearningInsights = () => {
            for (const row of learningInsightRows)
                personalizationGroup.remove(row);
            learningInsightRows = [];

            const insights = buildLearningInsights(settings.get_string('launch-learning-json'), {
                limit: settings.get_int('learning-insights-limit'),
                sort: settings.get_string('learning-insights-sort'),
            });

            if (!insights.length) {
                const emptyRow = new Adw.ActionRow({
                    title: 'No learned entries yet',
                    subtitle: 'Launch apps from search to build personalized ranking.',
                });
                emptyRow.set_activatable(false);
                personalizationGroup.add(emptyRow);
                learningInsightRows.push(emptyRow);
                return;
            }

            for (const entry of insights) {
                const row = new Adw.ActionRow({
                    title: `${entry.query} -> ${entry.appId}`,
                    subtitle: `Count ${entry.count} | Last used ${formatLearningTimestamp(entry.lastUsedMs)}`,
                });
                row.set_activatable(false);
                personalizationGroup.add(row);
                learningInsightRows.push(row);
            }
        };

        settings.connect('changed::learning-insights-sort', () => {
            if (learningSortDropDown.get_selected() !== getSortSelection())
                learningSortDropDown.set_selected(getSortSelection());
            rebuildLearningInsights();
        });
        settings.connect('changed::learning-insights-limit', rebuildLearningInsights);
        settings.connect('changed::launch-learning-json', rebuildLearningInsights);
        rebuildLearningInsights();

        const aliasesHeaderRow = new Adw.ActionRow({
            title: 'Alias rules',
            subtitle: 'Alias must match exactly.',
        });
        const addAliasButton = new Gtk.Button({label: 'Add alias'});
        aliasesHeaderRow.add_suffix(addAliasButton);
        personalizationGroup.add(aliasesHeaderRow);

        let aliases = parseAliasesConfig(settings.get_string('custom-aliases-json'));
        let aliasRows = [];

        const saveAliases = nextAliases => {
            const cleaned = parseAliasesConfig(JSON.stringify(nextAliases));
            aliases = cleaned;
            settings.set_string('custom-aliases-json', JSON.stringify(cleaned));
            rebuildAliasRows();
        };

        const rebuildAliasRows = () => {
            for (const row of aliasRows)
                personalizationGroup.remove(row);
            aliasRows = [];

            aliases.forEach((rule, index) => {
                const row = new Adw.ExpanderRow({
                    title: `Rule ${index + 1}: ${rule.alias}`,
                    subtitle: `${rule.type} -> ${formatAliasTarget(rule)}`,
                });
                row.set_enable_expansion(true);
                row.set_expanded(false);

                const aliasEntry = new Gtk.Entry({
                    text: rule.alias,
                });
                aliasEntry.set_placeholder_text('alias');
                const aliasRow = new Adw.ActionRow({
                    title: 'Alias',
                    subtitle: 'Exact key used in search. No spaces.',
                });
                aliasRow.set_activatable(false);
                aliasRow.add_suffix(aliasEntry);

                const typeModel = Gtk.StringList.new(ALIAS_TYPES);
                const typeDropDown = new Gtk.DropDown({model: typeModel});
                typeDropDown.set_selected(Math.max(0, ALIAS_TYPES.indexOf(rule.type)));
                typeDropDown.set_hexpand(false);
                typeDropDown.set_halign(Gtk.Align.END);
                const typeRow = new Adw.ActionRow({title: 'Type'});
                typeRow.set_activatable(false);
                typeRow.add_suffix(typeDropDown);
                const explanationRow = new Adw.ActionRow({
                    title: 'Explanation',
                    subtitle: `${aliasTypeDescription(rule.type)} Format: ${aliasTargetExample(rule.type)}`,
                });
                explanationRow.set_activatable(false);

                const targetEntry = new Gtk.Entry({
                    text: formatAliasTarget(rule),
                });
                targetEntry.set_placeholder_text(aliasTargetPlaceholder(rule.type));
                targetEntry.set_hexpand(true);
                targetEntry.set_width_chars(24);
                const targetRow = new Adw.ActionRow({title: 'Target'});
                targetRow.set_activatable(false);
                targetRow.add_suffix(targetEntry);

                typeDropDown.connect('notify::selected', dropDown => {
                    const selectedType = ALIAS_TYPES[dropDown.get_selected()] ?? 'rewrite';
                    targetEntry.set_placeholder_text(aliasTargetPlaceholder(selectedType));
                    explanationRow.set_subtitle(`${aliasTypeDescription(selectedType)} Format: ${aliasTargetExample(selectedType)}`);
                });

                const saveButton = new Gtk.Button({label: 'Save'});
                saveButton.connect('clicked', () => {
                    const alias = normalizeAlias(aliasEntry.get_text());
                    const type = ALIAS_TYPES[typeDropDown.get_selected()] ?? 'rewrite';
                    const target = parseAliasTarget(type, targetEntry.get_text());
                    if (!alias || /\s/.test(alias) || !target)
                        return;

                    const next = aliases.map((entry, entryIndex) =>
                        entryIndex === index ? {alias, type, target} : entry
                    );
                    saveAliases(next);
                });

                const deleteButton = new Gtk.Button({label: 'Delete'});
                deleteButton.connect('clicked', () => {
                    const next = aliases.filter((_, entryIndex) => entryIndex !== index);
                    saveAliases(next);
                });

                const actionsRow = new Adw.ActionRow({title: 'Actions'});
                actionsRow.set_activatable(false);
                actionsRow.add_suffix(saveButton);
                actionsRow.add_suffix(deleteButton);

                row.add_row(aliasRow);
                row.add_row(typeRow);
                row.add_row(explanationRow);
                row.add_row(targetRow);
                row.add_row(actionsRow);
                aliasRows.push(row);
                personalizationGroup.add(row);
            });
        };

        addAliasButton.connect('clicked', () => {
            const next = [...aliases, {alias: 'new', type: 'rewrite', target: {query: 'example'}}];
            saveAliases(next);
        });

        rebuildAliasRows();
        page.add(personalizationGroup);

        const advancedRankingGroup = new Adw.PreferencesGroup({
            title: 'Advanced ranking',
            description: 'Tie-break priorities when fuzzy scores are equal.',
        });

        addSpinRow(advancedRankingGroup, settings, 'weight-windows', 'Window weight', 'Prefer open windows when scores tie.', 0, 100, 1, 5);
        addSpinRow(advancedRankingGroup, settings, 'weight-apps', 'App weight', 'Prefer apps over recents when scores tie.', 0, 100, 1, 5);
        addSpinRow(advancedRankingGroup, settings, 'weight-recents', 'Recent weight', 'Boost for recents provider results.', 0, 100, 1, 5);
        addSpinRow(advancedRankingGroup, settings, 'weight-files', 'File weight', 'Boost for indexed file matches.', 0, 100, 1, 5);
        addSpinRow(advancedRankingGroup, settings, 'weight-emoji', 'Emoji weight', 'Boost for emoji matches.', 0, 100, 1, 5);
        addSpinRow(advancedRankingGroup, settings, 'weight-utility', 'Utility weight', 'Boost for calculator/currency/time rows.', 0, 100, 1, 5);

        page.add(advancedRankingGroup);

        window.add(page);
    }
}
