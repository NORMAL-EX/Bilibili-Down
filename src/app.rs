use crate::config::{Config, Theme, Language};
use crate::downloader::{DownloadManager, DownloadTask};
use crate::bilibili::{BilibiliApi, VideoInfo, QualityInfo};
use crate::ui::{home::HomePage, download_queue::DownloadQueuePage, settings::SettingsPage, login::LoginWindow, video_detail::VideoDetailWindow};
use eframe::egui;
use std::sync::Arc;
use parking_lot::RwLock;
use std::sync::mpsc;

#[cfg(debug_assertions)]
macro_rules! debug_println {
    ($($arg:tt)*) => { println!($($arg)*) }
}
#[cfg(not(debug_assertions))]
macro_rules! debug_println {
    ($($arg:tt)*) => {}
}

#[derive(Debug, Clone, PartialEq)]
pub enum Page {
    Home,
    DownloadQueue,
    Settings,
}

pub struct BilibiliDownApp {
    config: Arc<RwLock<Config>>,
    current_page: Page,
    download_manager: Arc<DownloadManager>,
    bilibili_api: Arc<BilibiliApi>,
    
    home_page: HomePage,
    download_queue_page: DownloadQueuePage,
    settings_page: SettingsPage,
    
    show_login_window: bool,
    login_window: LoginWindow,
    
    show_video_detail: bool,
    video_detail_window: Option<VideoDetailWindow>,
    
    user_avatar: Option<egui::TextureHandle>,
    default_avatar: egui::TextureHandle,
    is_logged_in: bool,
    username: Option<String>,
    
    runtime: Arc<tokio::runtime::Runtime>,
    video_info_receiver: Option<mpsc::Receiver<Result<VideoInfo, String>>>,
    avatar_receiver: Option<mpsc::Receiver<(Vec<u8>, String)>>,
    error_message: Option<String>,
    loading: bool,
    
    show_avatar_menu: bool,
    avatar_menu_id: egui::Id,
    avatar_button_rect: Option<egui::Rect>,
}

impl BilibiliDownApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let runtime = Arc::new(tokio::runtime::Runtime::new().unwrap());
        let config = Arc::new(RwLock::new(Config::load()));
        
        Self::setup_fonts(&cc.egui_ctx);
        Self::apply_theme(&cc.egui_ctx, &config.read().theme);
        
        let bilibili_api = Arc::new(BilibiliApi::new(runtime.clone()));
        
        let download_manager = Arc::new(DownloadManager::new(
            config.read().download_path.clone(),
            config.read().download_threads,
            runtime.clone(),
            bilibili_api.clone(),
            config.clone(),
        ));
        
        let default_avatar = Self::create_default_avatar_texture(cc);
        
        let mut app = Self {
            config: config.clone(),
            current_page: Page::Home,
            download_manager: download_manager.clone(),
            bilibili_api: bilibili_api.clone(),
            home_page: HomePage::new(),
            download_queue_page: DownloadQueuePage::new(download_manager.clone()),
            settings_page: SettingsPage::new(config.clone()),
            show_login_window: false,
            login_window: LoginWindow::new(bilibili_api.clone(), runtime.clone()),
            show_video_detail: false,
            video_detail_window: None,
            user_avatar: None,
            default_avatar,
            is_logged_in: false,
            username: None,
            runtime,
            video_info_receiver: None,
            avatar_receiver: None,
            error_message: None,
            loading: false,
            show_avatar_menu: false,
            avatar_menu_id: egui::Id::new("avatar_context_menu"),
            avatar_button_rect: None,
        };
        
        app.check_login_status(&cc.egui_ctx);
        app
    }
    
    fn setup_fonts(ctx: &egui::Context) {
        let mut fonts = egui::FontDefinitions::default();
        
        let font_data = include_bytes!("../assets/NotoSansSC.ttf");
        fonts.font_data.insert(
            "chinese_font".to_owned(),
            egui::FontData::from_static(font_data),
        );
        
        fonts.families
            .entry(egui::FontFamily::Proportional)
            .or_default()
            .insert(0, "chinese_font".to_owned());
        
        fonts.families
            .entry(egui::FontFamily::Monospace)
            .or_default()
            .push("chinese_font".to_owned());
        
        ctx.set_fonts(fonts);
    }
    
    fn create_default_avatar_texture(cc: &eframe::CreationContext<'_>) -> egui::TextureHandle {
        let avatar_bytes = include_bytes!("../assets/OIP.webp");
        
        let image = image::load_from_memory(avatar_bytes).unwrap_or_else(|_| {
            let size = 64;
            let mut pixels = vec![128u8; size * size * 4];
            for i in (0..pixels.len()).step_by(4) {
                pixels[i + 3] = 255;
            }
            image::DynamicImage::ImageRgba8(
                image::RgbaImage::from_raw(size as u32, size as u32, pixels).unwrap()
            )
        });
        
        let resized = image.resize_exact(64, 64, image::imageops::FilterType::Lanczos3);
        let rgba = resized.to_rgba8();
        let size = [rgba.width() as usize, rgba.height() as usize];
        let pixels = rgba.as_flat_samples();
        
        cc.egui_ctx.load_texture(
            "default_avatar",
            egui::ColorImage::from_rgba_unmultiplied(size, pixels.as_slice()),
            Default::default(),
        )
    }
    
    fn apply_theme(ctx: &egui::Context, theme: &Theme) {
        let visuals = match theme {
            Theme::System => {
                if cfg!(target_os = "windows") {
                    if Self::is_system_dark_mode() {
                        egui::Visuals::dark()
                    } else {
                        egui::Visuals::light()
                    }
                } else {
                    egui::Visuals::dark()
                }
            }
            Theme::Light => egui::Visuals::light(),
            Theme::Dark => egui::Visuals::dark(),
        };
        ctx.set_visuals(visuals);
    }
    
    fn is_system_dark_mode() -> bool {
        #[cfg(target_os = "windows")]
        {
            use winreg::enums::*;
            use winreg::RegKey;
            
            let hkcu = RegKey::predef(HKEY_CURRENT_USER);
            if let Ok(key) = hkcu.open_subkey("Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize") {
                if let Ok(value) = key.get_value::<u32, _>("AppsUseLightTheme") {
                    return value == 0;
                }
            }
        }
        false
    }
    
    fn check_login_status(&mut self, _ctx: &egui::Context) {
        let cookies = {
            let config = self.config.read();
            config.cookies.clone()
        };
        
        if let Some(cookies) = cookies {
            let api = self.bilibili_api.clone();
            let runtime = self.runtime.clone();
            let (tx, rx) = mpsc::channel();
            self.avatar_receiver = Some(rx);
            
            runtime.spawn(async move {
                api.set_cookies(&cookies).await;
                
                if let Ok(user_info) = api.get_user_info().await {
                    if let Ok(avatar_bytes) = api.download_avatar(&user_info.face).await {
                        let _ = tx.send((avatar_bytes, user_info.name));
                    }
                }
            });
            
            self.is_logged_in = true;
        }
    }
    
    #[allow(dead_code)]
    fn load_user_info(&mut self, _ctx: &egui::Context) {
        let api = self.bilibili_api.clone();
        let runtime = self.runtime.clone();
        let (tx, rx) = mpsc::channel();
        self.avatar_receiver = Some(rx);
        
        runtime.spawn(async move {
            if let Ok(user_info) = api.get_user_info().await {
                if let Ok(avatar_bytes) = api.download_avatar(&user_info.face).await {
                    let _ = tx.send((avatar_bytes, user_info.name));
                }
            }
        });
    }
    
    fn get_text(&self, key: &str) -> String {
        match self.config.read().language {
            Language::SimplifiedChinese => {
                match key {
                    "home" => "首页".to_string(),
                    "download_queue" => "下载队列".to_string(),
                    "settings" => "设置".to_string(),
                    "login" => "登录".to_string(),
                    "logout" => "退出登录".to_string(),
                    "relogin" => "重新登录".to_string(),
                    "not_logged_in" => "未登录".to_string(),
                    "logged_in_user" => "已登录用户".to_string(),
                    "parse_video" => "B站视频解析".to_string(),
                    "input_hint" => "请输入视频BV号或视频链接".to_string(),
                    "parse" => "解析".to_string(),
                    "download_video" => "解析下载视频".to_string(),
                    "download_mp3" => "解析下载MP3".to_string(),
                    "cancel" => "取消".to_string(),
                    "pause" => "暂停".to_string(),
                    "resume" => "继续".to_string(),
                    "delete" => "删除".to_string(),
                    "disclaimer1" => "该软件是免费软件，请谨防上当受骗".to_string(),
                    "disclaimer2" => "该软件仅用于学习和研究使用".to_string(),
                    "video_detail" => "视频详情".to_string(),
                    "parsing_video" => "正在解析视频信息...".to_string(),
                    "error" => "错误".to_string(),
                    "need_login" => "需要登录才能下载高质量视频".to_string(),
                    _ => key.to_string(),
                }
            }
            Language::English => {
                match key {
                    "home" => "Home".to_string(),
                    "download_queue" => "Download Queue".to_string(),
                    "settings" => "Settings".to_string(),
                    "login" => "Login".to_string(),
                    "logout" => "Logout".to_string(),
                    "relogin" => "Re-login".to_string(),
                    "not_logged_in" => "Not Logged In".to_string(),
                    "logged_in_user" => "Logged In User".to_string(),
                    "parse_video" => "Bilibili Video Parser".to_string(),
                    "input_hint" => "Enter video BV number or video link".to_string(),
                    "parse" => "Parse".to_string(),
                    "download_video" => "Download Video".to_string(),
                    "download_mp3" => "Download MP3".to_string(),
                    "cancel" => "Cancel".to_string(),
                    "pause" => "Pause".to_string(),
                    "resume" => "Resume".to_string(),
                    "delete" => "Delete".to_string(),
                    "disclaimer1" => "This software is free, beware of scams".to_string(),
                    "disclaimer2" => "This software is for learning and research only".to_string(),
                    "video_detail" => "Video Details".to_string(),
                    "parsing_video" => "Parsing video information...".to_string(),
                    "error" => "Error".to_string(),
                    "need_login" => "Login required for high quality video".to_string(),
                    _ => key.to_string(),
                }
            }
        }
    }
    
    fn parse_video(&mut self, input: String) {
        let api = self.bilibili_api.clone();
        let (tx, rx) = mpsc::channel();
        
        self.video_info_receiver = Some(rx);
        self.error_message = None;
        self.loading = true;
        
        self.runtime.spawn(async move {
            let result = match api.get_video_info(&input).await {
                Ok(info) => Ok(info),
                Err(e) => Err(e.to_string()),
            };
            let _ = tx.send(result);
        });
    }
    
    fn start_download(&mut self, video_info: VideoInfo, quality: QualityInfo, download_type: crate::ui::video_detail::DownloadType) {
        if !self.is_logged_in && quality.id > 32 {
            self.error_message = Some(self.get_text("need_login"));
            self.show_login_window = true;
            return;
        }
        
        let task = DownloadTask::new(
            video_info.bvid.clone(),
            video_info.title.clone(),
            video_info.owner.name.clone(),
            video_info.cover.clone(),
            quality.id,
            download_type == crate::ui::video_detail::DownloadType::Mp3,
            video_info.cid,
        );
        
        self.download_manager.add_task(task);
    }
    
    fn handle_logout(&mut self, ctx: &egui::Context) {
        self.is_logged_in = false;
        self.username = None;
        self.user_avatar = None;
        self.config.write().cookies = None;
        self.config.read().save();
        
        let api = self.bilibili_api.clone();
        self.runtime.spawn(async move {
            api.clear_cookies().await;
        });
        
        ctx.request_repaint();
    }
    
    fn handle_relogin(&mut self) {
        self.handle_logout(&egui::Context::default());
        self.show_login_window = true;
    }
}

impl eframe::App for BilibiliDownApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if let Some(receiver) = &self.video_info_receiver {
            if let Ok(result) = receiver.try_recv() {
                self.loading = false;
                match result {
                    Ok(video_info) => {
                        self.video_detail_window = Some(VideoDetailWindow::new(
                            video_info,
                            self.bilibili_api.clone(),
                            self.runtime.clone()
                        ));
                        self.show_video_detail = true;
                        self.error_message = None;
                    }
                    Err(err) => {
                        self.error_message = Some(err);
                    }
                }
                self.video_info_receiver = None;
            }
        }
        
        if let Some(receiver) = &self.avatar_receiver {
            if let Ok((avatar_bytes, username)) = receiver.try_recv() {
                if let Ok(image) = image::load_from_memory(&avatar_bytes) {
                    let rgba = image.to_rgba8();
                    let size = [rgba.width() as usize, rgba.height() as usize];
                    let pixels = rgba.as_flat_samples();
                    let color_image = egui::ColorImage::from_rgba_unmultiplied(
                        size,
                        pixels.as_slice(),
                    );
                    self.user_avatar = Some(ctx.load_texture(
                        "user_avatar",
                        color_image,
                        Default::default(),
                    ));
                    self.username = Some(username);
                    ctx.request_repaint();
                }
                self.avatar_receiver = None;
            }
        }
        
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                let home_text = self.get_text("home");
                let queue_text = self.get_text("download_queue");
                let settings_text = self.get_text("settings");
                
                ui.selectable_value(&mut self.current_page, Page::Home, home_text);
                ui.selectable_value(&mut self.current_page, Page::DownloadQueue, queue_text);
                ui.selectable_value(&mut self.current_page, Page::Settings, settings_text);
                
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let texture = self.user_avatar.as_ref().unwrap_or(&self.default_avatar);
                    let hover_text = if self.is_logged_in {
                        self.username.as_ref().unwrap_or(&self.get_text("logged_in_user")).clone()
                    } else {
                        self.get_text("not_logged_in")
                    };
                    
                    let response = ui.add(
                        egui::ImageButton::new((texture.id(), egui::Vec2::new(32.0, 32.0)))
                            .rounding(egui::Rounding::same(16.0))
                            .frame(false)
                    ).on_hover_text(hover_text);
                    
                    if response.clicked() {
                        if !self.is_logged_in {
                            self.show_login_window = true;
                        } else {
                            self.show_avatar_menu = !self.show_avatar_menu;
                            if self.show_avatar_menu {
                                self.avatar_button_rect = Some(response.rect);
                            }
                        }
                    }
                });
            });
        });
        
        if self.show_avatar_menu && self.is_logged_in {
            if let Some(button_rect) = self.avatar_button_rect {
                let menu_pos = button_rect.left_bottom() + egui::Vec2::new(0.0, 5.0);
                
                let mut close_menu = false;
                
                egui::Area::new(self.avatar_menu_id)
                    .fixed_pos(menu_pos)
                    .order(egui::Order::Foreground)
                    .show(ctx, |ui| {
                        let menu_response = egui::Frame::popup(ui.style())
                            .show(ui, |ui| {
                                ui.set_min_width(120.0);
                                
                                if let Some(username) = &self.username {
                                    ui.label(egui::RichText::new(username).strong());
                                    ui.separator();
                                }
                                
                                if ui.button(self.get_text("relogin")).clicked() {
                                    self.handle_relogin();
                                    close_menu = true;
                                }
                                
                                if ui.button(self.get_text("logout")).clicked() {
                                    self.handle_logout(ctx);
                                    close_menu = true;
                                }
                            });
                        
                        if ctx.input(|i| i.pointer.any_click()) {
                            let pointer_pos = ctx.input(|i| i.pointer.interact_pos()).unwrap_or_default();
                            
                            if !menu_response.response.rect.contains(pointer_pos) 
                                && !button_rect.contains(pointer_pos) {
                                close_menu = true;
                            }
                        }
                    });
                
                if close_menu {
                    self.show_avatar_menu = false;
                    self.avatar_button_rect = None;
                }
            }
        }
        
        egui::CentralPanel::default().show(ctx, |ui| {
            if let Some(error) = &self.error_message {
                ui.colored_label(egui::Color32::RED, format!("{}: {}", self.get_text("error"), error));
                ui.separator();
            }
            
            if self.loading {
                ui.vertical_centered(|ui| {
                    ui.add_space(100.0);
                    ui.spinner();
                    ui.label(self.get_text("parsing_video"));
                });
            } else {
                match self.current_page {
                    Page::Home => {
                        let parse_text = self.get_text("parse_video");
                        let disclaimer1 = self.get_text("disclaimer1");
                        let disclaimer2 = self.get_text("disclaimer2");
                        let input_hint = self.get_text("input_hint");
                        let parse_btn_text = self.get_text("parse");
                        let language = self.config.read().language.clone();
                        
                        if let Some(input) = self.home_page.show_with_texts_and_language(
                            ui,
                            &parse_text,
                            &disclaimer1,
                            &disclaimer2,
                            &input_hint,
                            &parse_btn_text,
                            &language
                        ) {
                            self.parse_video(input);
                        }
                    }
                    Page::DownloadQueue => {
                        let pause_text = self.get_text("pause");
                        let resume_text = self.get_text("resume");
                        let delete_text = self.get_text("delete");
                        
                        self.download_queue_page.show_with_texts(ui, &pause_text, &resume_text, &delete_text);
                    }
                    Page::Settings => {
                        let settings_text = self.get_text("settings");
                        if self.settings_page.show_with_text(ui, &settings_text) {
                            Self::apply_theme(ctx, &self.config.read().theme);
                        }
                    }
                }
            }
        });
        
        if self.show_login_window {
            let login_title = self.get_text("login");
            let mut login_result = None;
            let mut close_window = false;
            
            egui::Window::new(login_title)
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .default_size([400.0, 500.0])
                .show(ctx, |ui| {
                    if let Some(cookies) = self.login_window.show(ui) {
                        login_result = Some(cookies);
                    }
                    
                    ui.separator();
                    if ui.button(self.get_text("cancel")).clicked() {
                        close_window = true;
                    }
                });
            
            if close_window {
                self.show_login_window = false;
            }
            
            if let Some(cookies) = login_result {
                self.show_login_window = false;
                self.config.write().cookies = Some(cookies);
                self.config.read().save();
                self.check_login_status(ctx);
            }
        }
        
        if self.show_video_detail {
            let download_video_text = self.get_text("download_video");
            let download_mp3_text = self.get_text("download_mp3");
            let cancel_text = self.get_text("cancel");
            let video_detail_title = self.get_text("video_detail");
            
            if let Some(window) = &mut self.video_detail_window {
                let mut close_window = false;
                let mut download_request = None;
                
                egui::Window::new(video_detail_title)
                    .collapsible(false)
                    .resizable(true)
                    .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                    .default_size([700.0, 500.0])
                    .open(&mut self.show_video_detail)
                    .show(ctx, |ui| {
                        if let Some((video_info, quality, download_type)) = 
                            window.show_with_texts(ui, &download_video_text, &download_mp3_text, &cancel_text) {
                            download_request = Some((video_info, quality, download_type));
                            close_window = true;
                        }
                    });
                
                if let Some((video_info, quality, download_type)) = download_request {
                    self.start_download(video_info, quality, download_type);
                    self.current_page = Page::DownloadQueue;
                }
                
                if !self.show_video_detail || close_window {
                    self.video_detail_window = None;
                }
            } else {
                self.show_video_detail = false;
            }
        }
        
        ctx.request_repaint_after(std::time::Duration::from_millis(100));
    }
}