export function getResultHintIconSpec(kind) {
    if (kind === 'window') {
        return {
            relativePath: 'assets/icons/lucide/monitor.svg',
            fallbackIconName: 'focus-windows-symbolic',
        };
    }

    if (kind === 'app') {
        return {
            relativePath: 'assets/icons/lucide/app-window.svg',
            fallbackIconName: 'application-x-executable-symbolic',
        };
    }

    if (kind === 'file') {
        return {
            relativePath: 'assets/icons/lucide/files.svg',
            fallbackIconName: 'text-x-generic-symbolic',
        };
    }

    return null;
}

export function getResultHintActionLabel(kind, enterActionType) {
    if (enterActionType === 'copy')
        return 'Copy';
    if (enterActionType !== 'execute')
        return 'Enter';

    if (kind === 'window')
        return 'Focus';
    if (kind === 'app' || kind === 'file')
        return 'Open';
    if (kind === 'action')
        return 'Run';

    return 'Enter';
}
