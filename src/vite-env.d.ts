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
