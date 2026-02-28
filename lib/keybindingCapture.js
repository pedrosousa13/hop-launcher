export function sanitizeModifierState(state, mask) {
    return state & mask;
}

export function resolveTypedAccelerator(value, fallback, isValid) {
    const accel = value.trim() || fallback;
    return isValid(accel) ? accel : null;
}

export function interpretKeybindingPress({keyval, mods, keyNames}) {
    if (keyval === keyNames.escape)
        return {kind: 'cancel'};

    if (keyval === keyNames.backSpace || keyval === keyNames.delete)
        return {kind: 'clear'};

    if (mods === 0 || keyNames.modifiers.has(keyval))
        return {kind: 'ignore'};

    return {kind: 'set', keyval, mods};
}
