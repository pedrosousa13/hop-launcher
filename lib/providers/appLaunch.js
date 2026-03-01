function resolveBoolFlag(value) {
    if (typeof value === 'function')
        return Boolean(value());
    return Boolean(value);
}

function normalizeToken(value) {
    return (value ?? '')
        .toString()
        .trim()
        .toLowerCase()
        .replace(/\.desktop$/, '');
}

function canUseWindow(window) {
    if (!window)
        return false;
    if (resolveBoolFlag(window.skip_taskbar) || resolveBoolFlag(window.is_skip_taskbar))
        return false;
    if (window.is_override_redirect?.())
        return false;
    return typeof window.activate === 'function';
}

function windowMatchesApp(window, app) {
    const appId = normalizeToken(app?.get_id?.());
    const appExec = normalizeToken(app?.get_app_info?.()?.get_executable?.());
    const appName = normalizeToken(app?.get_name?.());

    if (!appId && !appExec && !appName)
        return false;

    const gtkAppId = normalizeToken(window?.get_gtk_application_id?.());
    const wmClass = normalizeToken(window?.get_wm_class?.());
    const wmClassInstance = normalizeToken(window?.get_wm_class_instance?.());

    if (appId && (gtkAppId === appId || wmClass === appId || wmClassInstance === appId))
        return true;
    if (appExec && (gtkAppId === appExec || wmClass === appExec || wmClassInstance === appExec))
        return true;
    if (appName && (gtkAppId === appName || wmClass === appName || wmClassInstance === appName))
        return true;

    return false;
}

function getDisplayWindows() {
    try {
        return global?.display?.get_tab_list?.(0, null) ?? [];
    } catch (_error) {
        return [];
    }
}

function focusExistingAppWindow(app, nowProvider) {
    const appWindows = app?.get_windows?.() ?? [];
    let candidate = appWindows.find(canUseWindow);
    if (!candidate) {
        const displayWindows = getDisplayWindows();
        candidate = displayWindows.find(window => canUseWindow(window) && windowMatchesApp(window, app));
    }

    if (!candidate)
        return false;

    const now = nowProvider();
    if (candidate.minimized && typeof candidate.unminimize === 'function')
        candidate.unminimize(now);
    candidate.activate(now);
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
