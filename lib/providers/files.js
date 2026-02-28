import Gio from 'gi://Gio';
import GLib from 'gi://GLib';

import {filterIndexedEntries} from '../index/fileIndexer.js';

const MAX_INDEX_ENTRIES = 5000;
const MAX_SCAN_DEPTH = 6;

function displayName(path) {
    const base = GLib.path_get_basename(path);
    return base || path;
}

function toStrvFolders(settings) {
    if (!settings?.get_strv)
        return [];
    const raw = settings.get_strv('indexed-folders');
    return raw.filter(Boolean);
}

function enumeratePath(entries, rootPath) {
    const stack = [{path: rootPath, depth: 0}];

    while (stack.length > 0 && entries.length < MAX_INDEX_ENTRIES) {
        const current = stack.pop();
        const file = Gio.File.new_for_path(current.path);
        let enumerator = null;

        try {
            enumerator = file.enumerate_children(
                'standard::name,standard::type,time::modified',
                Gio.FileQueryInfoFlags.NONE,
                null
            );

            let info = null;
            while ((info = enumerator.next_file(null)) !== null && entries.length < MAX_INDEX_ENTRIES) {
                const name = info.get_name();
                const childPath = GLib.build_filenamev([current.path, name]);
                const fileType = info.get_file_type();

                if (fileType === Gio.FileType.DIRECTORY) {
                    if (current.depth + 1 <= MAX_SCAN_DEPTH)
                        stack.push({path: childPath, depth: current.depth + 1});
                    continue;
                }

                if (fileType !== Gio.FileType.REGULAR)
                    continue;

                entries.push({
                    path: childPath,
                    name,
                    mtime: info.get_modification_date_time()?.to_unix?.() ?? 0,
                });
            }
        } catch (_) {
            // Skip unreadable directories and continue indexing.
        } finally {
            try {
                enumerator?.close(null);
            } catch (_) {}
        }
    }
}

export class FilesProvider {
    constructor(settings) {
        this._settings = settings;
        this._indexed = [];
    }

    refresh() {
        const folders = toStrvFolders(this._settings);
        const entries = [];
        for (const folder of folders)
            enumeratePath(entries, folder);
        this._indexed = entries;
    }

    getResults(query, mode = 'all') {
        if (mode !== 'all' && mode !== 'files')
            return [];

        const routeAllowsFiles = (query ?? '').trim().length > 0;
        if (!routeAllowsFiles)
            return [];

        if (this._indexed.length === 0)
            this.refresh();

        const maxResults = this._settings?.get_int?.('max-results') ?? 12;
        const matches = filterIndexedEntries(this._indexed, query, maxResults);
        return matches.map(entry => ({
            kind: 'file',
            id: `file:${entry.path}`,
            primaryText: entry.name,
            secondaryText: entry.path,
            execute: () => {
                const uri = Gio.File.new_for_path(entry.path).get_uri();
                Gio.AppInfo.launch_default_for_uri(uri, null);
            },
            _searchHaystack: `${entry.name} ${entry.path} ${displayName(entry.path)}`,
        }));
    }
}
