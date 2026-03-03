export class RecentsProvider {
    getResults() {
        // GNOME Shell extension sandbox access to recents varies by environment.
        // v1 keeps this provider optional and non-blocking.
        return [];
    }
}
