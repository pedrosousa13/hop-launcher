import Gio from 'gi://Gio';
import GLib from 'gi://GLib';

import {filterIndexedEntries} from '../index/fileIndexer.js';

const MAX_INDEX_ENTRIES = 5000;
const MAX_SCAN_DEPTH = 6;
const FILE_ATTRS = 'standard::name,standard::type,time::modified';

// Promisification mutates GJS prototypes globally, so keep it at module scope and do it once.
Gio._promisify(Gio.File.prototype, 'enumerate_children_async', 'enumerate_children_finish');
Gio._promisify(Gio.FileEnumerator.prototype, 'next_files_async', 'next_files_finish');
Gio._promisify(Gio.FileEnumerator.prototype, 'close_async', 'close_finish');

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

async function enumeratePathAsync(entries, rootPath, cancellable, isDestroyed) {
    const stack = [{path: rootPath, depth: 0}];

    while (stack.length > 0 && entries.length < MAX_INDEX_ENTRIES) {
        if (isDestroyed())
            return;

        const current = stack.pop();
        const file = Gio.File.new_for_path(current.path);
        let enumerator = null;

        try {
            enumerator = await file.enumerate_children_async(
                FILE_ATTRS,
                Gio.FileQueryInfoFlags.NONE,
                GLib.PRIORITY_DEFAULT,
                cancellable
            );

            while (entries.length < MAX_INDEX_ENTRIES) {
                if (isDestroyed())
                    return;

                const infos = await enumerator.next_files_async(
                    64,
                    GLib.PRIORITY_DEFAULT,
                    cancellable
                );
                if (!infos || infos.length === 0)
                    break;

                for (const info of infos) {
                    if (entries.length >= MAX_INDEX_ENTRIES)
                        break;

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
            }
        } catch (_) {
            // Skip unreadable directories and continue indexing.
        } finally {
            try {
                await enumerator?.close_async(GLib.PRIORITY_DEFAULT, cancellable);
            } catch (error) {
                console.debug(`[hop-launcher] file enumerator close failed for ${current.path}: ${error}`);
            }
        }
    }
}

export class FilesProvider {
    constructor(settings) {
        this._settings = settings;
        this._indexed = [];
        this.refreshOnOpen = false;
        this._onUpdate = null;
        this._refreshPromise = null;
        this._destroyed = false;
        this._cancellable = new Gio.Cancellable();
    }

    refresh() {
        if (this._refreshPromise || this._destroyed)
            return;

        this._refreshPromise = this._refresh()
            .catch(() => {
                this._indexed = [];
            })
            .finally(() => {
                this._refreshPromise = null;
                if (!this._destroyed)
                    this._onUpdate?.();
            });
    }

    async _refresh() {
        const folders = toStrvFolders(this._settings);
        const entries = [];
        for (const folder of folders) {
            await enumeratePathAsync(
                entries,
                folder,
                this._cancellable,
                () => this._destroyed || this._cancellable.is_cancelled()
            );
            if (this._destroyed || this._cancellable.is_cancelled())
                return;
        }
        this._indexed = entries;
    }

    setUpdateCallback(callback) {
        this._onUpdate = typeof callback === 'function' ? callback : null;
    }

    getResults(query, mode = 'all') {
        if (this._destroyed || (mode !== 'all' && mode !== 'files'))
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

    destroy() {
        this._destroyed = true;
        this._onUpdate = null;
        this._cancellable.cancel();
    }
}
