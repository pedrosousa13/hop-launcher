import {EMOJI_KEYWORDS} from '../data/emoji-keywords.js';

function normalize(text) {
    return (text ?? '').toString().trim().toLowerCase();
}

function toTerm(query) {
    const q = normalize(query);
    if (q.startsWith(':emoji '))
        return q.slice(7).trim();
    return q;
}

export function shouldHandleEmojiQuery(query) {
    const q = normalize(query);
    return q.startsWith(':emoji ') || q.length >= 2;
}

export function searchEmoji(query) {
    if (!shouldHandleEmojiQuery(query))
        return [];

    const term = toTerm(query);
    if (!term)
        return [];

    const ranked = EMOJI_KEYWORDS
        .map(entry => {
            const haystack = `${entry.name} ${entry.keywords.join(' ')}`;
            const exactName = entry.name === term ? 1000 : 0;
            const exactKeyword = entry.keywords.includes(term) ? 800 : 0;
            const startsName = entry.name.startsWith(term) ? 300 : 0;
            const contains = haystack.includes(term) ? 100 : 0;
            const score = exactName + exactKeyword + startsName + contains;
            return {...entry, score};
        })
        .filter(item => item.score > 0)
        .sort((a, b) => b.score - a.score || a.name.localeCompare(b.name));

    return ranked;
}

export class EmojiProvider {
    getResults(query) {
        const rows = searchEmoji(query).slice(0, 12);
        return rows.map(item => ({
            kind: 'emoji',
            id: `emoji:${item.emoji}`,
            primaryText: `${item.emoji} ${item.name}`,
            secondaryText: item.keywords.join(', '),
            execute: () => {},
        }));
    }
}
