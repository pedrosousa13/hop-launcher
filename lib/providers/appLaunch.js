function canUseWindow(window) {
    if (!window)
        return false;
    if (window.skip_taskbar || window.minimized)
        return false;
    if (window.is_override_redirect?.())
        return false;
    return typeof window.activate === 'function';
}

function focusExistingAppWindow(app, nowProvider) {
    const windows = app?.get_windows?.() ?? [];
    const candidate = windows.find(canUseWindow);
    if (!candidate)
        return false;

    candidate.activate(nowProvider());
    return true;
}

export function launchOrFocusApp(app, nowProvider = () => global.get_current_time()) {
    if (!app)
        return false;

    if (focusExistingAppWindow(app, nowProvider))
        return true;

    if (typeof app.activate === 'function') {
        app.activate();
        return true;
    }

    if (typeof app.open_new_window === 'function') {
        app.open_new_window(-1);
        return true;
    }

    if (typeof app.launch === 'function') {
        app.launch([], null);
        return true;
    }

    const appInfo = app.get_app_info?.();
    if (appInfo?.launch) {
        appInfo.launch([], null);
        return true;
    }

    return false;
}
