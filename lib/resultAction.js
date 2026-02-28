function normalizeText(value) {
    return (value ?? '').toString();
}

function inferEmojiGlyph(primaryText) {
    const text = normalizeText(primaryText).trim();
    if (!text)
        return '';
    return text.split(/\s+/)[0];
}

export function resolveEnterAction(result) {
    if (!result)
        return {type: 'none'};

    if (result.kind === 'utility') {
        const text = normalizeText(result.copyText || result.primaryText).trim();
        return text ? {type: 'copy', text} : {type: 'none'};
    }

    if (result.kind === 'emoji') {
        const text = normalizeText(result.copyText || inferEmojiGlyph(result.primaryText)).trim();
        return text ? {type: 'copy', text} : {type: 'none'};
    }

    if (typeof result.execute === 'function')
        return {type: 'execute'};

    return {type: 'none'};
}

