// 纯 Svelte + Vite（非 SvelteKit）。本项目是单页悬浮框，不需要路由/SSR/adapter。
import { vitePreprocess } from "@sveltejs/vite-plugin-svelte";

export default {
  preprocess: vitePreprocess(),
};
