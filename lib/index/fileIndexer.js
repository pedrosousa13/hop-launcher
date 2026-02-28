function normalize(text) {
    return (text ?? '').toString().toLowerCase();
}

export function scoreFileMatch(query, fileName, filePath) {
    const q = normalize(query).trim();
    if (!q)
        return 0;

    const name = normalize(fileName);
    const path = normalize(filePath);

    let score = 0;
    if (name === q)
        score += 120;
    if (name.startsWith(q))
        score += 70;
    if (name.includes(q))
        score += 45;
    if (path.includes(q))
        score += 20;

    const compactName = name.replace(/[^a-z0-9]/g, '');
    const compactQuery = q.replace(/[^a-z0-9]/g, '');
    if (compactQuery && compactName.includes(compactQuery))
        score += 20;

    score -= Math.max(0, name.length - q.length) * 0.1;
    return score;
}

export function filterIndexedEntries(entries, query, maxResults = 20) {
    const scored = entries
        .map(entry => ({
            ...entry,
            score: scoreFileMatch(query, entry.name, entry.path),
        }))
        .filter(entry => Number.isFinite(entry.score) && entry.score > 0)
        .sort((a, b) => b.score - a.score || a.name.localeCompare(b.name));

    return scored.slice(0, maxResults);
}
