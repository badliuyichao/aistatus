// 版本号单一真相源同步：package.json.version → tauri.conf.json + Cargo.toml。
// 挂在 tauri.conf.json 的 beforeBuildCommand 里，每次 `tauri build` 前自动跑，
// 保证三处版本号永远一致；改版本号只需改 package.json 一处。
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const version = JSON.parse(
  fs.readFileSync(path.join(root, "package.json"), "utf8")
).version;

let changed = false;

// 1) tauri.conf.json —— 整体 JSON 读写
const tauriPath = path.join(root, "src-tauri", "tauri.conf.json");
const tauri = JSON.parse(fs.readFileSync(tauriPath, "utf8"));
if (tauri.version !== version) {
  tauri.version = version;
  fs.writeFileSync(tauriPath, JSON.stringify(tauri, null, 2) + "\n", "utf8");
  changed = true;
}

// 2) Cargo.toml —— 只替换 [package] 段首个 version 行，保留注释与格式
//    （dependencies 里的 `xxx = { version = ".." }` 行首不是 version，不会被命中）
const cargoPath = path.join(root, "src-tauri", "Cargo.toml");
let cargo = fs.readFileSync(cargoPath, "utf8");
const re = /^(\s*version\s*=\s*")[^"]*(")/m;
if (re.test(cargo) && !cargo.includes(`version = "${version}"`)) {
  cargo = cargo.replace(re, `$1${version}$2`);
  fs.writeFileSync(cargoPath, cargo, "utf8");
  changed = true;
}

console.log(
  changed
    ? `[sync-version] 已同步到 ${version}`
    : `[sync-version] 三处版本一致 (${version})，无需改动`
);
