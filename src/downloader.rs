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
use serde_json::json;

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
        
        let mut manager = Self {
            tasks: Arc::new(RwLock::new(HashMap::new())),
            download_path,
            runtime: runtime.clone(),
            bilibili_api,
            config,
            aria2_client: Arc::new(RwLock::new(None)),
            aria2_process: None,
        };
        
        // 启动aria2和连接
        manager.start_aria2();
        manager.connect_aria2();
        
        // 启动状态更新任务
        let tasks = manager.tasks.clone();
        let aria2_client = manager.aria2_client.clone();
        runtime.spawn(async move {
            loop {
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                Self::update_all_status(tasks.clone(), aria2_client.clone()).await;
            }
        });
        
        manager
    }
    
    fn start_aria2(&mut self) {
        let aria2_path = Self::get_aria2_path();
        
        if !aria2_path.exists() {
            eprintln!("警告: aria2c 未找到！");
            eprintln!("请下载 aria2c 并放置到: {:?}", aria2_path);
            return;
        }
        
        // 杀死已存在的aria2进程
        #[cfg(target_os = "windows")]
        {
            let _ = Command::new("taskkill")
                .args(&["/F", "/IM", "aria2c.exe"])
                .output();
        }
        
        std::thread::sleep(std::time::Duration::from_millis(500));
        
        // 启动aria2c
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
        
        match cmd.spawn() {
            Ok(child) => {
                println!("aria2c 启动成功，PID: {:?}", child.id());
                self.aria2_process = Some(child);
                std::thread::sleep(std::time::Duration::from_secs(2));
            }
            Err(e) => {
                eprintln!("无法启动 aria2c: {}", e);
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
                        println!("成功连接到 aria2 RPC");
                        *aria2_client.write() = Some(client);
                        break;
                    }
                    Err(e) => {
                        retry_count += 1;
                        eprintln!("连接aria2失败 (尝试 {}/{}): {}", retry_count, max_retries, e);
                        if retry_count >= max_retries {
                            eprintln!("无法连接到 aria2 RPC");
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
        aria2_client: Arc<RwLock<Option<Aria2Client>>>
    ) {
        let client = {
            let guard = aria2_client.read();
            guard.as_ref().cloned()
        };
        
        if let Some(client) = client {
            let task_list: Vec<(String, Arc<RwLock<DownloadTask>>)> = {
                tasks.read().iter().map(|(id, task)| (id.clone(), task.clone())).collect()
            };
            
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
                        
                        // 检查视频下载状态
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
                        
                        // 检查音频下载状态
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
                        
                        // 更新任务状态
                        let status_arc = {
                            let task_read = task.read();
                            task_read.status.clone()
                        };
                        
                        if has_error {
                            *status_arc.write() = DownloadStatus::Failed(error_msg);
                        } else if all_complete && video_progress >= 1.0 && audio_progress >= 1.0 {
                            *status_arc.write() = DownloadStatus::Merging { progress: 0.5 };
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
        
        exe_dir.join("tools").join(aria2_name)
    }
    
    fn get_ffmpeg_path() -> PathBuf {
        let exe_dir = std::env::current_exe()
            .unwrap_or_else(|_| PathBuf::from("."))
            .parent()
            .unwrap_or(&PathBuf::from("."))
            .to_path_buf();
        
        #[cfg(target_os = "windows")]
        let ffmpeg_name = "ffmpeg.exe";
        #[cfg(not(target_os = "windows"))]
        let ffmpeg_name = "ffmpeg";
        
        exe_dir.join("tools").join(ffmpeg_name)
    }
    
    pub fn get_config(&self) -> Arc<RwLock<Config>> {
        self.config.clone()
    }
    
    pub fn add_task(&self, task: DownloadTask) {
        let task_id = task.id.clone();
        let task = Arc::new(RwLock::new(task));
        self.tasks.write().insert(task_id.clone(), task.clone());
        
        let download_path = self.download_path.clone();
        let bilibili_api = self.bilibili_api.clone();
        let aria2_client = self.aria2_client.clone();
        
        self.runtime.spawn(async move {
            // 延迟启动确保aria2准备就绪
            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
            Self::download_task(task, download_path, bilibili_api, aria2_client).await;
        });
    }
    
    async fn download_task(
        task: Arc<RwLock<DownloadTask>>,
        download_path: PathBuf,
        bilibili_api: Arc<BilibiliApi>,
        aria2_client: Arc<RwLock<Option<Aria2Client>>>,
    ) {
        let (id, title, is_mp3, quality, cid) = {
            let t = task.read();
            (t.id.clone(), t.title.clone(), t.is_mp3, t.quality, t.cid)
        };
        
        println!("开始下载任务: BV={}, 标题={}, 质量={}", id, title, quality);
        
        *task.write().status.write() = DownloadStatus::Downloading {
            progress: 0.0,
            speed: "获取下载地址...".to_string(),
        };
        
        // 获取下载URL
        println!("正在获取视频下载地址...");
        match bilibili_api.get_download_urls(&id, cid, quality).await {
            Ok((video_url, audio_url)) => {
                println!("成功获取下载地址");
                
                let safe_title = Self::sanitize_filename(&title);
                let video_file = download_path.join(format!("{}_video.m4s", safe_title));
                let audio_file = download_path.join(format!("{}_audio.m4s", safe_title));
                let output_file = if is_mp3 {
                    download_path.join(format!("{}.mp3", safe_title))
                } else {
                    download_path.join(format!("{}.mp4", safe_title))
                };
                
                // 检查是否有音频
                let has_audio = video_url != audio_url;
                task.write().has_audio = has_audio;
                
                // 等待客户端连接
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
                    // 准备下载选项
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
                    
                    // 添加视频下载任务
                    println!("添加视频下载任务到aria2...");
                    match client.add_uri(vec![video_url.clone()], Some(options.clone()), None, None).await {
                        Ok(gid) => {
                            println!("视频下载任务已添加，GID: {}", gid);
                            task.write().video_gid = Some(gid.clone());
                            task.write().video_path = Some(video_file.clone());
                            
                            // 如果有独立的音频流，添加音频下载
                            if has_audio {
                                let mut audio_options = options.clone();
                                audio_options.out = Some(audio_file.file_name().unwrap().to_string_lossy().to_string());
                                
                                println!("添加音频下载任务到aria2...");
                                match client.add_uri(vec![audio_url.clone()], Some(audio_options), None, None).await {
                                    Ok(audio_gid) => {
                                        println!("音频下载任务已添加，GID: {}", audio_gid);
                                        task.write().audio_gid = Some(audio_gid.clone());
                                        task.write().audio_path = Some(audio_file.clone());
                                    }
                                    Err(e) => {
                                        eprintln!("添加音频下载失败: {}", e);
                                        *task.write().status.write() = DownloadStatus::Failed(format!("添加音频下载失败: {}", e));
                                        return;
                                    }
                                }
                            }
                            
                            // 等待下载完成
                            loop {
                                tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                                
                                let status = task.read().status.read().clone();
                                match status {
                                    DownloadStatus::Merging { .. } => {
                                        println!("下载完成，开始合并文件...");
                                        break;
                                    }
                                    DownloadStatus::Failed(_) => {
                                        return;
                                    }
                                    _ => {}
                                }
                            }
                            
                            // 执行合并
                            let merge_success = if has_audio {
                                Self::merge_audio_video(&video_file, &audio_file, &output_file, is_mp3).await
                            } else {
                                std::fs::rename(&video_file, &output_file).is_ok()
                            };
                            
                            if merge_success {
                                println!("文件处理成功: {:?}", output_file);
                                // 清理临时文件
                                let _ = std::fs::remove_file(&video_file);
                                if has_audio {
                                    let _ = std::fs::remove_file(&audio_file);
                                }
                                
                                task.write().output_path = Some(output_file);
                                *task.write().status.write() = DownloadStatus::Completed;
                            } else {
                                eprintln!("合并文件失败");
                                *task.write().status.write() = DownloadStatus::Failed("合并音视频失败".to_string());
                            }
                        }
                        Err(e) => {
                            eprintln!("添加视频下载任务失败: {}", e);
                            *task.write().status.write() = DownloadStatus::Failed(format!("添加下载失败: {}", e));
                        }
                    }
                } else {
                    eprintln!("aria2客户端未连接");
                    *task.write().status.write() = DownloadStatus::Failed("aria2客户端未连接".to_string());
                }
            }
            Err(e) => {
                eprintln!("获取下载地址失败: {}", e);
                *task.write().status.write() = DownloadStatus::Failed(format!("获取下载地址失败: {}", e));
            }
        }
    }
    
    async fn merge_audio_video(video_path: &PathBuf, audio_path: &PathBuf, output_path: &PathBuf, is_mp3: bool) -> bool {
        let ffmpeg_path = Self::get_ffmpeg_path();
        
        if !ffmpeg_path.exists() {
            eprintln!("ffmpeg未找到: {:?}", ffmpeg_path);
            return false;
        }
        
        println!("使用ffmpeg合并文件...");
        
        let mut cmd = Command::new(ffmpeg_path);
        
        if is_mp3 {
            cmd.arg("-i").arg(audio_path.to_string_lossy().to_string())
                .arg("-vn")
                .arg("-acodec").arg("mp3")
                .arg("-ab").arg("320k");
        } else {
            cmd.arg("-i").arg(video_path.to_string_lossy().to_string())
                .arg("-i").arg(audio_path.to_string_lossy().to_string())
                .arg("-c").arg("copy");
        }
        
        cmd.arg("-y")
            .arg(output_path.to_string_lossy().to_string());
        
        match cmd.output() {
            Ok(output) => {
                if output.status.success() {
                    println!("ffmpeg合并成功");
                    true
                } else {
                    eprintln!("ffmpeg执行失败: {}", String::from_utf8_lossy(&output.stderr));
                    false
                }
            }
            Err(e) => {
                eprintln!("运行ffmpeg失败: {}", e);
                false
            }
        }
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
        self.tasks.read().values().cloned().collect()
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
        }
    }
    
    pub fn resume_task(&self, id: &str) {
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
    }
}

impl Drop for DownloadManager {
    fn drop(&mut self) {
        if let Some(mut child) = self.aria2_process.take() {
            let _ = child.kill();
        }
    }
}