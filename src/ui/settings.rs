use eframe::egui;
use crate::config::{Config, Theme, Language};
use std::sync::Arc;
use parking_lot::RwLock;

pub struct SettingsPage {
    config: Arc<RwLock<Config>>,
}

impl SettingsPage {
    pub fn new(config: Arc<RwLock<Config>>) -> Self {
        Self {
            config,
        }
    }
    
    pub fn show_with_text(&mut self, ui: &mut egui::Ui, _settings_text: &str) -> bool {
        let mut theme_changed = false;
        let mut config_changed = false;
        
        let (title_text, theme_text, language_text, threads_text, path_text, select_folder_text, restore_text, about_text, version_text, copyright_text, license_text, disclaimer_text) = {
            let config = self.config.read();
            match config.language {
                Language::SimplifiedChinese => (
                    "设置",
                    "主题:",
                    "语言:",
                    "下载线程:",
                    "下载路径:",
                    "选择文件夹",
                    "恢复默认",
                    "关于 Bilibili-Down",
                    "版本: 0.1.0",
                    "版权: © NORMAL-EX All rights",
                    "开源协议: MIT",
                    "本软件仅供学习和研究使用",
                ),
                Language::English => (
                    "Settings",
                    "Theme:",
                    "Language:",
                    "Download Threads:",
                    "Download Path:",
                    "Select Folder",
                    "Restore Defaults",
                    "About Bilibili-Down",
                    "Version: 0.1.0",
                    "Copyright: © NORMAL-EX All rights",
                    "License: MIT",
                    "This software is for learning and research only",
                ),
            }
        };
        
        ui.heading(egui::RichText::new(title_text).size(24.0));
        ui.separator();
        ui.add_space(10.0);
        
        egui::Grid::new("settings_grid")
            .num_columns(2)
            .spacing([40.0, 10.0])
            .show(ui, |ui| {
                ui.label(egui::RichText::new(theme_text).size(16.0));
                let mut config = self.config.write();
                let old_theme = config.theme.clone();
                
                let theme_names = match config.language {
                    Language::SimplifiedChinese => ["系统默认", "浅色模式", "深色模式"],
                    Language::English => ["System Default", "Light Mode", "Dark Mode"],
                };
                
                egui::ComboBox::from_id_salt("theme_combo")
                    .selected_text(match &config.theme {
                        Theme::System => theme_names[0],
                        Theme::Light => theme_names[1],
                        Theme::Dark => theme_names[2],
                    })
                    .show_ui(ui, |ui| {
                        if ui.selectable_value(&mut config.theme, Theme::System, theme_names[0]).clicked() {
                            config_changed = true;
                        }
                        if ui.selectable_value(&mut config.theme, Theme::Light, theme_names[1]).clicked() {
                            config_changed = true;
                        }
                        if ui.selectable_value(&mut config.theme, Theme::Dark, theme_names[2]).clicked() {
                            config_changed = true;
                        }
                    });
                theme_changed = old_theme != config.theme;
                ui.end_row();
                
                ui.label(egui::RichText::new(language_text).size(16.0));
                let old_lang = config.language.clone();
                egui::ComboBox::from_id_salt("language_combo")
                    .selected_text(match &config.language {
                        Language::SimplifiedChinese => "简体中文",
                        Language::English => "English",
                    })
                    .show_ui(ui, |ui| {
                        if ui.selectable_value(&mut config.language, Language::SimplifiedChinese, "简体中文").clicked() {
                            config_changed = true;
                        }
                        if ui.selectable_value(&mut config.language, Language::English, "English").clicked() {
                            config_changed = true;
                        }
                    });
                if old_lang != config.language {
                    config_changed = true;
                }
                ui.end_row();
                
                ui.label(egui::RichText::new(threads_text).size(16.0));
                let old_threads = config.download_threads;
                let threads_label = match config.language {
                    Language::SimplifiedChinese => format!("{} 线程", config.download_threads),
                    Language::English => format!("{} Threads", config.download_threads),
                };
                egui::ComboBox::from_id_salt("threads_combo")
                    .selected_text(threads_label)
                    .show_ui(ui, |ui| {
                        let thread_options = [8u32, 16, 32];
                        for threads in thread_options {
                            let label = match config.language {
                                Language::SimplifiedChinese => format!("{} 线程", threads),
                                Language::English => format!("{} Threads", threads),
                            };
                            if ui.selectable_value(&mut config.download_threads, threads, label).clicked() {
                                config_changed = true;
                            }
                        }
                    });
                if old_threads != config.download_threads {
                    config_changed = true;
                }
                ui.end_row();
                
                ui.label(egui::RichText::new(path_text).size(16.0));
                ui.horizontal(|ui| {
                    ui.label(config.download_path.display().to_string());
                    if ui.button(select_folder_text).clicked() {
                        if let Some(path) = rfd::FileDialog::new().pick_folder() {
                            config.download_path = path;
                            config_changed = true;
                        }
                    }
                });
                ui.end_row();
            });
        
        ui.add_space(30.0);
        
        ui.horizontal(|ui| {
            if ui.button(egui::RichText::new(restore_text).size(16.0)).clicked() {
                let mut config = self.config.write();
                *config = Config::default();
                config_changed = true;
                theme_changed = true;
            }
        });
        
        if config_changed {
            self.config.read().save();
        }
        
        ui.add_space(20.0);
        ui.separator();
        ui.add_space(10.0);
        
        ui.group(|ui| {
            ui.vertical(|ui| {
                ui.label(egui::RichText::new(about_text).size(18.0).strong());
                ui.add_space(5.0);
                ui.label(version_text);
                ui.label(copyright_text);
                ui.label(license_text);
                ui.add_space(5.0);
                ui.label(disclaimer_text);
            });
        });
        
        theme_changed
    }
}