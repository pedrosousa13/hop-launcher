import {TIMEZONE_ALIASES} from './data/timezone-aliases.js';

function normalizeToken(value) {
    return (value ?? '')
        .toString()
        .trim()
        .toLowerCase()
        .replace(/\s+/g, '_');
}

function normalizeIana(iana) {
    return (iana ?? '').toString().trim().toLowerCase().replace(/\//g, '_');
}

let cachedLookup = null;

function buildLookup() {
    const entries = [];
    const seenAliasIana = new Set();
    const exactTokens = new Set();

    const addEntry = (alias, iana) => {
        const normalizedAlias = normalizeToken(alias);
        const normalizedIana = normalizeIana(iana);
        const ianaLower = (iana ?? '').toString().toLowerCase();
        if (!normalizedAlias || !normalizedIana || !ianaLower)
            return;

        const key = `${normalizedAlias}:${ianaLower}`;
        if (seenAliasIana.has(key))
            return;
        seenAliasIana.add(key);

        entries.push({
            alias: normalizedAlias,
            iana,
            ianaLower,
            normalizedIana,
        });
        exactTokens.add(normalizedAlias);
        exactTokens.add(normalizedIana);
    };

    for (const [alias, iana] of Object.entries(TIMEZONE_ALIASES))
        addEntry(alias, iana);

    if (typeof Intl?.supportedValuesOf === 'function') {
        let zones = [];
        try {
            zones = Intl.supportedValuesOf('timeZone');
        } catch (_) {
            zones = [];
        }

        for (const zone of zones) {
            const iana = zone.toString();
            const normalizedIana = normalizeIana(iana);
            addEntry(normalizedIana, iana);

            const cityToken = normalizedIana.split('_').filter(Boolean).at(-1);
            if (cityToken && cityToken.length >= 2)
                addEntry(cityToken, iana);
        }
    }

    return {entries, exactTokens};
}

function getLookup() {
    if (!cachedLookup)
        cachedLookup = buildLookup();
    return cachedLookup;
}

function tokenFromQueryToken(queryToken) {
    return normalizeToken(queryToken);
}

export function hasExactTimezoneToken(queryToken) {
    const token = tokenFromQueryToken(queryToken);
    if (!token)
        return false;
    const {exactTokens} = getLookup();
    return exactTokens.has(token);
}

export function findTimezoneMatches(queryToken) {
    const token = tokenFromQueryToken(queryToken);
    if (!token)
        return [];

    const ianaToken = token.replace(/_/g, '/');
    const {entries} = getLookup();
    const matches = entries.filter(entry =>
        entry.alias.includes(token) ||
        entry.normalizedIana.includes(token) ||
        entry.ianaLower.includes(ianaToken)
    );

    matches.sort((a, b) => {
        const aExact = (a.alias === token || a.normalizedIana === token) ? 0 : 1;
        const bExact = (b.alias === token || b.normalizedIana === token) ? 0 : 1;
        if (aExact !== bExact)
            return aExact - bExact;
        return a.alias.localeCompare(b.alias);
    });

    const deduped = [];
    const seenIana = new Set();
    for (const match of matches) {
        if (seenIana.has(match.ianaLower))
            continue;
        seenIana.add(match.ianaLower);
        deduped.push({alias: match.alias, iana: match.iana});
    }

    return deduped;
}
