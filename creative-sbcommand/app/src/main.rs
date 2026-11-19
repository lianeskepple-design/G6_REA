#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod pages;
mod state;
mod theme;

use app::AuroraApp;

fn parse_screenshot_arg() -> Option<String> {
    let args: Vec<String> = std::env::args().collect();
    let mut i = 1;
    while i < args.len() {
        if args[i] == "--screenshot" {
            if i + 1 < args.len() {
                return Some(args[i + 1].clone());
            }
            return Some("screenshot.png".to_string());
        }
        if let Some(rest) = args[i].strip_prefix("--screenshot=") {
            return Some(rest.to_string());
        }
        i += 1;
    }
    None
}

fn main() -> eframe::Result<()> {
    let screenshot = parse_screenshot_arg();
    let screenshot_mode = screenshot.is_some();

    let mut viewport = eframe::egui::ViewportBuilder::default()
        .with_title("Aurora Audio Command")
        .with_inner_size([1109.0, 624.0])
        .with_min_inner_size([1000.0, 590.0])
        .with_resizable(false)
        .with_decorations(false)
        .with_active(!screenshot_mode);

    #[cfg(target_os = "windows")]
    if screenshot_mode {
        viewport = viewport.with_window_level(eframe::egui::WindowLevel::AlwaysOnBottom);
    }

    #[cfg_attr(not(target_os = "macos"), allow(unused_mut))]
    let mut native_options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };

    #[cfg(target_os = "macos")]
    if screenshot_mode {
        native_options.event_loop_builder = Some(Box::new(|builder| {
            builder.with_activate_ignoring_other_apps(false);
        }));
    }

    eframe::run_native(
        "Aurora Audio Command",
        native_options,
        Box::new(move |cc| Ok(Box::new(AuroraApp::new(cc, screenshot.clone())))),
    )
}
