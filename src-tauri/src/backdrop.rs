//! 平台原生毛玻璃 —— mac NSVisualEffectView vibrancy + Win acrylic/mica。
//!
//! 对应 legacy widget.py 的 `_try_enable_acrylic`（多级降级）。
//! 失败时静默返回，前端降级为 CSS 半透明卡片（.bg-card 已是半透明底）。
//!
//! window-vibrancy crate 提供跨平台 API：
//!   - mac:  apply_vibrancy（NSVisualEffectView）
//!   - win:  apply_mica / apply_acrylic（DWM）

#[cfg(target_os = "macos")]
use window_vibrancy::{apply_vibrancy, NSVisualEffectMaterial, NSVisualEffectState};

/// 对主窗口应用原生毛玻璃。
/// 成功返回 true（前端可用更透明的卡片底色）；失败返回 false（前端走降级底色）。
/// 返回 bool 通过事件告诉前端，但当前简化：直接尽力应用，前端 CSS 自适应。
pub fn apply(window: &tauri::WebviewWindow) -> bool {
    apply_platform(window)
}

#[cfg(target_os = "macos")]
fn apply_platform(window: &tauri::WebviewWindow) -> bool {
    match apply_vibrancy(
        window,
        // Sidebar：类似系统侧栏的半透明材质，悬浮框观感合适
        NSVisualEffectMaterial::Sidebar,
        // FollowsWindowActiveState：随窗口激活态变化
        Some(NSVisualEffectState::FollowsWindowActiveState),
        None, // 圆角交给窗口自身裁剪
    ) {
        Ok(_) => true,
        // 任何错误（平台/版本不支持）都走 CSS 降级，不区分变体名
        Err(e) => {
            eprintln!("[backdrop] mac vibrancy failed: {e:?}");
            false
        }
    }
}

#[cfg(target_os = "windows")]
fn apply_platform(window: &tauri::WebviewWindow) -> bool {
    // 先试 mica（Win11 22H2+），失败降级 acrylic（Win10/11 通用）
    use window_vibrancy::{apply_acrylic, apply_mica};
    if apply_mica(window, None).is_ok() {
        return true;
    }
    match apply_acrylic(window, Some((18, 18, 28, 125))) {
        Ok(_) => true,
        Err(e) => {
            eprintln!("[backdrop] acrylic failed: {e:?}");
            false
        }
    }
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
fn apply_platform(_window: &tauri::WebviewWindow) -> bool {
    false // Linux 等无原生特效，走 CSS 降级
}
