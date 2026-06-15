// src\ui\home.rs
use eframe::egui;
use crate::config::Language;

pub struct HomePage {
    pub input: String,  // 改为pub，允许外部访问
}

impl HomePage {
    pub fn new() -> Self {
        Self {
            input: String::new(),
        }
    }
    
    #[allow(dead_code)]
    pub fn show_with_texts(
        &mut self,
        ui: &mut egui::Ui,
        parse_video_text: &str,
        disclaimer1: &str,
        disclaimer2: &str,
        input_hint: &str,
        parse_btn_text: &str,
    ) -> Option<String> {
        let mut parse_requested = None;
        
        // 获取可用宽度
        let available_width = ui.available_width();
        let content_width = 500.0_f32.min(available_width - 40.0);
        let side_margin = (available_width - content_width) / 2.0;
        
        ui.vertical_centered(|ui| {
            ui.add_space(50.0);
            
            ui.heading(egui::RichText::new(parse_video_text).size(24.0));
            
            ui.add_space(30.0);
            
            // 使用边距来居中内容
            ui.horizontal(|ui| {
                ui.add_space(side_margin);
                ui.group(|ui| {
                    ui.set_width(content_width);
                    ui.vertical_centered(|ui| {
                        ui.label(egui::RichText::new(disclaimer1)
                            .color(egui::Color32::from_rgb(255, 100, 100))
                            .size(16.0));
                        ui.label(egui::RichText::new(disclaimer2)
                            .color(egui::Color32::from_rgb(255, 150, 100))
                            .size(16.0));
                    });
                });
                ui.add_space(side_margin);
            });
            
            ui.add_space(30.0);
            
            // 输入框和按钮居中
            ui.horizontal(|ui| {
                let input_width = 400.0_f32.min(available_width - 100.0);
                let input_margin = (available_width - input_width - 60.0) / 2.0;
                
                ui.add_space(input_margin);
                let response = ui.add(
                    egui::TextEdit::singleline(&mut self.input)
                        .desired_width(input_width)
                        .hint_text(input_hint)
                        .font(egui::TextStyle::Body)
                );
                
                if ui.button(egui::RichText::new(parse_btn_text).size(16.0)).clicked() 
                    || (response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter))) {
                    if !self.input.is_empty() {
                        parse_requested = Some(self.input.clone());
                        self.input.clear();
                    }
                }
                ui.add_space(input_margin);
            });
            
            ui.add_space(20.0);
            
            // 支持格式卡片居中
            ui.horizontal(|ui| {
                ui.add_space(side_margin);
                ui.group(|ui| {
                    ui.set_width(content_width);
                    ui.vertical_centered(|ui| {
                        ui.label("Supported input formats:");
                        ui.add_space(5.0);
                        ui.label("• BV ID: BV1xx411c7XE");
                        ui.label("• Full URL: https://www.bilibili.com/video/BV1xx411c7XE");
                        ui.label("• Short URL: https://b23.tv/xxxxxx");
                    });
                });
                ui.add_space(side_margin);
            });
        });
        
        parse_requested
    }
    
    pub fn show_with_texts_and_language(
        &mut self,
        ui: &mut egui::Ui,
        parse_video_text: &str,
        disclaimer1: &str,
        disclaimer2: &str,
        input_hint: &str,
        parse_btn_text: &str,
        language: &Language,
    ) -> Option<String> {
        let mut parse_requested = None;
        
        // 获取可用宽度
        let available_width = ui.available_width();
        let content_width = 500.0_f32.min(available_width - 40.0);
        let side_margin = (available_width - content_width) / 2.0;
        
        ui.vertical_centered(|ui| {
            ui.add_space(50.0);
            
            ui.heading(egui::RichText::new(parse_video_text).size(24.0));
            
            ui.add_space(30.0);
            
            // 使用边距来居中内容
            ui.horizontal(|ui| {
                ui.add_space(side_margin);
                ui.group(|ui| {
                    ui.set_width(content_width);
                    ui.vertical_centered(|ui| {
                        ui.label(egui::RichText::new(disclaimer1)
                            .color(egui::Color32::from_rgb(255, 100, 100))
                            .size(16.0));
                        ui.label(egui::RichText::new(disclaimer2)
                            .color(egui::Color32::from_rgb(255, 150, 100))
                            .size(16.0));
                    });
                });
                ui.add_space(side_margin);
            });
            
            ui.add_space(30.0);
            
            // 输入框和按钮居中
            ui.horizontal(|ui| {
                let input_width = 400.0_f32.min(available_width - 100.0);
                let input_margin = (available_width - input_width - 60.0) / 2.0;
                
                ui.add_space(input_margin);
                let response = ui.add(
                    egui::TextEdit::singleline(&mut self.input)
                        .desired_width(input_width)
                        .hint_text(input_hint)
                        .font(egui::TextStyle::Body)
                );
                
                if ui.button(egui::RichText::new(parse_btn_text).size(16.0)).clicked() 
                    || (response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter))) {
                    if !self.input.is_empty() {
                        parse_requested = Some(self.input.clone());
                        self.input.clear();
                    }
                }
                ui.add_space(input_margin);
            });
            
            ui.add_space(20.0);
            
            // 支持格式卡片居中
            ui.horizontal(|ui| {
                ui.add_space(side_margin);
                ui.group(|ui| {
                    ui.set_width(content_width);
                    ui.vertical_centered(|ui| {
                        match language {
                            Language::SimplifiedChinese => {
                                ui.label("支持的输入格式:");
                                ui.add_space(5.0);
                                ui.label("• BV号: BV1xx411c7XE");
                                ui.label("• 完整链接: https://www.bilibili.com/video/BV1xx411c7XE");
                                ui.label("• 短链接: https://b23.tv/xxxxxx");
                            }
                            Language::English => {
                                ui.label("Supported input formats:");
                                ui.add_space(5.0);
                                ui.label("• BV ID: BV1xx411c7XE");
                                ui.label("• Full URL: https://www.bilibili.com/video/BV1xx411c7XE");
                                ui.label("• Short URL: https://b23.tv/xxxxxx");
                            }
                        }
                    });
                });
                ui.add_space(side_margin);
            });
        });
        
        parse_requested
    }
}