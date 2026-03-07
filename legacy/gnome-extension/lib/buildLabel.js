export function formatBuildLabel(buildId, buildHash) {
    const id = (buildId ?? '').toString().trim();
    const hash = (buildHash ?? '').toString().trim();

    if (!id && !hash)
        return 'dev local';

    if (!hash)
        return `dev ${id}`;

    if (!id)
        return `dev ${hash.slice(0, 7)}`;

    return `dev ${id} ${hash.slice(0, 7)}`;
}
