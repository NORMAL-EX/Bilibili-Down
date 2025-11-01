// src\app.rs
use crate::config::{Config, Theme, Language};
use crate::downloader::{DownloadManager, DownloadTask};
use crate::bilibili::{BilibiliApi, VideoInfo, QualityInfo};
use crate::ui::{home::HomePage, download_queue::DownloadQueuePage, settings::SettingsPage, login::LoginWindow, video_detail::VideoDetailWindow};
use eframe::egui;
use std::sync::Arc;
use parking_lot::RwLock;
use std::sync::mpsc;
use clipboard::{ClipboardProvider, ClipboardContext};

#[cfg(target_os = "windows")]
use winreg::enums::*;
#[cfg(target_os = "windows")]
use winreg::RegKey;
#[cfg(target_os = "windows")]
use windows::{
    core::{HSTRING, IInspectable, ComInterface},
    Data::Xml::Dom::XmlDocument,
    UI::Notifications::{ToastNotification, ToastNotificationManager, ToastActivatedEventArgs},
    Foundation::TypedEventHandler,
};

// Windows DWM相关导入
#[cfg(target_os = "windows")]
use winapi::{
    um::dwmapi::DwmSetWindowAttribute,
    shared::windef::HWND,
};

// DWM窗口属性常量
#[cfg(target_os = "windows")]
const DWMWA_USE_IMMERSIVE_DARK_MODE: u32 = 20;

#[cfg(debug_assertions)]
macro_rules! debug_println {
    ($($arg:tt)*) => { println!($($arg)*) }
}
#[cfg(not(debug_assertions))]
macro_rules! debug_println {
    ($($arg:tt)*) => {}
}

#[cfg(debug_assertions)]
macro_rules! debug_eprintln {
    ($($arg:tt)*) => { eprintln!($($arg)*) }
}
#[cfg(not(debug_assertions))]
macro_rules! debug_eprintln {
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
    
    clipboard: ClipboardContext,
    last_clipboard_content: String,
    notification_shown_for: Vec<String>,
    startup_clipboard_content: String,
    app_started_time: std::time::Instant,

    show_parse_dialog: bool,
    parse_dialog_url: Option<String>,
    notification_handler: Option<mpsc::Receiver<String>>,
    
    #[cfg(target_os = "windows")]
    window_hwnd: Option<HWND>,
}

impl BilibiliDownApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let runtime = Arc::new(tokio::runtime::Runtime::new().unwrap());
        let config = Arc::new(RwLock::new(Config::load()));
        
        Self::setup_fonts(&cc.egui_ctx);
        
        // 获取窗口句柄
        #[cfg(target_os = "windows")]
        let window_hwnd = Self::get_window_hwnd_from_cc(cc);
        
        // 应用主题
        #[cfg(target_os = "windows")]
        Self::apply_theme_with_hwnd(&cc.egui_ctx, &config.read().theme, window_hwnd);
        #[cfg(not(target_os = "windows"))]
        Self::apply_theme_static(&cc.egui_ctx, &config.read().theme);
        
        let bilibili_api = Arc::new(BilibiliApi::new(runtime.clone()));
        
        let download_manager = Arc::new(DownloadManager::new(
            config.read().download_path.clone(),
            config.read().download_threads,
            runtime.clone(),
            bilibili_api.clone(),
            config.clone(),
        ));
        
        let default_avatar = Self::create_default_avatar_texture(cc);
        
        let mut clipboard: ClipboardContext = ClipboardProvider::new().unwrap();
        
        let startup_clipboard = clipboard.get_contents().unwrap_or_default();
        
        let (tx, rx) = mpsc::channel();
        
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
            clipboard,
            last_clipboard_content: startup_clipboard.clone(),
            notification_shown_for: Vec::new(),
            startup_clipboard_content: startup_clipboard,
            app_started_time: std::time::Instant::now(),
            show_parse_dialog: false,
            parse_dialog_url: None,
            notification_handler: Some(rx),
            #[cfg(target_os = "windows")]
            window_hwnd,
        };
        
        Self::set_notification_sender(tx);
        
        app.check_login_status(&cc.egui_ctx);
        app
    }
    
    // 从CreationContext获取窗口句柄
    #[cfg(target_os = "windows")]
    fn get_window_hwnd_from_cc(cc: &eframe::CreationContext<'_>) -> Option<HWND> {
        // 尝试通过winit获取窗口句柄
        use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
        
        if let Ok(window_handle) = cc.window_handle() {
            if let RawWindowHandle::Win32(handle) = window_handle.as_raw() {
                let hwnd = handle.hwnd.get() as HWND;
                debug_println!("成功获取窗口句柄: {:?}", hwnd);
                return Some(hwnd);
            }
        }
        
        debug_eprintln!("无法从CreationContext获取窗口句柄");
        None
    }
    
    fn set_notification_sender(sender: mpsc::Sender<String>) {
        use std::sync::Mutex;
        lazy_static::lazy_static! {
            static ref NOTIFICATION_SENDER: Mutex<Option<mpsc::Sender<String>>> = Mutex::new(None);
        }
        *NOTIFICATION_SENDER.lock().unwrap() = Some(sender);
    }
    
    #[allow(dead_code)]
    fn get_notification_sender() -> Option<mpsc::Sender<String>> {
        use std::sync::Mutex;
        lazy_static::lazy_static! {
            static ref NOTIFICATION_SENDER: Mutex<Option<mpsc::Sender<String>>> = Mutex::new(None);
        }
        NOTIFICATION_SENDER.lock().unwrap().clone()
    }
    
    fn setup_fonts(ctx: &egui::Context) {
        let mut fonts = egui::FontDefinitions::default();
        
        // 尝试加载Windows系统微软雅黑字体
        let font_loaded = Self::load_microsoft_yahei_font(&mut fonts);
        
        if font_loaded {
            // 设置微软雅黑为主要字体
            fonts.families
                .entry(egui::FontFamily::Proportional)
                .or_default()
                .insert(0, "microsoft_yahei".to_owned());
            
            fonts.families
                .entry(egui::FontFamily::Monospace)
                .or_default()
                .insert(0, "microsoft_yahei".to_owned());
        }
        
        ctx.set_fonts(fonts);
    }
    
    fn load_microsoft_yahei_font(fonts: &mut egui::FontDefinitions) -> bool {
        #[cfg(target_os = "windows")]
        {
            // 获取Windows字体目录
            let font_paths = Self::get_windows_font_paths();
            
            // 尝试加载微软雅黑字体文件
            for font_path in font_paths {
                if let Ok(font_data) = std::fs::read(&font_path) {
                    // 成功读取字体文件
                    fonts.font_data.insert(
                        "microsoft_yahei".to_owned(),
                        egui::FontData::from_owned(font_data)
                    );
                    return true;
                }
            }
            
            // 如果所有路径都失败，返回false
            false
        }
        
        #[cfg(not(target_os = "windows"))]
        {
            // 非Windows系统，不加载微软雅黑
            false
        }
    }
    
    #[cfg(target_os = "windows")]
    fn get_windows_font_paths() -> Vec<std::path::PathBuf> {
        use std::path::PathBuf;
        
        let mut paths = Vec::new();
        
        // 方法1: 从环境变量获取Windows目录
        if let Ok(windir) = std::env::var("WINDIR") {
            paths.push(PathBuf::from(&windir).join("Fonts").join("msyh.ttc"));
            paths.push(PathBuf::from(&windir).join("Fonts").join("msyh.ttf"));
        }
        
        // 方法2: 从SystemRoot环境变量获取
        if let Ok(systemroot) = std::env::var("SystemRoot") {
            paths.push(PathBuf::from(&systemroot).join("Fonts").join("msyh.ttc"));
            paths.push(PathBuf::from(&systemroot).join("Fonts").join("msyh.ttf"));
        }
        
        // 方法3: 使用默认路径（适用于大多数Windows系统）
        paths.push(PathBuf::from("C:\\Windows\\Fonts\\msyh.ttc"));
        paths.push(PathBuf::from("C:\\Windows\\Fonts\\msyh.ttf"));
        
        // 方法4: 使用注册表获取字体目录
        if let Some(font_path) = Self::get_font_from_registry() {
            paths.push(font_path);
        }
        
        paths
    }
    
    #[cfg(target_os = "windows")]
    fn get_font_from_registry() -> Option<std::path::PathBuf> {
        use std::path::PathBuf;
        
        let hkcu = RegKey::predef(HKEY_LOCAL_MACHINE);
        if let Ok(key) = hkcu.open_subkey("SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion\\Fonts") {
            if let Ok(value) = key.get_value::<String, _>("微软雅黑 & Microsoft YaHei UI (TrueType)") {
                // 如果是相对路径，需要加上Windows\Fonts目录
                if !value.contains(':') && !value.starts_with('\\') {
                    if let Ok(windir) = std::env::var("WINDIR") {
                        return Some(PathBuf::from(windir).join("Fonts").join(value));
                    }
                }
                return Some(PathBuf::from(value));
            }
        }
        None
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
    
    // 设置DWM标题栏颜色
    #[cfg(target_os = "windows")]
    fn set_dwm_titlebar_color(hwnd: HWND, use_dark_mode: bool) {
        unsafe {
            let value = if use_dark_mode { 1i32 } else { 0i32 };
            DwmSetWindowAttribute(
                hwnd,
                DWMWA_USE_IMMERSIVE_DARK_MODE,
                &value as *const _ as *const _,
                std::mem::size_of::<i32>() as u32,
            );
            debug_println!("设置DWM标题栏颜色: 深色模式={}", use_dark_mode);
        }
    }
    
    // Windows版本：静态方法应用主题
    #[cfg(target_os = "windows")]
    fn apply_theme_with_hwnd(ctx: &egui::Context, theme: &Theme, hwnd: Option<HWND>) {
        let (visuals, use_dark_mode) = match theme {
            Theme::System => {
                let is_dark = Self::is_system_dark_mode();
                (
                    if is_dark { egui::Visuals::dark() } else { egui::Visuals::light() },
                    is_dark
                )
            }
            Theme::Light => (egui::Visuals::light(), false),
            Theme::Dark => (egui::Visuals::dark(), true),
        };
        
        ctx.set_visuals(visuals);
        
        // 设置DWM标题栏颜色
        if let Some(hwnd) = hwnd {
            Self::set_dwm_titlebar_color(hwnd, use_dark_mode);
        }
    }
    
    // 非Windows版本：静态方法应用主题
    #[cfg(not(target_os = "windows"))]
    fn apply_theme_static(ctx: &egui::Context, theme: &Theme) {
        let visuals = match theme {
            Theme::System => {
                if Self::is_system_dark_mode() { 
                    egui::Visuals::dark() 
                } else { 
                    egui::Visuals::light() 
                }
            }
            Theme::Light => egui::Visuals::light(),
            Theme::Dark => egui::Visuals::dark(),
        };
        
        ctx.set_visuals(visuals);
    }
    
    // 实例方法：运行时应用主题
    fn apply_theme(&mut self, ctx: &egui::Context, theme: &Theme) {
        let (visuals, use_dark_mode) = match theme {
            Theme::System => {
                let is_dark = Self::is_system_dark_mode();
                (
                    if is_dark { egui::Visuals::dark() } else { egui::Visuals::light() },
                    is_dark
                )
            }
            Theme::Light => (egui::Visuals::light(), false),
            Theme::Dark => (egui::Visuals::dark(), true),
        };
        
        ctx.set_visuals(visuals);
        
        // 设置DWM标题栏颜色
        #[cfg(target_os = "windows")]
        if let Some(hwnd) = self.window_hwnd {
            Self::set_dwm_titlebar_color(hwnd, use_dark_mode);
        }
    }
    
    fn is_system_dark_mode() -> bool {
        #[cfg(target_os = "windows")]
        {
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
                    "input_hint" => "请输入视频BV号、视频链接或短链接".to_string(),
                    "parse" => "解析".to_string(),
                    "download_video" => "下载视频".to_string(),
                    "download_mp3" => "下载MP3".to_string(),
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
                    "parse_notification_title" => "检测到B站视频链接".to_string(),
                    "parse_notification_body" => "是否解析该视频？".to_string(),
                    "parse_confirm_title" => "视频解析确认".to_string(),
                    "parse_confirm_body" => "检测到B站链接，是否开始解析？".to_string(),
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
                    "input_hint" => "Enter BV ID, video link or short link".to_string(),
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
                    "parse_notification_title" => "Bilibili link detected".to_string(),
                    "parse_notification_body" => "Parse this video?".to_string(),
                    "parse_confirm_title" => "Video Parse Confirmation".to_string(),
                    "parse_confirm_body" => "Bilibili link detected, start parsing?".to_string(),
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
    
    fn check_clipboard(&mut self, ctx: &egui::Context) {
        if self.app_started_time.elapsed() < std::time::Duration::from_secs(3) {
            return;
        }
        
        if let Ok(contents) = self.clipboard.get_contents() {
            if contents == self.startup_clipboard_content {
                return;
            }
            
            if contents != self.last_clipboard_content {
                self.last_clipboard_content = contents.clone();
                
                if (contents.contains("bilibili.com/video/") || 
                    contents.contains("b23.tv/") || 
                    (contents.starts_with("BV") && contents.len() >= 10)) &&
                   !self.notification_shown_for.contains(&contents) {
                    
                    self.notification_shown_for.push(contents.clone());
                    
                    if self.notification_shown_for.len() > 10 {
                        self.notification_shown_for.remove(0);
                    }
                    // TODO:fix notification bug
                    // self.show_interactive_notification(contents.clone());
                    
                    ctx.request_repaint();
                }
            }
        }
    }
    
    #[allow(dead_code)]
    fn show_interactive_notification(&self, url: String) {
        #[cfg(target_os = "windows")]
        {
            use windows::core::Result;
            
            let title = self.get_text("parse_notification_title");
            let body = self.get_text("parse_notification_body");
            
            let result: Result<()> = (|| {
                let toast_xml = XmlDocument::new()?;
                
                // 使用简单的launch参数传递URL，避免复杂的XML转义问题
                let xml_content = format!(
                    r#"<toast activationType="foreground" launch="parseurl:{url}">
                        <visual>
                            <binding template="ToastGeneric">
                                <text>{title}</text>
                                <text>{body}</text>
                            </binding>
                        </visual>
                        <actions>
                            <action content="解析视频" arguments="parseurl:{url}" activationType="foreground"/>
                            <action content="忽略" arguments="dismiss" activationType="foreground"/>
                        </actions>
                        <audio src="ms-winsoundevent:Notification.Default" />
                    </toast>"#,
                    url = url.replace("&", "&amp;").replace("<", "&lt;").replace(">", "&gt;").replace("\"", "&quot;"),
                    title = title,
                    body = body
                );
                
                debug_println!("Toast XML: {}", xml_content);
                toast_xml.LoadXml(&HSTRING::from(&xml_content))?;
                
                let toast = ToastNotification::CreateToastNotification(&toast_xml)?;
                
                // 处理Toast激活事件
                let sender = Self::get_notification_sender();
                let url_clone = url.clone();
                toast.Activated(&TypedEventHandler::new(move |_toast, result: &Option<IInspectable>| {
                    debug_println!("Toast activated event triggered!");
                    
                    // 尝试获取参数
                    let mut should_send = false;
                    let mut parsed_url = url_clone.clone();
                    
                    if let Some(inspectable) = result {
                        // 尝试转换为ToastActivatedEventArgs
                        if let Ok(args) = inspectable.cast::<ToastActivatedEventArgs>() {
                            if let Ok(args_str) = args.Arguments() {
                                let args_string = args_str.to_string_lossy();
                                debug_println!("Toast arguments: {}", args_string);
                                
                                // 检查参数
                                if args_string.starts_with("parseurl:") {
                                    parsed_url = args_string[9..].to_string();
                                    should_send = true;
                                    debug_println!("Extracted URL: {}", parsed_url);
                                } else if args_string == "dismiss" {
                                    debug_println!("User dismissed notification");
                                    should_send = false;
                                } else if !args_string.is_empty() {
                                    // 其他参数也尝试发送
                                    should_send = true;
                                }
                            }
                        } else {
                            // 无法转换，但仍然发送URL
                            debug_println!("Could not cast to ToastActivatedEventArgs, sending URL anyway");
                            should_send = true;
                        }
                    } else {
                        // 点击了通知主体
                        debug_println!("Toast body clicked, sending URL");
                        should_send = true;
                    }
                    
                    // 发送URL到主线程
                    if should_send {
                        if let Some(ref sender) = sender {
                            debug_println!("Sending URL to main thread: {}", parsed_url);
                            if let Err(_e) = sender.send(parsed_url) {
                                debug_eprintln!("Failed to send URL: {:?}", _e);
                            }
                        } else {
                            debug_eprintln!("No sender available!");
                        }
                    }
                    
                    Ok(())
                }))?;
                
                // 使用PowerShell的AUMID进行测试，或者使用自定义的app_id
                // 首先尝试使用我们注册的app id
                let app_id = "BilibiliDown.App";
                let notifier_result = ToastNotificationManager::CreateToastNotifierWithId(&HSTRING::from(app_id));
                
                let notifier = match notifier_result {
                    Ok(n) => {
                        debug_println!("Using app id: {}", app_id);
                        n
                    }
                    Err(_) => {
                        // 如果失败，使用PowerShell的AUMID
                        let powershell_id = "{1AC14E77-02E7-4E5D-B744-2EB1AE5198B7}\\WindowsPowerShell\\v1.0\\powershell.exe";
                        debug_println!("Falling back to PowerShell AUMID: {}", powershell_id);
                        ToastNotificationManager::CreateToastNotifierWithId(&HSTRING::from(powershell_id))?
                    }
                };
                
                notifier.Show(&toast)?;
                debug_println!("Toast notification shown successfully");
                
                Ok(())
            })();
            
            if let Err(_e) = result {
                debug_eprintln!("显示通知失败: {:?}", _e);
            }
        }
        
        #[cfg(not(target_os = "windows"))]
        {
            use notify_rust::Notification;
            let _ = Notification::new()
                .appname("Bilibili-Down")
                .summary(&self.get_text("parse_notification_title"))
                .body(&self.get_text("parse_notification_body"))
                .icon("bilibili")
                .timeout(5000)
                .show();
        }
    }
}

impl eframe::App for BilibiliDownApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.check_clipboard(ctx);
        
        // 处理通知点击事件
        if let Some(ref receiver) = self.notification_handler {
            while let Ok(url) = receiver.try_recv() {
                debug_println!("收到通知点击事件，URL: {}", url);
                
                // 清理URL，移除parseurl:前缀
                let clean_url = if url.starts_with("parseurl:") {
                    url[9..].to_string()
                } else {
                    url
                };
                
                debug_println!("清理后的URL: {}", clean_url);
                self.parse_dialog_url = Some(clean_url);
                self.show_parse_dialog = true;
                ctx.request_repaint();
            }
        }
        
        // 显示解析确认对话框
        if self.show_parse_dialog {
            if let Some(url) = &self.parse_dialog_url.clone() {
                let url_clone = url.clone();
                let mut close_dialog = false;
                let mut should_parse = false;
                
                egui::Window::new(self.get_text("parse_confirm_title"))
                    .collapsible(false)
                    .resizable(false)
                    .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                    .show(ctx, |ui| {
                        ui.vertical_centered(|ui| {
                            ui.label(self.get_text("parse_confirm_body"));
                            ui.add_space(10.0);
                            
                            let display_url = if url_clone.len() > 50 {
                                format!("{}...", &url_clone[..50])
                            } else {
                                url_clone.clone()
                            };
                            ui.label(format!("链接: {}", display_url));
                            ui.add_space(20.0);
                            
                            ui.horizontal(|ui| {
                                if ui.button(egui::RichText::new(self.get_text("parse")).size(16.0)).clicked() {
                                    should_parse = true;
                                    close_dialog = true;
                                }
                                
                                if ui.button(egui::RichText::new(self.get_text("cancel")).size(16.0)).clicked() {
                                    close_dialog = true;
                                }
                            });
                        });
                    });
                
                if close_dialog {
                    self.show_parse_dialog = false;
                    let url_to_parse = self.parse_dialog_url.take();
                    
                    if should_parse {
                        if let Some(url) = url_to_parse {
                            debug_println!("开始解析视频: {}", url);
                            self.parse_video(url);
                        }
                    }
                }
            }
        }
        
        // 处理视频解析结果
        if let Some(receiver) = &self.video_info_receiver {
            if let Ok(result) = receiver.try_recv() {
                self.loading = false;
                match result {
                    Ok(video_info) => {
                        debug_println!("视频解析成功: {}", video_info.title);
                        self.video_detail_window = Some(VideoDetailWindow::new(
                            video_info,
                            self.bilibili_api.clone(),
                            self.runtime.clone(),
                            self.config.clone()
                        ));
                        self.show_video_detail = true;
                        self.error_message = None;
                    }
                    Err(err) => {
                        debug_eprintln!("视频解析失败: {}", err);
                        self.error_message = Some(err);
                    }
                }
                self.video_info_receiver = None;
            }
        }
        
        // 处理用户头像加载
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
        
        // 顶部导航栏
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
        
        // 用户菜单
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
        
        // 主面板
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
                        // 修复：在设置改变后重新获取主题
                        if self.settings_page.show_with_text(ui, &settings_text) {
                            // 重新读取新的主题值
                            let new_theme = self.config.read().theme.clone();
                            self.apply_theme(ctx, &new_theme);
                        }
                    }
                }
            }
        });
        
        // 登录窗口
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
        
        // 视频详情窗口
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
                    self.show_video_detail = false;
                }
            } else {
                self.show_video_detail = false;
            }
        }
        
        ctx.request_repaint_after(std::time::Duration::from_millis(500));
    }
}