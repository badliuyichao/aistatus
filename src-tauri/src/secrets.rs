//! 内置 API Key 解密模块（**仅作内置实例的凭据源**）。
//!
//! 编译期 build.rs 把 secrets.enc 的密文转成字节数组常量（XOR_KEY /
//! ENC_GLM / ENC_MINIMAX / ENC_DEEPSEEK，各为「密文列表」）编进二进制——
//! 明文 key 不以字符串形式出现在可执行文件里，避免被 strings / 静态扫描
//! 直接读出（提高门槛档，见 encrypt-keys.mjs 的安全说明）。
//!
//! 内置 key 与「分发」场景绑定：config.json 里的内置实例只存 `{builtin: 序号}`
//! 引用（不落盘明文），运行期拉取时经本模块解密、仅在内存短暂存在。
//! 用户自行添加的实例 key 不经过这里（明文存 config.json，见 config.rs）。
//!
//! 每品牌支持多个 key（同品牌多实例）：`glm_keys()` 等按序返回，
//! 序号即 config.json 中 `builtin` 字段的取值。ENC_DEEPSEEK 为可选：
//! secrets.enc 没有该字段时为空列表（无内置 DeepSeek key）。

// 引入编译期生成的密文常量（OUT_DIR/secrets.rs）。
include!(concat!(env!("OUT_DIR"), "/secrets.rs"));

/// XOR 解密：密文按密钥循环异或还原明文。
fn decrypt(enc: &[u8], key: &[u8]) -> Vec<u8> {
    enc.iter()
        .zip(key.iter().cycle())
        .map(|(b, k)| b ^ k)
        .collect()
}

/// 解密一个密文列表。
fn decrypt_list(list: &[&[u8]]) -> Vec<String> {
    list.iter()
        .map(|enc| String::from_utf8_lossy(&decrypt(enc, XOR_KEY)).into_owned())
        .collect()
}

/// 内置 GLM key 明文列表（序号 = config.json 内置实例的 builtin 字段）。
pub fn glm_keys() -> Vec<String> {
    decrypt_list(ENC_GLM)
}

/// 内置 MiniMax key 明文列表。
pub fn minimax_keys() -> Vec<String> {
    decrypt_list(ENC_MINIMAX)
}

/// 内置 DeepSeek key 明文列表；未内置（secrets.enc 无 encDeepseek）时为空。
pub fn deepseek_keys() -> Vec<String> {
    decrypt_list(ENC_DEEPSEEK)
}
