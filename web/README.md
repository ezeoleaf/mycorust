# mycorust on the web

A cozy in-browser mycelium garden. Open `index.html` and a spore drifts down, germinates, and starts exploring. Tap the soil to leave sugar crumbs. Seasons turn on their own; weather (rain, sun, warm, cold, snow) changes the light, the soil, and how fast the network grows. Use **sky** to peek at the next weather.

This folder is static on purpose so GitHub Pages can host it with no build step.

## Local

```bash
cd web
python3 -m http.server 8765
```

Then open http://localhost:8765

## GitHub Pages

The workflow in `.github/workflows/pages.yml` uploads this folder on every push to `main` that touches `web/`.

In the GitHub repo:

1. **Settings → Pages**
2. Source: **GitHub Actions**

The first run can also be started with **Actions → Deploy web → Run workflow**.
