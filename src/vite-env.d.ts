/// <reference types="svelte" />
/// <reference types="vite/client" />

// 让 TS 认识图片 import（Vite 处理资源导入）
declare module "*.png" {
  const src: string;
  export default src;
}
declare module "*.jpg" {
  const src: string;
  export default src;
}
declare module "*.svg" {
  const src: string;
  export default src;
}

// 构建时注入的版本标识（见 vite.config.js 的 define），运行时为字符串常量。
declare const __APP_VERSION__: string;
declare const __BUILD_TIME__: string;
declare const __GIT_HASH__: string;
