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

/// 对窗口应用系统原生圆角（仅 Win11 22000+）。
///
/// 无边框窗口 DWM 默认画直角；显式 `DWMWCP_ROUND` 让系统画圆角（半径 ~8 逻辑像素，
/// DPI 自适应），与 Win11 自带应用一致——mica/acrylic 毛玻璃随之圆角，无锯齿。
/// Win10 该属性不存在，调用失败静默忽略（窗口保持直角，符合 Win10 原生）。
///
/// 这是让「无边框透明窗口 + 原生毛玻璃」获得真圆角的唯一干净方式：CSS border-radius
/// 只裁 CSS 层，毛玻璃是 DWM 合成层、会从 CSS 圆角缝隙外露直角。
#[cfg(target_os = "windows")]
pub fn apply_rounded_corners(window: &tauri::WebviewWindow) {
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    use windows_sys::Win32::Graphics::Dwm::{
        DwmSetWindowAttribute, DWMWA_WINDOW_CORNER_PREFERENCE, DWM_WINDOW_CORNER_PREFERENCE,
        DWMWCP_ROUND,
    };

    let Ok(handle) = window.window_handle() else { return; };
    let RawWindowHandle::Win32(win32) = handle.as_raw() else { return; };
    // HWND = *mut c_void；raw-window-handle 给 NonZeroIsize，.get() → isize → 指针
    let hwnd = win32.hwnd.get() as *mut core::ffi::c_void;

    let pref: DWM_WINDOW_CORNER_PREFERENCE = DWMWCP_ROUND;
    unsafe {
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWA_WINDOW_CORNER_PREFERENCE as u32,
            &pref as *const _ as *const core::ffi::c_void,
            std::mem::size_of::<DWM_WINDOW_CORNER_PREFERENCE>() as u32,
        );
    }
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
