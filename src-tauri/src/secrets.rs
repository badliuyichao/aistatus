//! 内置 API Key 解密模块。
//!
//! 编译期 build.rs 把 secrets.enc 的密文转成字节数组常量（XOR_KEY / ENC_GLM /
//! ENC_MINIMAX），编进二进制——明文 key 不以字符串形式出现在可执行文件里，
//! 避免被 strings / 静态扫描直接读出（提高门槛档）。
//!
//! 运行期 fetcher 调用本模块的 glm_key() / minimax_key() 取明文 key 发请求。
//! 明文仅在该函数返回的 String 里短暂存在，用完即弃。

// 引入编译期生成的密文常量（OUT_DIR/secrets.rs）。
include!(concat!(env!("OUT_DIR"), "/secrets.rs"));

/// XOR 解密：密文按密钥循环异或还原明文。
fn decrypt(enc: &[u8], key: &[u8]) -> Vec<u8> {
    enc.iter()
        .zip(key.iter().cycle())
        .map(|(b, k)| b ^ k)
        .collect()
}

/// 内置 GLM key 明文。
pub fn glm_key() -> String {
    String::from_utf8_lossy(&decrypt(ENC_GLM, XOR_KEY)).into_owned()
}

/// 内置 MiniMax key 明文。
pub fn minimax_key() -> String {
    String::from_utf8_lossy(&decrypt(ENC_MINIMAX, XOR_KEY)).into_owned()
}
