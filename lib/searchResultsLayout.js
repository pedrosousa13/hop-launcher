function asArray(items) {
    return Array.isArray(items) ? items : [];
}

export function splitTailItems(items) {
    const rankedItems = [];
    const tailItems = [];

    for (const item of asArray(items)) {
        if (item?.appendToEnd)
            tailItems.push(item);
        else
            rankedItems.push(item);
    }

    return {rankedItems, tailItems};
}

export function combineRankedWithTail(rankedItems, tailItems, maxResults) {
    const limit = Number.isFinite(maxResults) ? Math.max(1, Math.floor(maxResults)) : 10;
    const ranked = asArray(rankedItems);
    const tail = asArray(tailItems);
    const tailLimit = Math.min(limit, tail.length);
    const rankedLimit = Math.max(0, limit - tailLimit);
    return [...ranked.slice(0, rankedLimit), ...tail.slice(0, tailLimit)];
}
