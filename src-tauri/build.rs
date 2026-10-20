// 编译期构建脚本：
//   1. 调用 Tauri 标准构建流程
//   2. 读 secrets.enc（密文，已提交 git）→ 生成 OUT_DIR/secrets.rs 字节数组常量
//      明文 key 永远不出现在源码或二进制字符串表里（运行期由 secrets.rs XOR 解密）。
//
// 密文来源：scripts/encrypt-keys.mjs 读本机 keys.local.json 加密产出。
// 若 secrets.enc 缺失（如全新 clone 且未放置 key），编译报错并提示如何生成。

use std::env;
use std::fs;
use std::path::Path;

fn main() {
    tauri_build::build();

    // 读密文。secrets.enc 缺失 / 字段不全时按空处理（警告不 panic）：
    // API Key 已改为运行期配置（设置对话框），内置 key 仅作为升级迁移的
    // 播种源——全新 clone 不放置 key 也能构建，只是首次启动无种子实例。
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let enc_path = Path::new(&manifest_dir).join("secrets.enc");

    let payload: serde_json::Value = match fs::read_to_string(&enc_path) {
        Ok(body) => serde_json::from_str(&body).expect("secrets.enc 解析失败"),
        Err(_) => {
            println!(
                "cargo:warning=secrets.enc 缺失（{}），内置 key 为空；\
                 API Key 可在应用设置对话框运行期配置，或用 keys.local.json 运行 \
                 `node scripts/encrypt-keys.mjs` 生成密文作为首次启动种子。",
                enc_path.display()
            );
            serde_json::json!({})
        }
    };

    // XOR 主密钥是单个 base64 字符串；三家密文是数组（新格式，同品牌多 key），
    // 兼容旧版单字符串格式（视为单元素数组）。缺失 → 空列表（无播种）。
    let key = payload
        .get("key")
        .and_then(|v| v.as_str())
        .map(base64_decode)
        .unwrap_or_default();
    let enc_glm = optional_field(&payload, "encGlm");
    let enc_minimax = optional_field(&payload, "encMinimax");
    let enc_deepseek = optional_field(&payload, "encDeepseek");

    // 生成 secrets.rs：XOR 密钥 + 三家密文列表（同品牌多 key，按序对应
    // keys.local.json 里该品牌数组的顺序，运行期按 builtin 序号取用）。
    // 编进二进制的是数字数组字面量，不是可读字符串。
    let out_dir = env::var("OUT_DIR").unwrap();
    let dest = Path::new(&out_dir).join("secrets.rs");
    let src = format!(
        "// 由 build.rs 自动生成，勿手改。\n\
         pub const XOR_KEY: &[u8] = &{k};\n\
         pub const ENC_GLM: &[&[u8]] = &{g};\n\
         pub const ENC_MINIMAX: &[&[u8]] = &{m};\n\
         pub const ENC_DEEPSEEK: &[&[u8]] = &{d};\n",
        k = fmt_byte_array(&key),
        g = fmt_nested_byte_arrays(&enc_glm),
        m = fmt_nested_byte_arrays(&enc_minimax),
        d = fmt_nested_byte_arrays(&enc_deepseek),
    );

    fs::write(&dest, src).expect("写 secrets.rs 失败");

    // secrets.enc 改了要重新生成
    println!("cargo:rerun-if-changed=secrets.enc");
}

/// 取 base64 密文列表：数组（新格式）逐项解码；兼容旧版单字符串格式。
/// 缺失 / 非法 → 空列表（build.rs 不再因字段缺失失败）。
fn optional_field(payload: &serde_json::Value, name: &str) -> Vec<Vec<u8>> {
    match payload.get(name) {
        Some(serde_json::Value::Array(arr)) => arr
            .iter()
            .filter_map(|v| v.as_str())
            .map(base64_decode)
            .collect(),
        Some(serde_json::Value::String(s)) => vec![base64_decode(s)],
        _ => Vec::new(),
    }
}

/// 极简 base64 解码（不引入 base64 crate，避免给 build-dependencies 加负担）。
fn base64_decode(s: &str) -> Vec<u8> {
    let s: String = s.chars().filter(|c| !c.is_whitespace()).collect();
    let table: [i16; 256] = {
        let mut t = [-1i16; 256];
        let alpha = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        for (i, &c) in alpha.iter().enumerate() {
            t[c as usize] = i as i16;
        }
        t
    };
    let bytes: Vec<u8> = s.into_bytes();
    let mut out = Vec::with_capacity(bytes.len() * 3 / 4);
    for chunk in bytes.chunks(4) {
        let vals: Vec<i16> = chunk.iter().map(|&c| table[c as usize]).collect();
        let b0 = vals[0];
        let b1 = vals[1];
        if b0 < 0 || b1 < 0 {
            break;
        }
        out.push(((b0 as u32) << 2 | (b1 as u32) >> 4) as u8);
        if chunk.len() > 2 && vals[2] >= 0 {
            out.push((((b1 as u32) & 0x0F) << 4 | (vals[2] as u32) >> 2) as u8);
            if chunk.len() > 3 && vals[3] >= 0 {
                out.push((((vals[2] as u32) & 0x03) << 6 | vals[3] as u32) as u8);
            }
        }
    }
    out
}

/// 字节切片格式化为 Rust 数组字面量：[0x12, 0xAB, ...]
fn fmt_byte_array(bs: &[u8]) -> String {
    let inner: Vec<String> = bs.iter().map(|b| format!("0x{:02x}", b)).collect();
    format!("[{}]", inner.join(", "))
}

/// 密文列表格式化为嵌套 Rust 数组字面量（内层为引用）：[&[0x12, ...], &[0xAB, ...]]
fn fmt_nested_byte_arrays(list: &[Vec<u8>]) -> String {
    let inner: Vec<String> = list
        .iter()
        .map(|bs| format!("&{}", fmt_byte_array(bs)))
        .collect();
    format!("[{}]", inner.join(", "))
}
