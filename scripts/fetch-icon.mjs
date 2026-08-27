#!/usr/bin/env node
// 下载 Lobe Icons 品牌图标到 src/assets（新增服务商时用，方法详见 AGENTS.md「品牌图标」）。
//
// 用法：node scripts/fetch-icon.mjs <slug> [保存名]
//   slug    Lobe Icons 图标标识，在 https://lobehub.com/icons 搜索（如 zhipu / minimax / xiaomimimo）
//   保存名  生成 src/assets/<保存名>.png，默认同 slug
//
// 走 unpkg CDN 拉 640×640 light PNG，失败自动换 npmmirror 国内源；
// 校验 PNG 魔数，404 页 / HTML 错误页不会落盘。
import { writeFile } from "node:fs/promises";
import path from "node:path";

const CDNS = [
  (s) => `https://unpkg.com/@lobehub/icons-static-png@latest/light/${s}.png`,
  (s) => `https://registry.npmmirror.com/@lobehub/icons-static-png/latest/files/light/${s}.png`,
];

const slug = process.argv[2];
if (!slug) {
  console.error(
    "用法: node scripts/fetch-icon.mjs <slug> [保存名]\n" +
      "例:   node scripts/fetch-icon.mjs xiaomimimo mimo\n" +
      "slug 在 https://lobehub.com/icons 搜索"
  );
  process.exit(1);
}
const name = process.argv[3] || slug;
const out = path.join(process.cwd(), "src", "assets", `${name}.png`);

let lastErr;
for (const url of CDNS) {
  try {
    const res = await fetch(url(slug), { redirect: "follow" });
    if (!res.ok) throw new Error(`HTTP ${res.status}`);
    const buf = Buffer.from(await res.arrayBuffer());
    if (buf.length < 8 || buf.readUInt32BE(0) !== 0x89504e47) {
      throw new Error("响应不是 PNG（slug 拼错了？到 lobehub.com/icons 核对）");
    }
    await writeFile(out, buf);
    console.log(`已保存 ${out}（${buf.length} 字节）`);
    console.log(`下一步: src/components/ServiceCard.svelte 的 logo() 加 n.startsWith("<前缀>") 分支`);
    process.exit(0);
  } catch (e) {
    lastErr = e;
    console.error(`[fetch-icon] ${url(slug)} 失败: ${e.message}`);
  }
}
console.error(`[fetch-icon] 所有 CDN 均失败，最后错误: ${lastErr?.message ?? lastErr}`);
process.exit(1);
