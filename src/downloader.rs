use std::path::PathBuf;
use std::sync::Arc;
use parking_lot::RwLock;
use std::collections::HashMap;
use tokio::runtime::Runtime;
use crate::bilibili::BilibiliApi;
use crate::config::Config;
use std::process::Command;
use aria2_ws::{Client as Aria2Client, TaskOptions};
use aria2_ws::response::TaskStatus;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::time::{SystemTime, UNIX_EPOCH};

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x08000000;

#[cfg(target_os = "windows")]
use winapi::{
    um::{
        tlhelp32::{CreateToolhelp32Snapshot, Process32First, Process32Next, PROCESSENTRY32, TH32CS_SNAPPROCESS},
        processthreadsapi::{OpenProcess, TerminateProcess},
        handleapi::CloseHandle,
        winnt::PROCESS_TERMINATE,
    },
    shared::minwindef::DWORD,
};

#[cfg(target_os = "windows")]
use std::os::windows::ffi::OsStringExt;

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
pub enum DownloadStatus {
    Waiting,
    Downloading { progress: f32, speed: String },
    Paused,
    Merging { progress: f32 },
    Completed,
    Failed(String),
}

#[derive(Debug, Clone)]
pub struct DownloadTask {
    pub id: String,
    pub title: String,
    pub author: String,
    pub cover: String,
    pub quality: u32,
    pub is_mp3: bool,
    pub status: Arc<RwLock<DownloadStatus>>,
    pub video_path: Option<PathBuf>,
    pub audio_path: Option<PathBuf>,
    pub output_path: Option<PathBuf>,
    pub cid: u64,
    pub video_gid: Option<String>,
    pub audio_gid: Option<String>,
    pub has_audio: bool,
    pub created_at: i64,
}

impl DownloadTask {
    pub fn new(id: String, title: String, author: String, cover: String, quality: u32, is_mp3: bool, cid: u64) -> Self {
        Self {
            id,
            title,
            author,
            cover,
            quality,
            is_mp3,
            status: Arc::new(RwLock::new(DownloadStatus::Waiting)),
            video_path: None,
            audio_path: None,
            output_path: None,
            cid,
            video_gid: None,
            audio_gid: None,
            has_audio: false,
            created_at: Self::now_millis(),
        }
    }

    fn now_millis() -> i64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_millis().min(i64::MAX as u128) as i64)
            .unwrap_or(0)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "state", content = "data")]
enum PersistedDownloadStatus {
    Waiting,
    Paused,
    Completed,
    Failed(String),
}

impl From<&DownloadStatus> for PersistedDownloadStatus {
    fn from(status: &DownloadStatus) -> Self {
        match status {
            DownloadStatus::Waiting => Self::Waiting,
            DownloadStatus::Downloading { .. } | DownloadStatus::Merging { .. } => Self::Paused,
            DownloadStatus::Paused => Self::Paused,
            DownloadStatus::Completed => Self::Completed,
            DownloadStatus::Failed(err) => Self::Failed(err.clone()),
        }
    }
}

impl From<PersistedDownloadStatus> for DownloadStatus {
    fn from(status: PersistedDownloadStatus) -> Self {
        match status {
            PersistedDownloadStatus::Waiting => DownloadStatus::Paused,
            PersistedDownloadStatus::Paused => DownloadStatus::Paused,
            PersistedDownloadStatus::Completed => DownloadStatus::Completed,
            PersistedDownloadStatus::Failed(err) => DownloadStatus::Failed(err),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PersistedDownloadTask {
    id: String,
    title: String,
    author: String,
    cover: String,
    quality: u32,
    is_mp3: bool,
    status: PersistedDownloadStatus,
    video_path: Option<PathBuf>,
    audio_path: Option<PathBuf>,
    output_path: Option<PathBuf>,
    cid: u64,
    has_audio: bool,
    created_at: i64,
}

impl From<&DownloadTask> for PersistedDownloadTask {
    fn from(task: &DownloadTask) -> Self {
        Self {
            id: task.id.clone(),
            title: task.title.clone(),
            author: task.author.clone(),
            cover: task.cover.clone(),
            quality: task.quality,
            is_mp3: task.is_mp3,
            status: PersistedDownloadStatus::from(&*task.status.read()),
            video_path: task.video_path.clone(),
            audio_path: task.audio_path.clone(),
            output_path: task.output_path.clone(),
            cid: task.cid,
            has_audio: task.has_audio,
            created_at: task.created_at,
        }
    }
}

impl From<PersistedDownloadTask> for DownloadTask {
    fn from(task: PersistedDownloadTask) -> Self {
        Self {
            id: task.id,
            title: task.title,
            author: task.author,
            cover: task.cover,
            quality: task.quality,
            is_mp3: task.is_mp3,
            status: Arc::new(RwLock::new(DownloadStatus::from(task.status))),
            video_path: task.video_path,
            audio_path: task.audio_path,
            output_path: task.output_path,
            cid: task.cid,
            video_gid: None,
            audio_gid: None,
            has_audio: task.has_audio,
            created_at: task.created_at,
        }
    }
}

pub struct DownloadManager {
    tasks: Arc<RwLock<HashMap<String, Arc<RwLock<DownloadTask>>>>>,
    download_path: PathBuf,
    runtime: Arc<Runtime>,
    bilibili_api: Arc<BilibiliApi>,
    config: Arc<RwLock<Config>>,
    aria2_client: Arc<RwLock<Option<Aria2Client>>>,
    aria2_process: Option<std::process::Child>,
    task_store_path: PathBuf,
}

impl DownloadManager {
    pub fn new(
        download_path: PathBuf,
        _threads: u32,
        runtime: Arc<Runtime>,
        bilibili_api: Arc<BilibiliApi>,
        config: Arc<RwLock<Config>>
    ) -> Self {
        if !download_path.exists() {
            let _ = std::fs::create_dir_all(&download_path);
        }

        let task_store_path = Self::task_store_path();
        
        let mut manager = Self {
            tasks: Arc::new(RwLock::new(HashMap::new())),
            download_path,
            runtime: runtime.clone(),
            bilibili_api,
            config,
            aria2_client: Arc::new(RwLock::new(None)),
            aria2_process: None,
            task_store_path,
        };

        manager.load_tasks();
        
        manager.start_aria2();
        manager.connect_aria2();
        
        let tasks = manager.tasks.clone();
        let aria2_client = manager.aria2_client.clone();
        let task_store_path = manager.task_store_path.clone();
        runtime.spawn(async move {
            loop {
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                Self::update_all_status(tasks.clone(), aria2_client.clone(), task_store_path.clone()).await;
            }
        });
        
        manager
    }

    fn task_store_path() -> PathBuf {
        if let Some(config_dir) = dirs::config_dir() {
            let app_config = config_dir.join("bilibili-down");
            let _ = std::fs::create_dir_all(&app_config);
            return app_config.join("tasks.json");
        }

        std::env::current_exe()
            .unwrap_or_else(|_| PathBuf::from("."))
            .parent()
            .unwrap_or(&PathBuf::from("."))
            .join("tasks.json")
    }

    fn load_tasks(&mut self) {
        if !self.task_store_path.exists() {
            return;
        }

        let Ok(content) = std::fs::read_to_string(&self.task_store_path) else {
            return;
        };
        let Ok(saved_tasks) = serde_json::from_str::<Vec<PersistedDownloadTask>>(&content) else {
            return;
        };

        let mut tasks = self.tasks.write();
        for saved_task in saved_tasks {
            let task: DownloadTask = saved_task.into();
            tasks.insert(task.id.clone(), Arc::new(RwLock::new(task)));
        }
    }

    fn save_tasks(&self) {
        Self::save_tasks_to_path(&self.tasks, &self.task_store_path);
    }

    fn save_tasks_to_path(
        tasks: &Arc<RwLock<HashMap<String, Arc<RwLock<DownloadTask>>>>>,
        task_store_path: &PathBuf,
    ) {
        let mut snapshot: Vec<PersistedDownloadTask> = tasks
            .read()
            .values()
            .map(|task| {
                let task = task.read();
                PersistedDownloadTask::from(&*task)
            })
            .collect();

        snapshot.sort_by_key(|task| task.created_at);

        if let Some(parent) = task_store_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }

        if let Ok(content) = serde_json::to_string_pretty(&snapshot) {
            let _ = std::fs::write(task_store_path, content);
        }
    }
    
    #[cfg(target_os = "windows")]
    fn kill_processes_by_name(process_name: &str) {
        unsafe {
            let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
            if snapshot == winapi::um::handleapi::INVALID_HANDLE_VALUE {
                return;
            }
            
            let mut process_entry: PROCESSENTRY32 = std::mem::zeroed();
            process_entry.dwSize = std::mem::size_of::<PROCESSENTRY32>() as DWORD;
            
            if Process32First(snapshot, &mut process_entry) != 0 {
                loop {
                    let exe_name_wide: Vec<u16> = process_entry.szExeFile
                        .iter()
                        .take_while(|&&c| c != 0)
                        .map(|&c| c as u8 as u16)
                        .collect();
                    
                    let exe_name = std::ffi::OsString::from_wide(&exe_name_wide);
                    
                    if let Some(exe_str) = exe_name.to_str() {
                        if exe_str.eq_ignore_ascii_case(process_name) {
                            let process_handle = OpenProcess(
                                PROCESS_TERMINATE,
                                0,
                                process_entry.th32ProcessID
                            );
                            
                            if !process_handle.is_null() {
                                TerminateProcess(process_handle, 101);
                                CloseHandle(process_handle);
                                debug_println!("终止进程: {} (PID: {})", exe_str, process_entry.th32ProcessID);
                            }
                        }
                    }
                    
                    if Process32Next(snapshot, &mut process_entry) == 0 {
                        break;
                    }
                }
            }
            
            CloseHandle(snapshot);
        }
    }
    
    #[cfg(not(target_os = "windows"))]
    fn kill_processes_by_name(process_name: &str) {
        // 使用 pkill 来终止进程
        let _ = Command::new("pkill")
            .arg("-f")
            .arg(process_name)
            .output();
    }
    
    fn start_aria2(&mut self) {
        let aria2_path = Self::get_aria2_path();
        
        if !aria2_path.exists() {
            debug_eprintln!("警告: aria2c 未找到！");
            debug_eprintln!("请下载 aria2c 并放置到: {:?}", aria2_path);
            return;
        }
        
        #[cfg(target_os = "windows")]
        Self::kill_processes_by_name("aria2c.exe");
        #[cfg(not(target_os = "windows"))]
        Self::kill_processes_by_name("aria2c");
        
        std::thread::sleep(std::time::Duration::from_millis(500));
        
        let mut cmd = Command::new(&aria2_path);
        cmd.arg("--enable-rpc")
            .arg("--rpc-listen-all=false")
            .arg("--rpc-listen-port=6800")
            .arg("--rpc-allow-origin-all")
            .arg("--continue=true")
            .arg("--max-connection-per-server=16")
            .arg("--split=16")
            .arg("--min-split-size=1M")
            .arg("--piece-length=1M")
            .arg("--allow-piece-length-change=true")
            .arg("--dir")
            .arg(&self.download_path)
            .arg("--check-certificate=false")
            .arg("--user-agent=Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36");
        
        #[cfg(target_os = "windows")]
        {
            cmd.creation_flags(CREATE_NO_WINDOW);
        }
        
        match cmd.spawn() {
            Ok(child) => {
                debug_println!("aria2c 启动成功，PID: {:?}", child.id());
                self.aria2_process = Some(child);
                std::thread::sleep(std::time::Duration::from_secs(2));
            }
            Err(_e) => {
                debug_eprintln!("无法启动 aria2c: {}", _e);
            }
        }
    }
    
    fn connect_aria2(&self) {
        let aria2_client = self.aria2_client.clone();
        let runtime = self.runtime.clone();
        
        runtime.spawn(async move {
            let mut retry_count = 0;
            let max_retries = 10;
            
            loop {
                match Aria2Client::connect("ws://localhost:6800/jsonrpc", None).await {
                    Ok(client) => {
                        debug_println!("成功连接到 aria2 RPC");
                        *aria2_client.write() = Some(client);
                        break;
                    }
                    Err(_e) => {
                        retry_count += 1;
                        debug_eprintln!("连接aria2失败 (尝试 {}/{}): {}", retry_count, max_retries, _e);
                        if retry_count >= max_retries {
                            debug_eprintln!("无法连接到 aria2 RPC");
                            break;
                        }
                        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                    }
                }
            }
        });
    }
    
    async fn update_all_status(
        tasks: Arc<RwLock<HashMap<String, Arc<RwLock<DownloadTask>>>>>,
        aria2_client: Arc<RwLock<Option<Aria2Client>>>,
        task_store_path: PathBuf,
    ) {
        let client = {
            let guard = aria2_client.read();
            guard.as_ref().cloned()
        };
        
        if let Some(client) = client {
            let task_list: Vec<(String, Arc<RwLock<DownloadTask>>)> = {
                tasks.read().iter().map(|(id, task)| (id.clone(), task.clone())).collect()
            };
            let mut changed = false;
            
            for (_id, task) in task_list.iter() {
                let (video_gid, audio_gid, has_audio) = {
                    let task_read = task.read();
                    (
                        task_read.video_gid.clone(),
                        task_read.audio_gid.clone(),
                        task_read.has_audio,
                    )
                };
                
                let current_status = {
                    let task_read = task.read();
                    let status = task_read.status.read().clone();
                    status
                };
                
                match current_status {
                    DownloadStatus::Downloading { .. } => {
                        let mut video_progress = 0.0;
                        let mut audio_progress = 0.0;
                        let mut total_speed = 0u64;
                        let mut all_complete = true;
                        let mut has_error = false;
                        let mut error_msg = String::new();
                        
                        if let Some(gid) = video_gid {
                            match client.tell_status(gid).await {
                                Ok(status) => {
                                    let total = status.total_length;
                                    let completed = status.completed_length;
                                    
                                    if total > 0 {
                                        video_progress = completed as f32 / total as f32;
                                    }
                                    
                                    total_speed += status.download_speed;
                                    
                                    match status.status {
                                        TaskStatus::Complete => {
                                            video_progress = 1.0;
                                        }
                                        TaskStatus::Active | TaskStatus::Waiting => {
                                            all_complete = false;
                                        }
                                        TaskStatus::Error => {
                                            has_error = true;
                                            error_msg = status.error_message.unwrap_or_else(|| "视频下载失败".to_string());
                                        }
                                        _ => {
                                            all_complete = false;
                                        }
                                    }
                                }
                                Err(_) => {
                                    all_complete = false;
                                }
                            }
                        }
                        
                        if has_audio {
                            if let Some(gid) = audio_gid {
                                match client.tell_status(gid).await {
                                    Ok(status) => {
                                        let total = status.total_length;
                                        let completed = status.completed_length;
                                        
                                        if total > 0 {
                                            audio_progress = completed as f32 / total as f32;
                                        }
                                        
                                        total_speed += status.download_speed;
                                        
                                        match status.status {
                                            TaskStatus::Complete => {
                                                audio_progress = 1.0;
                                            }
                                            TaskStatus::Active | TaskStatus::Waiting => {
                                                all_complete = false;
                                            }
                                            TaskStatus::Error => {
                                                has_error = true;
                                                error_msg = status.error_message.unwrap_or_else(|| "音频下载失败".to_string());
                                            }
                                            _ => {
                                                all_complete = false;
                                            }
                                        }
                                    }
                                    Err(_) => {
                                        all_complete = false;
                                    }
                                }
                            }
                        } else {
                            audio_progress = 1.0;
                        }
                        
                        let status_arc = {
                            let task_read = task.read();
                            task_read.status.clone()
                        };
                        
                        if has_error {
                            *status_arc.write() = DownloadStatus::Failed(error_msg);
                            changed = true;
                        } else if all_complete && video_progress >= 1.0 && audio_progress >= 1.0 {
                            *status_arc.write() = DownloadStatus::Merging { progress: 0.5 };
                            changed = true;
                        } else {
                            let total_progress = if has_audio {
                                (video_progress + audio_progress) / 2.0
                            } else {
                                video_progress
                            };
                            
                            let speed_str = Self::format_speed(total_speed);
                            *status_arc.write() = DownloadStatus::Downloading {
                                progress: total_progress,
                                speed: speed_str,
                            };
                        }
                    }
                    _ => {}
                }
            }

            if changed {
                Self::save_tasks_to_path(&tasks, &task_store_path);
            }
        }
    }
    
    fn format_speed(bytes_per_sec: u64) -> String {
        if bytes_per_sec < 1024 {
            format!("{} B/s", bytes_per_sec)
        } else if bytes_per_sec < 1024 * 1024 {
            format!("{:.1} KB/s", bytes_per_sec as f64 / 1024.0)
        } else {
            format!("{:.1} MB/s", bytes_per_sec as f64 / (1024.0 * 1024.0))
        }
    }
    
    fn get_aria2_path() -> PathBuf {
        let exe_dir = std::env::current_exe()
            .unwrap_or_else(|_| PathBuf::from("."))
            .parent()
            .unwrap_or(&PathBuf::from("."))
            .to_path_buf();
        
        #[cfg(target_os = "windows")]
        let aria2_name = "aria2c.exe";
        #[cfg(not(target_os = "windows"))]
        let aria2_name = "aria2c";
        
        // 先检查 tools 目录
        let tools_path = exe_dir.join("tools").join(aria2_name);
        if tools_path.exists() {
            return tools_path;
        }
        
        // 再检查可执行文件旁边
        let beside_path = exe_dir.join(aria2_name);
        if beside_path.exists() {
            return beside_path;
        }
        
        // 非 Windows 平台：尝试从 PATH 中查找
        #[cfg(not(target_os = "windows"))]
        {
            if let Ok(output) = Command::new("which").arg("aria2c").output() {
                if output.status.success() {
                    let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
                    if !path.is_empty() {
                        return PathBuf::from(path);
                    }
                }
            }
        }
        
        // 回退到 tools 目录路径
        tools_path
    }
    
    pub fn get_config(&self) -> Arc<RwLock<Config>> {
        self.config.clone()
    }
    
    pub fn add_task(&self, task: DownloadTask) {
        let task_id = task.id.clone();
        let task = Arc::new(RwLock::new(task));
        self.tasks.write().insert(task_id.clone(), task.clone());
        self.save_tasks();
        
        let download_path = self.download_path.clone();
        let bilibili_api = self.bilibili_api.clone();
        let aria2_client = self.aria2_client.clone();
        let tasks = self.tasks.clone();
        let task_store_path = self.task_store_path.clone();
        
        self.runtime.spawn(async move {
            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
            Self::download_task(task, download_path, bilibili_api, aria2_client, tasks, task_store_path).await;
        });
    }
    
    async fn download_task(
        task: Arc<RwLock<DownloadTask>>,
        download_path: PathBuf,
        bilibili_api: Arc<BilibiliApi>,
        aria2_client: Arc<RwLock<Option<Aria2Client>>>,
        tasks: Arc<RwLock<HashMap<String, Arc<RwLock<DownloadTask>>>>>,
        task_store_path: PathBuf,
    ) {
        let (id, title, is_mp3, quality, cid) = {
            let t = task.read();
            (t.id.clone(), t.title.clone(), t.is_mp3, t.quality, t.cid)
        };
        
        debug_println!("开始下载任务: BV={}, 标题={}, 质量={}", id, title, quality);
        
        *task.write().status.write() = DownloadStatus::Downloading {
            progress: 0.0,
            speed: "获取下载地址...".to_string(),
        };
        Self::save_tasks_to_path(&tasks, &task_store_path);
        
        debug_println!("正在获取视频下载地址...");
        match bilibili_api.get_download_urls(&id, cid, quality).await {
            Ok((video_url, audio_url)) => {
                debug_println!("成功获取下载地址");
                
                let safe_title = Self::sanitize_filename(&title);
                let video_file = download_path.join(format!("{}_video.m4s", safe_title));
                let audio_file = download_path.join(format!("{}_audio.m4s", safe_title));
                let output_file = if is_mp3 {
                    download_path.join(format!("{}.mp3", safe_title))
                } else {
                    download_path.join(format!("{}.mp4", safe_title))
                };
                
                let has_audio = video_url != audio_url;
                task.write().has_audio = has_audio;
                Self::save_tasks_to_path(&tasks, &task_store_path);
                
                let mut retry_count = 0;
                while aria2_client.read().is_none() && retry_count < 10 {
                    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                    retry_count += 1;
                }
                
                let client = {
                    let guard = aria2_client.read();
                    guard.as_ref().cloned()
                };
                
                if let Some(client) = client {
                    let options = TaskOptions {
                        dir: Some(download_path.to_string_lossy().to_string()),
                        out: Some(video_file.file_name().unwrap().to_string_lossy().to_string()),
                        header: Some(vec![
                            "Referer: https://www.bilibili.com".to_string(),
                            "User-Agent: Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36".to_string(),
                            "Accept: */*".to_string(),
                            "Accept-Language: zh-CN,zh;q=0.9,en;q=0.8".to_string(),
                            "Origin: https://www.bilibili.com".to_string(),
                        ]),
                        split: Some(16),
                        max_connection_per_server: Some(16),
                        extra_options: json!({
                            "min-split-size": "1M",
                            "piece-length": "1M",
                            "allow-piece-length-change": "true",
                            "check-certificate": "false",
                            "auto-file-renaming": "false",
                            "allow-overwrite": "true",
                        }).as_object().unwrap().clone(),
                        ..Default::default()
                    };
                    
                    debug_println!("添加视频下载任务到aria2...");
                    match client.add_uri(vec![video_url.clone()], Some(options.clone()), None, None).await {
                        Ok(gid) => {
                            debug_println!("视频下载任务已添加，GID: {}", gid);
                            task.write().video_gid = Some(gid.clone());
                            task.write().video_path = Some(video_file.clone());
                            Self::save_tasks_to_path(&tasks, &task_store_path);
                            
                            if has_audio {
                                let mut audio_options = options.clone();
                                audio_options.out = Some(audio_file.file_name().unwrap().to_string_lossy().to_string());
                                
                                debug_println!("添加音频下载任务到aria2...");
                                match client.add_uri(vec![audio_url.clone()], Some(audio_options), None, None).await {
                                    Ok(audio_gid) => {
                                        debug_println!("音频下载任务已添加，GID: {}", audio_gid);
                                        task.write().audio_gid = Some(audio_gid.clone());
                                        task.write().audio_path = Some(audio_file.clone());
                                        Self::save_tasks_to_path(&tasks, &task_store_path);
                                    }
                                    Err(e) => {
                                        debug_eprintln!("添加音频下载失败: {}", e);
                                        *task.write().status.write() = DownloadStatus::Failed(format!("添加音频下载失败: {}", e));
                                        Self::save_tasks_to_path(&tasks, &task_store_path);
                                        return;
                                    }
                                }
                            }
                            
                            loop {
                                tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                                
                                let status = task.read().status.read().clone();
                                match status {
                                    DownloadStatus::Merging { .. } => {
                                        debug_println!("下载完成，开始合并文件...");
                                        break;
                                    }
                                    DownloadStatus::Failed(_) => {
                                        Self::save_tasks_to_path(&tasks, &task_store_path);
                                        return;
                                    }
                                    _ => {}
                                }
                            }
                            
                            let merge_success = if has_audio {
                                Self::merge_audio_video(&video_file, &audio_file, &output_file, is_mp3).await
                            } else {
                                std::fs::rename(&video_file, &output_file).is_ok()
                            };
                            
                            if merge_success {
                                debug_println!("文件处理成功: {:?}", output_file);
                                let _ = std::fs::remove_file(&video_file);
                                if has_audio {
                                    let _ = std::fs::remove_file(&audio_file);
                                }
                                
                                task.write().output_path = Some(output_file);
                                *task.write().status.write() = DownloadStatus::Completed;
                                Self::save_tasks_to_path(&tasks, &task_store_path);
                            } else {
                                debug_eprintln!("合并文件失败");
                                *task.write().status.write() = DownloadStatus::Failed("合并音视频失败".to_string());
                                Self::save_tasks_to_path(&tasks, &task_store_path);
                            }
                        }
                        Err(e) => {
                            debug_eprintln!("添加视频下载任务失败: {}", e);
                            *task.write().status.write() = DownloadStatus::Failed(format!("添加下载失败: {}", e));
                            Self::save_tasks_to_path(&tasks, &task_store_path);
                        }
                    }
                } else {
                    debug_eprintln!("aria2客户端未连接");
                    *task.write().status.write() = DownloadStatus::Failed("aria2客户端未连接".to_string());
                    Self::save_tasks_to_path(&tasks, &task_store_path);
                }
            }
            Err(e) => {
                debug_eprintln!("获取下载地址失败: {}", e);
                *task.write().status.write() = DownloadStatus::Failed(format!("获取下载地址失败: {}", e));
                Self::save_tasks_to_path(&tasks, &task_store_path);
            }
        }
    }
    
    async fn merge_audio_video(video_path: &PathBuf, audio_path: &PathBuf, output_path: &PathBuf, is_mp3: bool) -> bool {
        if is_mp3 {
            debug_println!("使用媒体处理 DLL 转换 MP3...");
            return match crate::media_dll::audio_to_mp3(audio_path, output_path, 320) {
                Ok(()) => {
                    debug_println!("MP3 转换成功");
                    true
                }
                Err(_e) => {
                    debug_eprintln!("MP3 转换失败: {}", _e);
                    false
                }
            };
        }

        debug_println!("使用内嵌 MP4 muxer 合并文件...");
        return match crate::mp4_muxer::mux_m4s_to_mp4(video_path, audio_path, output_path) {
            Ok(()) => {
                debug_println!("MP4 合并成功");
                true
            }
            Err(_e) => {
                debug_eprintln!("MP4 合并失败: {}", _e);
                false
            }
        };

    }
    
    fn sanitize_filename(filename: &str) -> String {
        filename.chars()
            .map(|c| {
                if "\\/:*?\"<>|".contains(c) || c.is_control() {
                    '_'
                } else {
                    c
                }
            })
            .collect::<String>()
            .trim()
            .chars()
            .take(200)
            .collect()
    }
    
    pub fn get_tasks(&self) -> Vec<Arc<RwLock<DownloadTask>>> {
        let mut tasks: Vec<_> = self.tasks.read().values().cloned().collect();
        tasks.sort_by_key(|task| task.read().created_at);
        tasks
    }
    
    pub fn pause_task(&self, id: &str) {
        if let Some(task) = self.tasks.read().get(id) {
            let (video_gid, audio_gid) = {
                let task_read = task.read();
                (task_read.video_gid.clone(), task_read.audio_gid.clone())
            };
            
            let aria2_client = self.aria2_client.clone();
            self.runtime.spawn(async move {
                let client = {
                    let guard = aria2_client.read();
                    guard.as_ref().cloned()
                };
                
                if let Some(client) = client {
                    if let Some(gid) = video_gid {
                        let _ = client.pause(gid).await;
                    }
                    if let Some(gid) = audio_gid {
                        let _ = client.pause(gid).await;
                    }
                }
            });
            
            *task.write().status.write() = DownloadStatus::Paused;
            self.save_tasks();
        }
    }
    
    pub fn resume_task(&self, id: &str) {
        if let Some(task) = self.tasks.read().get(id) {
            let (video_gid, audio_gid, should_restart) = {
                let task_read = task.read();
                let status = task_read.status.read().clone();
                (
                    task_read.video_gid.clone(),
                    task_read.audio_gid.clone(),
                    matches!(status, DownloadStatus::Failed(_)) || (task_read.video_gid.is_none() && task_read.audio_gid.is_none()),
                )
            };

            if should_restart {
                {
                    let mut task_write = task.write();
                    task_write.video_gid = None;
                    task_write.audio_gid = None;
                    task_write.video_path = None;
                    task_write.audio_path = None;
                    *task_write.status.write() = DownloadStatus::Waiting;
                }
                self.save_tasks();

                let task = task.clone();
                let download_path = self.download_path.clone();
                let bilibili_api = self.bilibili_api.clone();
                let aria2_client = self.aria2_client.clone();
                let tasks = self.tasks.clone();
                let task_store_path = self.task_store_path.clone();

                self.runtime.spawn(async move {
                    Self::download_task(task, download_path, bilibili_api, aria2_client, tasks, task_store_path).await;
                });

                return;
            }
            
            let aria2_client = self.aria2_client.clone();
            self.runtime.spawn(async move {
                let client = {
                    let guard = aria2_client.read();
                    guard.as_ref().cloned()
                };
                
                if let Some(client) = client {
                    if let Some(gid) = video_gid {
                        let _ = client.unpause(gid).await;
                    }
                    if let Some(gid) = audio_gid {
                        let _ = client.unpause(gid).await;
                    }
                }
            });
            
            *task.write().status.write() = DownloadStatus::Downloading {
                progress: 0.0,
                speed: "恢复中...".to_string(),
            };
            self.save_tasks();
        }
    }
    
    pub fn cancel_task(&self, id: &str) {
        if let Some(task) = self.tasks.read().get(id) {
            let (video_gid, audio_gid) = {
                let task_read = task.read();
                (task_read.video_gid.clone(), task_read.audio_gid.clone())
            };
            
            let aria2_client = self.aria2_client.clone();
            self.runtime.spawn(async move {
                let client = {
                    let guard = aria2_client.read();
                    guard.as_ref().cloned()
                };
                
                if let Some(client) = client {
                    if let Some(gid) = video_gid {
                        let _ = client.remove(gid).await;
                    }
                    if let Some(gid) = audio_gid {
                        let _ = client.remove(gid).await;
                    }
                }
            });
        }
        
        self.tasks.write().remove(id);
        self.save_tasks();
    }
}

impl Drop for DownloadManager {
    fn drop(&mut self) {
        if let Some(mut child) = self.aria2_process.take() {
            let _ = child.kill();
        }
    }
}
