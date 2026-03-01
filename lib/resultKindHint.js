export function getResultHintIconName(kind) {
    if (kind === 'window')
        return 'focus-windows-symbolic';
    if (kind === 'app')
        return 'application-x-executable-symbolic';
    if (kind === 'file')
        return 'text-x-generic-symbolic';
    return null;
}
