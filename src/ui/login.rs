use eframe::egui;
use crate::bilibili::{BilibiliApi, LoginStatus};
use std::sync::Arc;
use tokio::runtime::Runtime;
use std::time::{SystemTime, UNIX_EPOCH};

pub struct LoginWindow {
    api: Arc<BilibiliApi>,
    runtime: Arc<Runtime>,
    qrcode_image: Option<egui::TextureHandle>,
    qrcode_key: Option<String>,
    qrcode_url: Option<String>,
    status: LoginStatus,
    checking: bool,
    last_check_time: u64,
}

impl LoginWindow {
    pub fn new(api: Arc<BilibiliApi>, runtime: Arc<Runtime>) -> Self {
        Self {
            api,
            runtime,
            qrcode_image: None,
            qrcode_key: None,
            qrcode_url: None,
            status: LoginStatus::Waiting,
            checking: false,
            last_check_time: 0,
        }
    }
    
    pub fn show(&mut self, ui: &mut egui::Ui) -> Option<String> {
        let mut cookies = None;
        
        ui.vertical_centered(|ui| {
            ui.heading(egui::RichText::new("B站账号登录").size(20.0));
            ui.separator();
            ui.add_space(10.0);
            
            if self.qrcode_image.is_none() {
                if ui.button(egui::RichText::new("生成登录二维码").size(16.0)).clicked() {
                    self.generate_qrcode(ui.ctx());
                }
                
                ui.add_space(10.0);
                ui.label("点击按钮生成二维码，使用B站手机APP扫码登录");
            } else {
                ui.label(egui::RichText::new("请使用手机B站APP扫描二维码登录").size(14.0));
                ui.add_space(10.0);
                
                if let Some(texture) = &self.qrcode_image {
                    ui.add(egui::Image::new(texture).max_size(egui::Vec2::new(256.0, 256.0)));
                }
                
                ui.add_space(10.0);
                
                match &self.status {
                    LoginStatus::Waiting => {
                        ui.horizontal(|ui| {
                            ui.spinner();
                            ui.label("等待扫码...");
                        });
                        self.check_status();
                    }
                    LoginStatus::Scanned => {
                        ui.horizontal(|ui| {
                            ui.spinner();
                            ui.label(egui::RichText::new("已扫码，请在手机上确认")
                                .color(egui::Color32::from_rgb(0, 150, 255)));
                        });
                        self.check_status();
                    }
                    LoginStatus::Success { cookies: c } => {
                        ui.label(egui::RichText::new("✔ 登录成功！")
                            .color(egui::Color32::GREEN)
                            .size(16.0));
                        cookies = Some(c.clone());
                    }
                    LoginStatus::Expired => {
                        ui.label(egui::RichText::new("二维码已过期")
                            .color(egui::Color32::RED));
                        if ui.button("重新生成").clicked() {
                            self.generate_qrcode(ui.ctx());
                        }
                    }
                }
                
                ui.add_space(10.0);
                
                if matches!(self.status, LoginStatus::Waiting | LoginStatus::Scanned) {
                    if ui.button("取消登录").clicked() {
                        self.qrcode_image = None;
                        self.qrcode_key = None;
                        self.qrcode_url = None;
                        self.status = LoginStatus::Waiting;
                        self.checking = false;
                    }
                }
            }
        });
        
        cookies
    }
    
    fn generate_qrcode(&mut self, ctx: &egui::Context) {
        let api = self.api.clone();
        let runtime = self.runtime.clone();
        
        let handle = runtime.spawn(async move {
            api.generate_qrcode().await
        });
        
        if let Ok(result) = runtime.block_on(handle) {
            if let Ok((url, key)) = result {
                self.qrcode_url = Some(url.clone());
                self.qrcode_key = Some(key);
                
                let qr_image = self.create_qrcode_image(&url);
                self.qrcode_image = Some(ctx.load_texture(
                    "qrcode",
                    qr_image,
                    Default::default(),
                ));
                self.status = LoginStatus::Waiting;
                self.checking = false;
            }
        }
    }
    
    fn check_status(&mut self) {
        if self.checking || self.qrcode_key.is_none() {
            return;
        }
        
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
        if now - self.last_check_time < 2 {
            return;
        }
        
        self.last_check_time = now;
        self.checking = true;
        
        let api = self.api.clone();
        let key = self.qrcode_key.clone().unwrap();
        let runtime = self.runtime.clone();
        
        let handle = runtime.spawn(async move {
            api.poll_qrcode(&key).await
        });
        
        if let Ok(result) = runtime.block_on(handle) {
            if let Ok(status) = result {
                self.status = status;
            }
        }
        
        self.checking = false;
    }
    
    fn create_qrcode_image(&self, url: &str) -> egui::ColorImage {
        use qrcode::{QrCode, EcLevel};
        
        let code = QrCode::with_error_correction_level(url, EcLevel::M).unwrap_or_else(|_| {
            QrCode::with_error_correction_level("https://www.bilibili.com", EcLevel::M).unwrap()
        });
        
        let image = code.render::<image::Luma<u8>>()
            .quiet_zone(true)
            .module_dimensions(8, 8)
            .build();
        
        let width = image.width() as usize;
        let height = image.height() as usize;
        let mut pixels = vec![0u8; width * height * 4];
        
        for (i, pixel) in image.pixels().enumerate() {
            let idx = i * 4;
            let value = pixel[0];
            pixels[idx] = value;
            pixels[idx + 1] = value;
            pixels[idx + 2] = value;
            pixels[idx + 3] = 255;
        }
        
        egui::ColorImage::from_rgba_unmultiplied([width, height], &pixels)
    }
}