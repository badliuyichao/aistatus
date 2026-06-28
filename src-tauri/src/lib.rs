//! AI 余额监控 · Tauri 后端入口。
//!
//! 对应 legacy `main.py` + widget 的窗口/托盘初始化：
//!   - 建窗口（无边框/置顶/透明，配置在 tauri.conf.json）
//!   - 应用原生毛玻璃（backdrop 模块）
//!   - 锚定屏幕右下角（_anchor_to_bottom_right）
//!   - 系统托盘（显示/退出）
//!   - 启动后台拉取 + 60s 定时

pub mod backdrop;
pub mod commands;
pub mod config;
pub mod data;
mod fetcher;
mod merge;
mod state;

use std::time::Duration;

use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    Emitter, Manager, PhysicalPosition, Rect,
};

use config::load_keys;
use state::AppState;

/// 窗口距屏幕右下角的留白（对应 legacy WINDOW_SCREEN_MARGIN）。
const WINDOW_SCREEN_MARGIN: i32 = 20;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let initial_keys = load_keys();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(AppState::new(initial_keys))
        // macOS：点红绿灯红色（关闭）不退出进程，改为隐藏窗口（常驻托盘）。
        // 真正退出只能通过托盘菜单「退出」。对齐原生 mac 后台应用行为。
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                // 阻止默认关闭，仅隐藏窗口
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_services,
            commands::get_keys,
            commands::save_keys,
            commands::refresh_now,
            commands::needs_key_setup,
            commands::open_balance_file,
        ])
        .setup(|app| {
            // macOS：设为 Accessory（后台应用），不显示在 Dock 栏。
            // 配合红绿灯「关闭=隐藏窗口」+ 托盘常驻，形成标准 mac 后台应用。
            #[cfg(target_os = "macos")]
            {
                use tauri::ActivationPolicy;
                let _ = app.set_activation_policy(ActivationPolicy::Accessory);
            }

            // ── 窗口初始化：毛玻璃 + 初始定位 ──
            // 毛玻璃立即应用；定位延迟到下一帧（setup 阶段 set_position 太早，
            // 会被窗口初始化覆盖）。spawn 让出事件循环，窗口创建完成后再定位。
            // 配合 tauri.conf.json 的 visible:false：先定位到正确位置，再 show，
            // 避免窗口在屏幕中间出现后跳到目标位置（视觉跳变）。
            if let Some(window) = app.get_webview_window("main") {
                // 原生毛玻璃；失败静默（前端 CSS 降级）
                let _ = backdrop::apply(&window);

                // 窗口装饰按平台区分（config 里 decorations:true 是为 macOS 创建
                // 带红绿灯的标题栏；mac 的 titleBarStyle:Overlay + hiddenTitle 让
                // 标题栏透明、只剩红绿灯叠加在内容上）。
                // 非 mac 平台：关掉装饰，用无边框悬浮框观感。
                #[cfg(not(target_os = "macos"))]
                {
                    let _ = window.set_decorations(false);
                }

                let win = window.clone();
                let handle = app.handle().clone();
                tauri::async_runtime::spawn(async move {
                    // 让出当前事件循环，等窗口 + 托盘图标完成初始化。
                    // 注意：仅 yield_now 不够——macOS NSStatusItem 创建后需要
                    // 一次 runloop 轮回才完成菜单栏排版（把图标摆到右侧正确位置）。
                    // 提前查 rect() 会拿到排版前的临时坐标。这里补一个短延时，
                    // 让系统 runloop 有时间排版托盘图标。
                    tokio::time::sleep(Duration::from_millis(80)).await;

                    #[cfg(target_os = "macos")]
                    {
                        // 优先：跟随托盘图标真实坐标（与点击托盘弹出位置一致）。
                        // rect() 是主动查询（读 NSStatusItem.window frame），不依赖点击。
                        //
                        // macOS 已知行为：NSStatusItem 的 window.frame 启动早期可能停留在
                        // 临时坐标（如 x=0，即屏幕左侧），需要系统 runloop 刷新后才反映
                        // 真实位置。这里轮询 rect() 直到 x 落到屏幕右半区，最多等 ~1.5s，
                        // 仍异常则降级到屏幕右上角估算。
                        let screen_w = win
                            .current_monitor()
                            .ok()
                            .flatten()
                            .map(|m| m.size().width as i32)
                            .unwrap_or(0);

                        let mut positioned = false;
                        let mut rect_opt: Option<tauri::Rect> = None;
                        for _ in 0..15 {
                            if let Some(tray) = handle.tray_by_id("main") {
                                if let Ok(Some(rect)) = tray.rect() {
                                    let x = match &rect.position {
                                        tauri::Position::Physical(p) => p.x,
                                        tauri::Position::Logical(p) => p.x as i32,
                                    };
                                    // 有效坐标：x 在屏幕右半区（图标确实排在菜单栏右侧）
                                    if x > screen_w / 2 {
                                        rect_opt = Some(rect);
                                        break;
                                    }
                                }
                            }
                            tokio::time::sleep(Duration::from_millis(100)).await;
                        }

                        if let Some(rect) = rect_opt {
                            anchor_near_tray(&win, &rect);
                            positioned = true;
                        }

                        // 降级：拿不到图标坐标才用屏幕右上角估算
                        if !positioned {
                            anchor_top_right(&win);
                        }
                    }
                    #[cfg(not(target_os = "macos"))]
                    {
                        anchor_to_bottom_right(&win);
                    }

                    // 仅定位，不显示——窗口默认隐藏，用户通过托盘菜单
                    // 「打开主界面」唤起。坐标已算好，唤起时即可就位。
                });
            }

            // ── 系统托盘菜单 ──
            // 左键/右键点击托盘都弹出此菜单。窗口默认隐藏，通过「打开主界面」显示。
            let show_item = MenuItem::with_id(app, "show", "打开主界面", true, None::<&str>)?;
            let config_item = MenuItem::with_id(app, "config", "设置…", true, None::<&str>)?;
            let quit_item = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show_item, &config_item, &quit_item])?;

            let mut tray = TrayIconBuilder::with_id("main")
                .tooltip("AI API 余额监控")
                .menu(&menu)
                // 左键点击也显示菜单（与右键行为一致）
                .show_menu_on_left_click(true)
                .on_menu_event(|app, event| match event.id().as_ref() {
                    "show" => {
                        // 打开主界面：macOS 上跟随托盘图标位置弹出，再显示并聚焦
                        #[cfg(target_os = "macos")]
                        {
                            if let Some(tray) = app.tray_by_id("main") {
                                if let Ok(Some(rect)) = tray.rect() {
                                    if let Some(window) = app.get_webview_window("main") {
                                        anchor_near_tray(&window, &rect);
                                    }
                                }
                            }
                        }
                        show_main_window(app);
                    }
                    "config" => {
                        // 设置：显示主窗口并通知前端打开配置对话框
                        show_main_window(app);
                        let _ = app.emit("open-config", ());
                    }
                    "quit" => app.exit(0),
                    _ => {}
                });

            // 托盘图标：优先用打包的图标文件
            if let Some(icon) = app.default_window_icon().cloned() {
                tray = tray.icon(icon);
            }
            tray.build(app)?;

            // ── 启动时触发一次后台拉取（对应 legacy widget.request_refresh）──
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                AppState::request_refresh(handle).await;
            });

            // ── 60s 定时刷新（对应 legacy main.py 的 QTimer 60_000）──
            let handle2 = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let mut interval = tokio::time::interval(Duration::from_secs(60));
                interval.tick().await; // 跳过首次立即触发
                loop {
                    interval.tick().await;
                    AppState::request_refresh(handle2.clone()).await;
                }
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

/// 显示主窗口（托盘「显示」/ 双击）。对应 legacy `_show_window`。
fn show_main_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

/// 把窗口移到主屏工作区右下角（排除 Dock/任务栏）。
/// 对应 legacy `_anchor_to_bottom_right`。Windows 及其他平台用。
#[cfg(not(target_os = "macos"))]
fn anchor_to_bottom_right(window: &tauri::WebviewWindow) {
    let monitor = match window.current_monitor() {
        Ok(Some(m)) => m,
        _ => return,
    };
    let mon_size = monitor.size();
    let mon_pos = monitor.position();
    let win_size = match window.outer_size() {
        Ok(s) => s,
        Err(_) => return,
    };

    // 工作区右下角 = 屏幕原点 + 屏幕尺寸 - 留白；
    // 屏幕原点可能是负数（多屏），所以要加 mon_pos。
    let x = mon_pos.x as i32 + mon_size.width as i32 - win_size.width as i32 - WINDOW_SCREEN_MARGIN;
    let y = mon_pos.y as i32 + mon_size.height as i32
        - win_size.height as i32
        - WINDOW_SCREEN_MARGIN;
    // 钳制不滑出左上角
    let x = x.max(mon_pos.x as i32);
    let y = y.max(mon_pos.y as i32);
    let _ = window.set_position(PhysicalPosition { x, y });
}

/// macOS 启动默认位置：主屏右上角、菜单栏正下方。
/// （之后点击托盘图标会由 anchor_near_tray 动态跟随）
#[cfg(target_os = "macos")]
fn anchor_top_right(window: &tauri::WebviewWindow) {
    let monitor = match window.current_monitor() {
        Ok(Some(m)) => m,
        _ => return,
    };
    let mon_size = monitor.size();
    let mon_pos = monitor.position();
    let win_size = match window.outer_size() {
        Ok(s) => s,
        Err(_) => return,
    };

    // 右上角：x = 屏幕右边 - 窗口宽 - 留白；
    // y = 菜单栏高度（约 25px）+ 小留白，紧贴菜单栏下方。
    let x = mon_pos.x as i32 + mon_size.width as i32 - win_size.width as i32 - WINDOW_SCREEN_MARGIN;
    let menu_bar_h: i32 = 25; // macOS 菜单栏固定高度
    let y = mon_pos.y as i32 + menu_bar_h + 4;
    let _ = window.set_position(PhysicalPosition { x, y });
}

/// macOS：窗口跟随托盘图标弹出。
/// 水平居中于图标中心，顶部贴菜单栏下方（图标底部），并做屏幕边界钳制。
/// 这是 macOS 原生菜单栏下拉面板的标准定位方式。
#[cfg(target_os = "macos")]
fn anchor_near_tray(window: &tauri::WebviewWindow, icon_rect: &Rect) {
    let monitor = match window.current_monitor() {
        Ok(Some(m)) => m,
        _ => return,
    };
    let mon_size = monitor.size();
    let mon_pos = monitor.position();
    let win_size = match window.outer_size() {
        Ok(s) => s,
        Err(_) => return,
    };

    // 托盘图标矩形：Rect<Position>，position/size 是字段；
    // Position/Size 是枚举（Physical/Logical），这里按物理像素解包。
    let (icon_x, icon_y) = match &icon_rect.position {
        tauri::Position::Physical(p) => (p.x, p.y),
        tauri::Position::Logical(p) => (p.x as i32, p.y as i32),
    };
    let (icon_w, icon_h) = match &icon_rect.size {
        tauri::Size::Physical(s) => (s.width as i32, s.height as i32),
        tauri::Size::Logical(s) => (s.width as i32, s.height as i32),
    };
    let icon_center_x = icon_x + icon_w / 2;

    // 窗口水平居中于图标中心
    let mut x = icon_center_x - win_size.width as i32 / 2;
    // 顶部贴图标底部（即菜单栏下方）
    let y = icon_y + icon_h + 4;

    // 水平钳制：不滑出屏幕左右边界
    let min_x = mon_pos.x as i32 + WINDOW_SCREEN_MARGIN;
    let max_x = mon_pos.x as i32 + mon_size.width as i32 - win_size.width as i32 - WINDOW_SCREEN_MARGIN;
    x = x.clamp(min_x, max_x.max(min_x));

    let _ = window.set_position(PhysicalPosition { x, y });
}
