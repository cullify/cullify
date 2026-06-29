// Tauri serves static assets from the bundled frontend, so Cullify runs as an
// SPA with an index fallback for desktop deep links such as /cull/ or /settings/.
// See: https://svelte.dev/docs/kit/single-page-apps
// See: https://v2.tauri.app/start/frontend/sveltekit/ for more info
import adapter from "@sveltejs/adapter-static";
import { vitePreprocess } from "@sveltejs/vite-plugin-svelte";

/** @type {import('@sveltejs/kit').Config} */
const config = {
  preprocess: vitePreprocess(),
  kit: {
    adapter: adapter({
      fallback: "index.html",
    }),
    paths: {
      relative: false,
    },
    version: {
      name: "0.1.0",
    },
  },
};

export default config;
