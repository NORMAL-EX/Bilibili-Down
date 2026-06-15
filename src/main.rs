// main.rs
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

    #[cfg(target_os = "windows")]
    {
        use winreg::enums::*;
        use winreg::RegKey;

        let app_id = "BilibiliDown.App";
        let display_name = "Bilibili-Down";
        let exe_path = std::env::current_exe().unwrap_or_default();
        let exe_str = exe_path.to_string_lossy().to_string();

        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        if let Ok((app_key, _)) =
            hkcu.create_subkey(format!("Software\\Classes\\AppUserModelId\\{}", app_id))
        {
            let _ = app_key.set_value("DisplayName", &display_name);
            let _ = app_key.set_value("IconUri", &exe_str);
            let _ = app_key.set_value("IconBackgroundColor", &"2196F3");
            let _ = app_key.set_value("ShowInSettings", &1u32);
        }

        // FFI 调用 SetCurrentProcessExplicitAppUserModelID
        use std::ffi::OsStr;
        use std::os::windows::ffi::OsStrExt;
        use winapi::um::winnt::LPCWSTR;

        #[link(name = "shell32")]
        extern "system" {
            fn SetCurrentProcessExplicitAppUserModelID(appID: LPCWSTR);
        }

        let app_id_wide: Vec<u16> = OsStr::new(app_id)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();

        unsafe {
            SetCurrentProcessExplicitAppUserModelID(app_id_wide.as_ptr());
        }
    }

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1000.0, 650.0])
            .with_resizable(false)
            .with_maximize_button(false)
            .with_icon(load_icon()),
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

fn load_icon() -> egui::IconData {
    #[cfg(target_os = "windows")]
    {
        let icon_bytes = include_bytes!("../assets/bilibili.ico");

        let icon_dir = ico::IconDir::read(std::io::Cursor::new(icon_bytes))
            .unwrap_or_else(|_| ico::IconDir::new(ico::ResourceType::Icon));

        if let Some(entry) = icon_dir.entries().first() {
            if let Ok(image) = entry.decode() {
                let rgba = image.rgba_data().to_vec();
                let width = image.width();
                let height = image.height();

                return egui::IconData { rgba, width, height };
            }
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        // 非Windows平台尝试加载PNG图标
        let png_bytes = include_bytes!("../assets/bilibili.png");
        if let Ok(img) = image::load_from_memory(png_bytes) {
            let rgba = img.to_rgba8();
            let width = rgba.width();
            let height = rgba.height();
            return egui::IconData {
                rgba: rgba.into_raw(),
                width,
                height,
            };
        }
    }

    // 回退：生成纯色图标
    egui::IconData {
        rgba: vec![255; 32 * 32 * 4],
        width: 32,
        height: 32,
    }
}
