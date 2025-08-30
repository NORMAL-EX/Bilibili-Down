use eframe::egui;
use crate::downloader::{DownloadManager, DownloadTask, DownloadStatus};
use std::sync::Arc;
use parking_lot::RwLock;
use std::collections::HashMap;
use crate::config::Language;

// Windows平台特定导入
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

// Windows平台下的CREATE_NO_WINDOW常量
#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x08000000;

pub struct DownloadQueuePage {
    download_manager: Arc<DownloadManager>,
    cover_cache: HashMap<String, egui::TextureHandle>,
    cover_loading: HashMap<String, bool>,
}

impl DownloadQueuePage {
    pub fn new(download_manager: Arc<DownloadManager>) -> Self {
        Self { 
            download_manager,
            cover_cache: HashMap::new(),
            cover_loading: HashMap::new(),
        }
    }
    
    pub fn show_with_texts(&mut self, ui: &mut egui::Ui, pause_text: &str, resume_text: &str, delete_text: &str) {
        let tasks = self.download_manager.get_tasks();
        
        if tasks.is_empty() {
            let (empty_text, hint_text) = {
                let config = self.download_manager.get_config();
                let lang = config.read().language.clone();
                match lang {
                    Language::SimplifiedChinese => (
                        "没有下载任务",
                        "请先在首页解析视频并添加下载任务"
                    ),
                    Language::English => (
                        "No download tasks",
                        "Please parse a video on the home page and add a download task"
                    ),
                }
            };
            
            ui.vertical_centered(|ui| {
                ui.add_space(100.0);
                ui.label(egui::RichText::new(empty_text).size(18.0));
                ui.add_space(20.0);
                ui.label(hint_text);
            });
            return;
        }
        
        let mut actions = Vec::new();
        
        egui::ScrollArea::vertical().show(ui, |ui| {
            for task in &tasks {
                ui.add_space(5.0);
                ui.group(|ui| {
                    if let Some(action) = self.show_task(ui, task.clone(), pause_text, resume_text, delete_text) {
                        actions.push(action);
                    }
                });
            }
        });
        
        for (task_id, action) in actions {
            match action.as_str() {
                "pause" => self.download_manager.pause_task(&task_id),
                "resume" => self.download_manager.resume_task(&task_id),
                "delete" => self.download_manager.cancel_task(&task_id),
                _ => {}
            }
        }
    }
    
    fn show_task(
        &mut self,
        ui: &mut egui::Ui,
        task: Arc<RwLock<DownloadTask>>,
        pause_text: &str,
        resume_text: &str,
        delete_text: &str
    ) -> Option<(String, String)> {
        let mut action = None;
        let config = self.download_manager.get_config();
        let lang = config.read().language.clone();
        
        ui.horizontal(|ui| {
            let (task_id, task_title, task_author, is_mp3, cover_url) = {
                let task_read = task.read();
                (
                    task_read.id.clone(),
                    task_read.title.clone(),
                    task_read.author.clone(),
                    task_read.is_mp3,
                    task_read.cover.clone(),
                )
            };
            
            // 显示封面
            if !self.cover_cache.contains_key(&task_id) && !self.cover_loading.contains_key(&task_id) {
                // 开始加载封面
                self.cover_loading.insert(task_id.clone(), true);
                let ctx = ui.ctx().clone();
                let _task_id_clone = task_id.clone();
                
                // 异步加载封面
                std::thread::spawn(move || {
                    if let Ok(response) = reqwest::blocking::get(&cover_url) {
                        if let Ok(bytes) = response.bytes() {
                            if let Ok(image) = image::load_from_memory(&bytes) {
                                let rgba = image.resize(120, 67, image::imageops::FilterType::Lanczos3).to_rgba8();
                                let size = [rgba.width() as usize, rgba.height() as usize];
                                let pixels = rgba.as_flat_samples();
                                let _color_image = egui::ColorImage::from_rgba_unmultiplied(
                                    size,
                                    pixels.as_slice(),
                                );
                                
                                ctx.request_repaint();
                            }
                        }
                    }
                });
                
                // 显示占位图
                let placeholder = Self::create_placeholder_thumbnail();
                let texture = ui.ctx().load_texture(
                    format!("cover_placeholder_{}", task_id),
                    placeholder,
                    Default::default(),
                );
                ui.add(egui::Image::new(&texture)
                    .max_size(egui::Vec2::new(120.0, 67.0))
                    .rounding(3.0));
            } else if let Some(texture) = self.cover_cache.get(&task_id) {
                ui.add(egui::Image::new(texture)
                    .max_size(egui::Vec2::new(120.0, 67.0))
                    .rounding(3.0));
            } else {
                // 正在加载，显示占位图
                let placeholder = Self::create_placeholder_thumbnail();
                let texture = ui.ctx().load_texture(
                    format!("cover_loading_{}", task_id),
                    placeholder,
                    Default::default(),
                );
                ui.add(egui::Image::new(&texture)
                    .max_size(egui::Vec2::new(120.0, 67.0))
                    .rounding(3.0));
            }
            
            ui.vertical(|ui| {
                ui.label(egui::RichText::new(&task_title).size(16.0).strong());
                
                let (author_label, bv_label, format_label) = match lang {
                    Language::SimplifiedChinese => ("作者", "BV号", if is_mp3 { "格式: MP3" } else { "格式: 视频" }),
                    Language::English => ("Author", "BV ID", if is_mp3 { "Format: MP3" } else { "Format: Video" }),
                };
                
                ui.horizontal(|ui| {
                    ui.label(format!("{}: {}", author_label, task_author));
                    ui.separator();
                    ui.label(format!("{}: {}", bv_label, task_id));
                    ui.separator();
                    ui.label(format_label);
                });
                
                ui.add_space(5.0);
                
                let task_read = task.read();
                let status = task_read.status.read();
                
                match &*status {
                    DownloadStatus::Waiting => {
                        let waiting_text = match lang {
                            Language::SimplifiedChinese => "等待下载...",
                            Language::English => "Waiting to download...",
                        };
                        ui.horizontal(|ui| {
                            ui.spinner();
                            ui.label(waiting_text);
                        });
                    }
                    DownloadStatus::Downloading { progress, speed } => {
                        let downloading_text = match lang {
                            Language::SimplifiedChinese => format!("下载中: {:.1}% - 速度: {}", progress * 100.0, speed),
                            Language::English => format!("Downloading: {:.1}% - Speed: {}", progress * 100.0, speed),
                        };
                        ui.label(downloading_text);
                        ui.add(egui::ProgressBar::new(*progress)
                            .show_percentage()
                            .animate(true));
                        
                        ui.add_space(5.0);
                        ui.horizontal(|ui| {
                            if ui.button(pause_text).clicked() {
                                action = Some((task_id.clone(), "pause".to_string()));
                            }
                        });
                    }
                    DownloadStatus::Paused => {
                        let paused_text = match lang {
                            Language::SimplifiedChinese => "已暂停",
                            Language::English => "Paused",
                        };
                        ui.label(egui::RichText::new(paused_text).color(egui::Color32::from_rgb(255, 200, 0)));
                        
                        ui.add_space(5.0);
                        ui.horizontal(|ui| {
                            if ui.button(resume_text).clicked() {
                                action = Some((task_id.clone(), "resume".to_string()));
                            }
                            if ui.button(egui::RichText::new(delete_text)
                                .color(egui::Color32::from_rgb(255, 100, 100)))
                                .clicked() {
                                action = Some((task_id.clone(), "delete".to_string()));
                            }
                        });
                    }
                    DownloadStatus::Merging { progress } => {
                        let merging_text = match lang {
                            Language::SimplifiedChinese => format!("合并音视频中: {:.1}%", progress * 100.0),
                            Language::English => format!("Merging audio and video: {:.1}%", progress * 100.0),
                        };
                        ui.label(merging_text);
                        ui.add(egui::ProgressBar::new(*progress)
                            .show_percentage()
                            .animate(true));
                    }
                    DownloadStatus::Completed => {
                        let completed_text = match lang {
                            Language::SimplifiedChinese => "下载完成",
                            Language::English => "Download completed",
                        };
                        let open_folder_text = match lang {
                            Language::SimplifiedChinese => "打开文件夹",
                            Language::English => "Open Folder",
                        };
                        
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("✔").color(egui::Color32::GREEN).size(16.0));
                            ui.label(egui::RichText::new(completed_text).color(egui::Color32::GREEN));
                            if let Some(path) = &task_read.output_path {
                                if ui.button(open_folder_text).clicked() {
                                    if let Some(parent) = path.parent() {
                                        #[cfg(target_os = "windows")]
                                        {
                                            let mut cmd = std::process::Command::new("explorer");
                                            cmd.arg(parent);
                                            cmd.creation_flags(CREATE_NO_WINDOW);
                                            let _ = cmd.spawn();
                                        }
                                        #[cfg(target_os = "macos")]
                                        {
                                            let _ = std::process::Command::new("open")
                                                .arg(parent)
                                                .spawn();
                                        }
                                        #[cfg(target_os = "linux")]
                                        {
                                            let _ = std::process::Command::new("xdg-open")
                                                .arg(parent)
                                                .spawn();
                                        }
                                    }
                                }
                            }
                        });
                        
                        ui.add_space(5.0);
                        ui.horizontal(|ui| {
                            if ui.button(egui::RichText::new(delete_text)
                                .color(egui::Color32::from_rgb(255, 100, 100)))
                                .clicked() {
                                action = Some((task_id.clone(), "delete".to_string()));
                            }
                        });
                    }
                    DownloadStatus::Failed(err) => {
                        let failed_text = match lang {
                            Language::SimplifiedChinese => format!("失败: {}", err),
                            Language::English => format!("Failed: {}", err),
                        };
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("✗").color(egui::Color32::RED).size(16.0));
                            ui.label(egui::RichText::new(failed_text).color(egui::Color32::RED));
                        });
                        
                        ui.add_space(5.0);
                        ui.horizontal(|ui| {
                            if ui.button(egui::RichText::new(delete_text)
                                .color(egui::Color32::from_rgb(255, 100, 100)))
                                .clicked() {
                                action = Some((task_id.clone(), "delete".to_string()));
                            }
                        });
                    }
                }
            });
        });
        
        action
    }
    
    fn create_placeholder_thumbnail() -> egui::ColorImage {
        let width = 120;
        let height = 67;
        let mut pixels = vec![0u8; width * height * 4];
        
        for y in 0..height {
            for x in 0..width {
                let idx = (y * width + x) * 4;
                pixels[idx] = 60;
                pixels[idx + 1] = 60;
                pixels[idx + 2] = 60;
                pixels[idx + 3] = 255;
            }
        }
        
        egui::ColorImage::from_rgba_unmultiplied([width, height], &pixels)
    }
}