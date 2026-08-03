// 编译期 API Key 加密工具（本地手动运行一次）。
//
// 用途：把本机明文 key（src-tauri/keys.local.json，已 .gitignore）用随机 XOR 密钥
// 加密成密文，写入 src-tauri/secrets.enc（此文件提交 git，仅含密文）。
// 编译时 build.rs 读 secrets.enc 生成 Rust 字节数组常量，运行期 secrets.rs 解密。
//
// 用法：node scripts/encrypt-keys.mjs
//
// 安全说明：XOR 不是密码学强加密，目的是挡住 strings / 静态扫描（提高门槛档），
// 让明文不以可读字符串出现在二进制里。逆向高手理论上仍可还原（客户端密钥铁律）。
// 换 key 时：改 keys.local.json 后重跑本脚本，再重新编译。

import { readFileSync, writeFileSync, existsSync } from "node:fs";
import { randomBytes } from "node:crypto";
import { resolve, dirname } from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = dirname(fileURLToPath(import.meta.url));
const root = resolve(__dirname, "..");

const srcPath = resolve(root, "src-tauri/keys.local.json");
const outPath = resolve(root, "src-tauri/secrets.enc");

if (!existsSync(srcPath)) {
  console.error(`✗ 找不到明文 key 源：${srcPath}`);
  console.error("  请先创建该文件（格式 {glm, minimax}），并加入 .gitignore。");
  process.exit(1);
}

const { glm, minimax } = JSON.parse(readFileSync(srcPath, "utf8"));
if (!glm || !minimax) {
  console.error("✗ keys.local.json 缺少 glm 或 minimax 字段");
  process.exit(1);
}

// XOR 加密：明文按密钥循环异或。密钥够长（32 字节）足以覆盖 key 前缀特征。
function xorEncrypt(plain, key) {
  const buf = Buffer.from(plain, "utf8");
  const out = Buffer.alloc(buf.length);
  for (let i = 0; i < buf.length; i++) {
    out[i] = buf[i] ^ key[i % key.length];
  }
  return out;
}

const key = randomBytes(32); // 每次重新生成，密文随之变化
const encGlm = xorEncrypt(glm, key);
const encMinimax = xorEncrypt(minimax, key);

// secrets.enc 格式：JSON，含 key/encGlm/encMinimax 的 base64。
// 提交此文件到 git 是安全的——只有密文与密钥，没有明文。
const payload = {
  key: key.toString("base64"),
  encGlm: encGlm.toString("base64"),
  encMinimax: encMinimax.toString("base64"),
};

writeFileSync(outPath, JSON.stringify(payload, null, 2) + "\n", "utf8");
console.log(`✓ 已加密并写入 ${outPath}`);
console.log(`  GLM 密文 ${encGlm.length} 字节，MiniMax 密文 ${encMinimax.length} 字节`);
console.log(`  该文件可安全提交 git（仅含密文）。keys.local.json 切勿提交。`);
