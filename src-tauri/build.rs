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

    // 读密文
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let enc_path = Path::new(&manifest_dir).join("secrets.enc");

    let payload: serde_json::Value = match fs::read_to_string(&enc_path) {
        Ok(body) => serde_json::from_str(&body).expect("secrets.enc 解析失败"),
        Err(_) => {
            panic!(
                "找不到 secrets.enc（{}）。请用明文 keys.local.json 运行 \
                 `node scripts/encrypt-keys.mjs` 生成。",
                enc_path.display()
            )
        }
    };

    // base64 解码为字节
    let key = base64_decode(&payload["key"].as_str().expect("secrets.enc 缺 key"));
    let enc_glm = base64_decode(&payload["encGlm"].as_str().expect("secrets.enc 缺 encGlm"));
    let enc_minimax = base64_decode(
        &payload["encMinimax"]
            .as_str()
            .expect("secrets.enc 缺 encMinimax"),
    );

    // 生成 secrets.rs：三个 pub const 字节数组（密钥 + 两家密文）。
    // 编进二进制的是数字数组字面量，不是可读字符串。
    let out_dir = env::var("OUT_DIR").unwrap();
    let dest = Path::new(&out_dir).join("secrets.rs");
    let src = format!(
        "// 由 build.rs 自动生成，勿手改。\n\
         pub const XOR_KEY: &[u8] = &{k};\n\
         pub const ENC_GLM: &[u8] = &{g};\n\
         pub const ENC_MINIMAX: &[u8] = &{m};\n",
        k = fmt_byte_array(&key),
        g = fmt_byte_array(&enc_glm),
        m = fmt_byte_array(&enc_minimax),
    );

    fs::write(&dest, src).expect("写 secrets.rs 失败");

    // secrets.enc 改了要重新生成
    println!("cargo:rerun-if-changed=secrets.enc");
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
