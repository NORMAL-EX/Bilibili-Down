#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod config;
mod downloader;
mod bilibili;
mod ui;

use eframe::egui;

fn main() -> Result<(), eframe::Error> {
    env_logger::init();
    
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1000.0, 650.0])
           // .with_min_inner_size([600.0, 500.0])
           // .with_max_inner_size([700.0, 500.0])
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