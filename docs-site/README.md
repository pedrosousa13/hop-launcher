# Hop Launcher Docs Site

Modern Astro docs + storytelling site for Hop Launcher.

## Development

From `docs-site/`:

```bash
npm install
npm run dev
```

## Verification

```bash
npm run astro -- check
npm run build
```

## Cloudflare Deployment

Cloudflare adapter is configured via `@astrojs/cloudflare` and `wrangler.jsonc`.

```bash
npm run build
npx wrangler deploy
```

## Notes

- The homepage includes an interactive launcher demo for key query flows.
- Docs are routed as separate pages under `/docs/*` using Astro content collections.
- Primary CTA is `View Features`.
- Installation messaging is intentionally a placeholder (`Install (Official Channel Soon)`) until official release channels are ready.
