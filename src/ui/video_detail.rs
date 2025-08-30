use eframe::egui;
use crate::bilibili::{BilibiliApi, VideoInfo, QualityInfo};
use std::sync::Arc;
use tokio::runtime::Runtime;
use std::sync::mpsc;

#[derive(Debug, Clone, PartialEq)]
pub enum DownloadType {
    Video,
    Mp3,
}

pub struct VideoDetailWindow {
    video_info: VideoInfo,
    selected_quality: usize,
    api: Arc<BilibiliApi>,
    runtime: Arc<Runtime>,
    cover_texture: Option<egui::TextureHandle>,
    cover_receiver: Option<mpsc::Receiver<Vec<u8>>>,
}

impl VideoDetailWindow {
    pub fn new(video_info: VideoInfo, api: Arc<BilibiliApi>, runtime: Arc<Runtime>) -> Self {
        let mut window = Self {
            video_info: video_info.clone(),
            selected_quality: 0,
            api: api.clone(),
            runtime: runtime.clone(),
            cover_texture: None,
            cover_receiver: None,
        };
        
        window.load_cover();
        window
    }
    
    fn load_cover(&mut self) {
        let cover_url = self.video_info.cover.clone();
        let api = self.api.clone();
        let (tx, rx) = mpsc::channel();
        self.cover_receiver = Some(rx);
        
        self.runtime.spawn(async move {
            if let Ok(bytes) = api.download_avatar(&cover_url).await {
                let _ = tx.send(bytes);
            }
        });
    }
    
    fn create_placeholder_cover() -> egui::ColorImage {
        let width = 320;
        let height = 180;
        let mut pixels = vec![0u8; width * height * 4];
        
        for y in 0..height {
            for x in 0..width {
                let idx = (y * width + x) * 4;
                pixels[idx] = (100 + (x * 100 / width)) as u8;
                pixels[idx + 1] = (100 + (y * 100 / height)) as u8;
                pixels[idx + 2] = 150;
                pixels[idx + 3] = 255;
            }
        }
        
        egui::ColorImage::from_rgba_unmultiplied([width, height], &pixels)
    }
    
    pub fn show_with_texts(
        &mut self,
        ui: &mut egui::Ui,
        download_video_text: &str,
        download_mp3_text: &str,
        cancel_text: &str
    ) -> Option<(VideoInfo, QualityInfo, DownloadType)> {
        let mut result = None;
        let mut should_close = false;
        
        if let Some(receiver) = &self.cover_receiver {
            if let Ok(cover_bytes) = receiver.try_recv() {
                if let Ok(image) = image::load_from_memory(&cover_bytes) {
                    let rgba = image.to_rgba8();
                    let size = [rgba.width() as usize, rgba.height() as usize];
                    let pixels = rgba.as_flat_samples();
                    let color_image = egui::ColorImage::from_rgba_unmultiplied(
                        size,
                        pixels.as_slice(),
                    );
                    self.cover_texture = Some(ui.ctx().load_texture(
                        "video_cover",
                        color_image,
                        Default::default(),
                    ));
                } else {
                    let cover = Self::create_placeholder_cover();
                    self.cover_texture = Some(ui.ctx().load_texture(
                        "video_cover",
                        cover,
                        Default::default(),
                    ));
                }
                self.cover_receiver = None;
            }
        }
        
        ui.vertical(|ui| {
            ui.horizontal(|ui| {
                if let Some(texture) = &self.cover_texture {
                    ui.add(egui::Image::new(texture)
                        .max_size(egui::Vec2::new(320.0, 180.0))
                        .rounding(5.0));
                } else {
                    ui.group(|ui| {
                        ui.set_min_size(egui::Vec2::new(320.0, 180.0));
                        ui.centered_and_justified(|ui| {
                            ui.spinner();
                        });
                    });
                }
                
                ui.vertical(|ui| {
                    ui.label(egui::RichText::new(&self.video_info.title).size(18.0).strong());
                    ui.add_space(10.0);
                    
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("UP主:").strong());
                        ui.label(&self.video_info.owner.name);
                    });
                    
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("BV号:").strong());
                        ui.label(&self.video_info.bvid);
                    });
                });
            });
            
            ui.add_space(10.0);
            ui.separator();
            ui.add_space(10.0);
            
            ui.group(|ui| {
                ui.vertical(|ui| {
                    ui.label(egui::RichText::new("视频简介:").strong());
                    ui.add_space(5.0);
                    egui::ScrollArea::vertical()
                        .max_height(100.0)
                        .show(ui, |ui| {
                            ui.label(&self.video_info.desc);
                        });
                });
            });
            
            ui.add_space(10.0);
            ui.separator();
            ui.add_space(10.0);
            
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("选择画质:").strong());
                egui::ComboBox::from_id_salt("quality_select")
                    .selected_text(&self.video_info.qualities[self.selected_quality].desc)
                    .show_ui(ui, |ui| {
                        for (i, quality) in self.video_info.qualities.iter().enumerate() {
                            ui.selectable_value(&mut self.selected_quality, i, &quality.desc);
                        }
                    });
            });
            
            ui.add_space(20.0);
            ui.separator();
            ui.add_space(20.0);
            
            ui.horizontal(|ui| {
                if ui.button(egui::RichText::new(download_video_text)
                    .size(16.0)
                    .color(egui::Color32::from_rgb(100, 200, 255)))
                    .clicked() {
                    result = Some((
                        self.video_info.clone(),
                        self.video_info.qualities[self.selected_quality].clone(),
                        DownloadType::Video,
                    ));
                }
                
                ui.add_space(10.0);
                
                if ui.button(egui::RichText::new(download_mp3_text)
                    .size(16.0)
                    .color(egui::Color32::from_rgb(100, 255, 150)))
                    .clicked() {
                    result = Some((
                        self.video_info.clone(),
                        self.video_info.qualities[self.selected_quality].clone(),
                        DownloadType::Mp3,
                    ));
                }
                
                ui.add_space(10.0);
                
                if ui.button(egui::RichText::new(cancel_text)
                    .size(16.0))
                    .clicked() {
                    should_close = true;
                }
            });
        });
        
        if should_close {
            None
        } else {
            result
        }
    }
}