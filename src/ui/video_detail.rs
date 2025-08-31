use eframe::egui;
use crate::bilibili::{BilibiliApi, VideoInfo, QualityInfo};
use crate::config::{Config, Language};
use std::sync::Arc;
use tokio::runtime::Runtime;
use std::sync::mpsc;
use parking_lot::RwLock;

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
    config: Arc<RwLock<Config>>,
}

impl VideoDetailWindow {
    pub fn new(video_info: VideoInfo, api: Arc<BilibiliApi>, runtime: Arc<Runtime>, config: Arc<RwLock<Config>>) -> Self {
        let available_qualities: Vec<usize> = video_info.qualities
            .iter()
            .enumerate()
            .filter(|(_, q)| q.is_available)
            .map(|(i, _)| i)
            .collect();
        
        let selected_quality = available_qualities.first().copied().unwrap_or(0);
        
        let mut window = Self {
            video_info: video_info.clone(),
            selected_quality,
            api: api.clone(),
            runtime: runtime.clone(),
            cover_texture: None,
            cover_receiver: None,
            config,
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
    
    fn get_text(&self, key: &str) -> String {
        let lang = self.config.read().language.clone();
        match lang {
            Language::SimplifiedChinese => {
                match key {
                    "up_owner" => "UP主".to_string(),
                    "bv_id" => "BV号".to_string(),
                    "video_description" => "视频简介".to_string(),
                    "select_quality" => "选择画质".to_string(),
                    "quality_unavailable" => "该画质不可用".to_string(),
                    _ => key.to_string(),
                }
            }
            Language::English => {
                match key {
                    "up_owner" => "UP".to_string(),
                    "bv_id" => "BV ID".to_string(),
                    "video_description" => "Video Description".to_string(),
                    "select_quality" => "Select Quality".to_string(),
                    "quality_unavailable" => "This quality is unavailable".to_string(),
                    _ => key.to_string(),
                }
            }
        }
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
                    
                    let up_owner_text = self.get_text("up_owner");
                    let bv_id_text = self.get_text("bv_id");
                    
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new(format!("{}:", up_owner_text)).strong());
                        ui.label(&self.video_info.owner.name);
                    });
                    
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new(format!("{}:", bv_id_text)).strong());
                        ui.label(&self.video_info.bvid);
                    });
                });
            });
            
            ui.add_space(10.0);
            ui.separator();
            ui.add_space(10.0);
            
            ui.group(|ui| {
                ui.vertical(|ui| {
                    let video_desc_text = self.get_text("video_description");
                    ui.label(egui::RichText::new(format!("{}:", video_desc_text)).strong());
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
                let select_quality_text = self.get_text("select_quality");
                ui.label(egui::RichText::new(format!("{}:", select_quality_text)).strong());
                
                let current_quality = &self.video_info.qualities[self.selected_quality];
                let display_text = if current_quality.is_available {
                    current_quality.desc.clone()
                } else {
                    format!("{} ({})", current_quality.desc, self.get_text("quality_unavailable"))
                };
                
                egui::ComboBox::from_id_salt("quality_select")
                    .selected_text(&display_text)
                    .show_ui(ui, |ui| {
                        for (i, quality) in self.video_info.qualities.iter().enumerate() {
                            let is_selectable = quality.is_available;
                            
                            ui.add_enabled_ui(is_selectable, |ui| {
                                let label_text = if is_selectable {
                                    quality.desc.clone()
                                } else {
                                    quality.desc.clone()
                                };
                                
                                let label = if is_selectable {
                                    egui::RichText::new(label_text)
                                } else {
                                    egui::RichText::new(label_text).color(egui::Color32::from_rgb(128, 128, 128))
                                };
                                
                                if ui.selectable_label(self.selected_quality == i, label).clicked() && is_selectable {
                                    self.selected_quality = i;
                                }
                            });
                        }
                    });
            });
            
            if !self.video_info.qualities[self.selected_quality].is_available {
                ui.colored_label(egui::Color32::from_rgb(255, 100, 100), self.get_text("quality_unavailable"));
            }
            
            ui.add_space(20.0);
            ui.separator();
            ui.add_space(20.0);
            
            ui.horizontal(|ui| {
                let is_quality_available = self.video_info.qualities[self.selected_quality].is_available;
                
                ui.add_enabled_ui(is_quality_available, |ui| {
                    if ui.button(egui::RichText::new(download_video_text)
                        .size(16.0)
                        .color(if is_quality_available { 
                            egui::Color32::from_rgb(100, 200, 255) 
                        } else { 
                            egui::Color32::from_rgb(128, 128, 128) 
                        }))
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
                        .color(if is_quality_available { 
                            egui::Color32::from_rgb(100, 255, 150) 
                        } else { 
                            egui::Color32::from_rgb(128, 128, 128) 
                        }))
                        .clicked() {
                        result = Some((
                            self.video_info.clone(),
                            self.video_info.qualities[self.selected_quality].clone(),
                            DownloadType::Mp3,
                        ));
                    }
                });
                
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