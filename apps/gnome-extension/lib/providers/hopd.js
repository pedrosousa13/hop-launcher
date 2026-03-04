const DEFAULT_TTL_MS = 5 * 1000;

function normalizeQuery(query) {
    return (query ?? '').toString().trim();
}

function cacheKey(query, mode) {
    return `${mode}:${normalizeQuery(query).toLowerCase()}`;
}

function shouldHandleQuery(query, mode) {
    if (mode === 'apps' || mode === 'windows' || mode === 'files' || mode === 'settings' || mode === 'recents')
        return true;
    if (mode === 'weather' || mode === 'timezone' || mode === 'emoji')
        return true;
    if (mode !== 'all')
        return false;

    const q = query.toLowerCase();
    return q.startsWith('weather ') ||
        q.startsWith('wx ') ||
        q.endsWith(' weather') ||
        q.startsWith('time ') ||
        q.startsWith('time in ') ||
        q.startsWith('tz ') ||
        q.startsWith('emoji ') ||
        q.startsWith(':emoji ');
}

function mapKind(kind) {
    if (kind === 'app')
        return 'app';
    if (kind === 'window')
        return 'window';
    if (kind === 'file')
        return 'file';
    return 'utility';
}

function buildPendingRow(query, key) {
    return {
        kind: 'utility',
        id: `hopd-pending:${key}`,
        primaryText: `Searching via hopd: ${query}`,
        secondaryText: 'hopd daemon',
        copyText: '',
        execute: () => {},
    };
}

function buildErrorRow(query, key, reason = 'request failed') {
    return {
        kind: 'utility',
        id: `hopd-error:${key}`,
        primaryText: `hopd unavailable for "${query}"`,
        secondaryText: reason,
        copyText: '',
        execute: () => {},
    };
}

export class HopdProvider {
    constructor(options = {}) {
        this._requestIpc = options.requestIpc ?? (async () => {
            throw new Error('hopd transport unavailable');
        });
        this._ttlMs = Number.isFinite(options.ttlMs) ? options.ttlMs : DEFAULT_TTL_MS;
        this._limit = Number.isFinite(options.limit) ? options.limit : 8;
        this._cache = new Map();
        this._inflight = new Map();
        this._onUpdate = null;
        this._destroyed = false;
        this._convergenceEnabled = Boolean(options.convergenceEnabled);
        this._isConvergenceEnabled = typeof options.isConvergenceEnabled === 'function'
            ? options.isConvergenceEnabled
            : null;
    }

    setUpdateCallback(callback) {
        this._onUpdate = typeof callback === 'function' ? callback : null;
    }

    _isStale(entry) {
        return !entry || (Date.now() - entry.updatedAtMs) > this._ttlMs;
    }

    setConvergenceEnabled(enabled) {
        this._convergenceEnabled = Boolean(enabled);
        this._cache.clear();
        this._onUpdate?.();
    }

    _convergenceEnabledNow() {
        if (this._isConvergenceEnabled)
            return Boolean(this._isConvergenceEnabled());
        return this._convergenceEnabled;
    }

    _toRows(items = []) {
        return items.map(item => {
            const id = (item?.id ?? '').toString();
            const kind = (item?.kind ?? '').toString();
            const title = (item?.title ?? '').toString() || id;
            return {
                kind: mapKind(kind),
                id: `hopd:${id}`,
                primaryText: title,
                secondaryText: `hopd • ${kind || 'result'}`,
                copyText: title,
                execute: () => this._requestIpc({
                    method: 'actions.execute',
                    params: {
                        result_id: id,
                        action: 'enter',
                    },
                }),
            };
        });
    }

    _startFetch(query, mode, key) {
        if (this._destroyed || this._inflight.has(key))
            return;

        const run = Promise.resolve()
            .then(() => this._requestIpc({
                method: 'search.query',
                params: {
                    query,
                    mode,
                    limit: this._limit,
                },
            }))
            .then(response => {
                if (this._destroyed)
                    return;
                const rows = this._toRows(response?.result?.results ?? []);
                this._cache.set(key, {
                    rows,
                    error: null,
                    updatedAtMs: Date.now(),
                });
                this._onUpdate?.();
            })
            .catch(error => {
                if (this._destroyed)
                    return;
                this._cache.set(key, {
                    rows: null,
                    error: (error?.message ?? 'request failed').toString(),
                    updatedAtMs: Date.now(),
                });
                this._onUpdate?.();
            })
            .finally(() => {
                this._inflight.delete(key);
            });

        this._inflight.set(key, run);
    }

    getResults(query, mode = 'all') {
        if (this._destroyed)
            return [];

        const normalized = normalizeQuery(query);
        if (!normalized)
            return [];
        const convergenceEnabled = this._convergenceEnabledNow();
        const isConvergenceMode = mode === 'apps' || mode === 'windows' || mode === 'files' || mode === 'settings' || mode === 'recents';
        if (!convergenceEnabled && isConvergenceMode)
            return [];
        if (!shouldHandleQuery(normalized, mode))
            return [];

        const key = cacheKey(normalized, mode);
        const cached = this._cache.get(key);
        const stale = this._isStale(cached);

        if (!stale && cached) {
            if (Array.isArray(cached.rows))
                return cached.rows;
            return [buildErrorRow(normalized, key, cached.error)];
        }

        this._startFetch(normalized, mode, key);
        if (cached) {
            if (Array.isArray(cached.rows))
                return cached.rows;
            return [buildErrorRow(normalized, key, cached.error)];
        }

        return [buildPendingRow(normalized, key)];
    }

    destroy() {
        this._destroyed = true;
        this._onUpdate = null;
        this._inflight.clear();
    }
}
