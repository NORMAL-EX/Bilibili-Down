#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod config;
mod downloader;
mod bilibili;
mod ui;

use eframe::egui;

fn main() -> Result<(), eframe::Error> {
    #[cfg(not(debug_assertions))]
    {
        // Release模式下不初始化日志
    }
    #[cfg(debug_assertions)]
    {
        env_logger::init();
    }
    
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1000.0, 650.0])
            .with_resizable(false)
            .with_maximize_button(false),
        ..Default::default()
    };
    
    eframe::run_native(
        "Bilibili-Down",
        options,
        Box::new(|cc| {
            egui_extras::install_image_loaders(&cc.egui_ctx);
            Ok(Box::new(app::BilibiliDownApp::new(cc)))
        }),
    )
}