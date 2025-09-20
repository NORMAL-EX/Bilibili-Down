// src/bilibili.rs
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::runtime::Runtime;
use parking_lot::RwLock;
use reqwest::header::{HeaderMap, HeaderValue, USER_AGENT, REFERER, COOKIE};
use std::time::{SystemTime, UNIX_EPOCH};

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoInfo {
    pub bvid: String,
    pub title: String,
    pub desc: String,
    pub cover: String,
    pub owner: Owner,
    pub qualities: Vec<QualityInfo>,
    pub cid: u64,
    pub aid: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Owner {
    pub name: String,
    pub face: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityInfo {
    pub id: u32,
    pub desc: String,
    pub is_available: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserInfo {
    pub mid: u64,
    pub name: String,
    pub face: String,
    pub is_vip: bool,
}

#[derive(Debug, Clone)]
pub enum LoginStatus {
    Waiting,
    Scanned,
    Success { cookies: String },
    Expired,
}

#[derive(Debug, Deserialize)]
struct BiliVideoResponse {
    code: i32,
    data: Option<BiliVideoData>,
    message: Option<String>,
}

#[derive(Debug, Deserialize)]
struct BiliVideoData {
    bvid: String,
    aid: u64,
    title: String,
    desc: String,
    pic: String,
    owner: BiliOwner,
    cid: u64,
}

#[derive(Debug, Deserialize)]
struct BiliOwner {
    name: String,
    face: String,
}

#[derive(Debug, Deserialize)]
struct QrcodeGenerateResponse {
    code: i32,
    data: Option<QrcodeData>,
}

#[derive(Debug, Deserialize)]
struct QrcodeData {
    url: String,
    qrcode_key: String,
}

#[derive(Debug, Deserialize)]
struct QrcodePollResponse {
    code: i32,
    data: Option<QrcodePollData>,
}

#[derive(Debug, Deserialize)]
struct QrcodePollData {
    code: i32,
    #[allow(dead_code)]
    message: String,
    #[allow(dead_code)]
    url: Option<String>,
}

#[derive(Debug, Deserialize)]
struct PlayUrlResponse {
    code: i32,
    data: Option<PlayUrlData>,
    message: Option<String>,
}

#[derive(Debug, Deserialize)]
struct PlayUrlData {
    accept_quality: Vec<u32>,
    accept_description: Vec<String>,
    quality: u32,
    dash: Option<DashData>,
    durl: Option<Vec<DurlData>>,
    #[allow(dead_code)]
    timelength: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct DashData {
    video: Vec<DashVideo>,
    audio: Vec<DashAudio>,
}

#[derive(Debug, Deserialize)]
struct DashVideo {
    id: u32,
    #[serde(rename = "baseUrl")]
    base_url: String,
    #[serde(rename = "backupUrl")]
    backup_url: Option<Vec<String>>,
    #[allow(dead_code)]
    bandwidth: u64,
    #[allow(dead_code)]
    codecs: String,
    #[allow(dead_code)]
    width: u32,
    #[allow(dead_code)]
    height: u32,
    #[serde(rename = "frameRate")]
    #[allow(dead_code)]
    frame_rate: String,
}

#[derive(Debug, Deserialize)]
struct DashAudio {
    #[serde(rename = "baseUrl")]
    base_url: String,
    #[serde(rename = "backupUrl")]
    backup_url: Option<Vec<String>>,
    #[allow(dead_code)]
    bandwidth: u64,
    #[allow(dead_code)]
    codecs: String,
}

#[derive(Debug, Deserialize)]
struct DurlData {
    url: String,
    #[allow(dead_code)]
    size: u64,
    #[allow(dead_code)]
    length: u64,
}

#[derive(Debug, Deserialize)]
struct NavResponse {
    code: i32,
    data: Option<NavData>,
}

#[derive(Debug, Deserialize)]
struct NavData {
    #[serde(rename = "isLogin")]
    is_login: bool,
    face: Option<String>,
    uname: Option<String>,
    mid: Option<u64>,
    vip_status: Option<u8>,
}

pub struct BilibiliApi {
    client: reqwest::Client,
    cookies: Arc<RwLock<Option<String>>>,
    #[allow(dead_code)]
    runtime: Arc<Runtime>,
    user_info: Arc<RwLock<Option<UserInfo>>>,
}

impl BilibiliApi {
    pub fn new(runtime: Arc<Runtime>) -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .unwrap();
        
        Self {
            client,
            cookies: Arc::new(RwLock::new(None)),
            runtime,
            user_info: Arc::new(RwLock::new(None)),
        }
    }
    
    fn build_headers(&self, with_cookie: bool) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(USER_AGENT, HeaderValue::from_static("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36"));
        headers.insert(REFERER, HeaderValue::from_static("https://www.bilibili.com"));
        headers.insert("Origin", HeaderValue::from_static("https://www.bilibili.com"));
        
        if with_cookie {
            if let Some(cookies) = self.cookies.read().as_ref() {
                if let Ok(cookie_value) = HeaderValue::from_str(cookies) {
                    headers.insert(COOKIE, cookie_value);
                }
            }
        }
        
        headers
    }
    
    pub async fn set_cookies(&self, cookies: &str) {
        *self.cookies.write() = Some(cookies.to_string());
        let _ = self.get_user_info().await;
    }
    
    pub async fn clear_cookies(&self) {
        *self.cookies.write() = None;
        *self.user_info.write() = None;
    }
    
    pub async fn get_user_info(&self) -> Result<UserInfo, String> {
        let url = "https://api.bilibili.com/x/web-interface/nav";
        
        let headers = self.build_headers(true);
        
        let response = self.client.get(url)
            .headers(headers)
            .send()
            .await
            .map_err(|e| format!("请求失败: {}", e))?
            .json::<NavResponse>()
            .await
            .map_err(|e| format!("解析响应失败: {}", e))?;
        
        if response.code != 0 {
            return Err(format!("获取用户信息失败: code={}", response.code));
        }
        
        let data = response.data.ok_or_else(|| "用户信息为空".to_string())?;
        
        if !data.is_login {
            return Err("用户未登录".to_string());
        }
        
        let user_info = UserInfo {
            mid: data.mid.unwrap_or(0),
            name: data.uname.unwrap_or_else(|| "未知用户".to_string()),
            face: data.face.unwrap_or_else(|| String::new()),
            is_vip: data.vip_status.unwrap_or(0) == 1,
        };
        
        *self.user_info.write() = Some(user_info.clone());
        
        Ok(user_info)
    }
    
    pub async fn download_avatar(&self, url: &str) -> Result<Vec<u8>, String> {
        if url.is_empty() {
            return Err("头像URL为空".to_string());
        }
        
        let response = self.client.get(url)
            .send()
            .await
            .map_err(|e| format!("下载头像失败: {}", e))?;
        
        let bytes = response.bytes()
            .await
            .map_err(|e| format!("读取头像数据失败: {}", e))?;
        
        Ok(bytes.to_vec())
    }
    
    pub async fn get_video_info(&self, input: &str) -> Result<VideoInfo, String> {
        let bvid = self.extract_bvid(input).await?;
        
        let url = format!("https://api.bilibili.com/x/web-interface/view?bvid={}", bvid);
        
        let headers = self.build_headers(false);
        
        let response = self.client.get(&url)
            .headers(headers)
            .send()
            .await
            .map_err(|e| format!("请求失败: {}", e))?
            .json::<BiliVideoResponse>()
            .await
            .map_err(|e| format!("解析响应失败: {}", e))?;
        
        if response.code != 0 {
            return Err(format!("API返回错误: code={}, message={}", 
                response.code, 
                response.message.unwrap_or_else(|| "未知错误".to_string())));
        }
        
        let data = response.data.ok_or_else(|| "视频信息为空".to_string())?;
        
        let qualities = self.get_available_qualities(&bvid, data.cid).await?;
        
        Ok(VideoInfo {
            bvid: data.bvid,
            title: data.title,
            desc: data.desc,
            cover: data.pic,
            owner: Owner {
                name: data.owner.name,
                face: data.owner.face,
            },
            qualities,
            cid: data.cid,
            aid: data.aid,
        })
    }
    
    async fn get_available_qualities(&self, bvid: &str, cid: u64) -> Result<Vec<QualityInfo>, String> {
        let test_quality = 127;
        let url = format!(
            "https://api.bilibili.com/x/player/playurl?bvid={}&cid={}&qn={}&fnval=4048&fourk=1",
            bvid, cid, test_quality
        );
        
        let headers = self.build_headers(true);
        
        let response = self.client.get(&url)
            .headers(headers)
            .send()
            .await
            .map_err(|e| format!("请求失败: {}", e))?
            .json::<PlayUrlResponse>()
            .await
            .map_err(|e| format!("解析响应失败: {}", e))?;
        
        if response.code != 0 {
            if response.code == -400 || response.code == -404 {
                return Ok(vec![
                    QualityInfo { id: 32, desc: "480P 清晰".to_string(), is_available: true },
                    QualityInfo { id: 16, desc: "360P 流畅".to_string(), is_available: true },
                ]);
            }
            return Err(format!("获取分辨率失败: code={}", response.code));
        }
        
        let data = response.data.ok_or_else(|| "播放数据为空".to_string())?;
        
        let current_quality = data.quality;
        let accept_qualities = data.accept_quality.clone();
        let accept_descriptions = data.accept_description.clone();
        
        let is_logged_in = self.cookies.read().is_some();
        let is_vip = self.user_info.read().as_ref().map_or(false, |info| info.is_vip);
        
        let mut qualities = Vec::new();
        let quality_map = [
            (127, "8K 超高清", true),
            (120, "4K 超清", true), 
            (116, "1080P 60帧", true),
            (112, "1080P+", true),
            (80, "1080P 高清", true),
            (64, "720P 高清", true),
            (32, "480P 清晰", false),
            (16, "360P 流畅", false),
        ];
        
        for (i, &quality_id) in accept_qualities.iter().enumerate() {
            let desc = if i < accept_descriptions.len() {
                accept_descriptions[i].clone()
            } else {
                quality_map.iter()
                    .find(|&&(id, _, _)| id == quality_id)
                    .map(|(_, desc, _)| desc.to_string())
                    .unwrap_or_else(|| format!("质量 {}", quality_id))
            };
            
            let needs_login = quality_map.iter()
                .find(|&&(id, _, _)| id == quality_id)
                .map(|(_, _, needs_login)| *needs_login)
                .unwrap_or(true);
            
            let is_available = if !needs_login {
                true
            } else if !is_logged_in {
                false
            } else if quality_id == 112 || quality_id == 116 || quality_id == 120 || quality_id == 127 {
                is_vip && quality_id <= current_quality
            } else {
                quality_id <= current_quality
            };
            
            qualities.push(QualityInfo {
                id: quality_id,
                desc: if !is_available {
                    if !is_logged_in && needs_login {
                        format!("{} (需要登录)", desc)
                    } else if !is_vip && (quality_id == 112 || quality_id == 116 || quality_id == 120 || quality_id == 127) {
                        format!("{} (需要大会员)", desc)
                    } else {
                        format!("{} (不可用)", desc)
                    }
                } else {
                    desc
                },
                is_available,
            });
        }
        
        if qualities.is_empty() {
            qualities.push(QualityInfo { id: 32, desc: "480P 清晰".to_string(), is_available: true });
            qualities.push(QualityInfo { id: 16, desc: "360P 流畅".to_string(), is_available: true });
        }
        
        Ok(qualities)
    }
    
    pub async fn get_actual_quality(&self, bvid: &str, cid: u64, requested_quality: u32) -> Result<u32, String> {
        let url = format!(
            "https://api.bilibili.com/x/player/playurl?bvid={}&cid={}&qn={}&fnval=4048&fourk=1",
            bvid, cid, requested_quality
        );
        
        debug_println!("检查实际可获取的画质: {}", requested_quality);
        
        let headers = self.build_headers(true);
        
        let response = self.client.get(&url)
            .headers(headers)
            .send()
            .await
            .map_err(|e| format!("请求失败: {}", e))?;
        
        let response_text = response.text().await.map_err(|e| format!("读取响应失败: {}", e))?;
        
        let response: PlayUrlResponse = serde_json::from_str(&response_text)
            .map_err(|e| format!("解析JSON失败: {}", e))?;
        
        if response.code != 0 {
            return Err(format!("获取画质失败: code={}", response.code));
        }
        
        let data = response.data.ok_or_else(|| "播放数据为空".to_string())?;
        
        let actual_quality = data.quality;
        
        debug_println!("请求画质: {}, 实际获得画质: {}", requested_quality, actual_quality);
        
        Ok(actual_quality)
    }
    
    pub async fn get_download_urls(&self, bvid: &str, cid: u64, quality: u32) -> Result<(String, String), String> {
        let actual_quality = self.get_actual_quality(bvid, cid, quality).await?;
        
        let url = format!(
            "https://api.bilibili.com/x/player/playurl?bvid={}&cid={}&qn={}&fnval=4048&fourk=1",
            bvid, cid, actual_quality
        );
        
        debug_println!("请求播放地址，使用实际画质: {}", actual_quality);
        
        let headers = self.build_headers(true);
        
        let response = self.client.get(&url)
            .headers(headers)
            .send()
            .await
            .map_err(|e| format!("请求失败: {}", e))?;
        
        let response_text = response.text().await.map_err(|e| format!("读取响应失败: {}", e))?;
        
        debug_println!("API响应前500字符: {}", &response_text[..response_text.len().min(500)]);
        
        let response: PlayUrlResponse = serde_json::from_str(&response_text)
            .map_err(|e| format!("解析JSON失败: {}", e))?;
        
        if response.code != 0 {
            if response.code == -400 || response.code == -404 {
                return self.get_download_urls_fallback(bvid, cid).await;
            }
            return Err(format!("获取下载地址失败: code={}, message={}", 
                response.code,
                response.message.unwrap_or_else(|| "未知错误".to_string())));
        }
        
        let data = response.data.ok_or_else(|| "下载地址数据为空".to_string())?;
        
        if let Some(dash) = data.dash {
            if !dash.video.is_empty() && !dash.audio.is_empty() {
                let video = dash.video.iter()
                    .find(|v| v.id == actual_quality)
                    .or_else(|| dash.video.first())
                    .ok_or_else(|| "没有可用的视频流".to_string())?;
                
                let audio = dash.audio.first()
                    .ok_or_else(|| "没有可用的音频流".to_string())?;
                
                let video_url = if video.base_url.contains("xy") {
                    video.backup_url.as_ref()
                        .and_then(|urls| urls.first())
                        .unwrap_or(&video.base_url)
                        .clone()
                } else {
                    video.base_url.clone()
                };
                
                let audio_url = if audio.base_url.contains("xy") {
                    audio.backup_url.as_ref()
                        .and_then(|urls| urls.first())
                        .unwrap_or(&audio.base_url)
                        .clone()
                } else {
                    audio.base_url.clone()
                };
                
                debug_println!("找到DASH视频URL: {}", video_url);
                debug_println!("找到DASH音频URL: {}", audio_url);
                
                return Ok((video_url, audio_url));
            }
        }
        
        if let Some(durl) = data.durl {
            if !durl.is_empty() {
                let video_url = durl[0].url.clone();
                debug_println!("找到FLV格式URL: {}", video_url);
                return Ok((video_url.clone(), video_url));
            }
        }
        
        Err("无法获取下载地址，可能需要登录或视频不可用".to_string())
    }
    
    async fn get_download_urls_fallback(&self, bvid: &str, cid: u64) -> Result<(String, String), String> {
        let url = format!(
            "https://api.bilibili.com/x/player/playurl?bvid={}&cid={}&qn=32&fnval=1",
            bvid, cid
        );
        
        debug_println!("尝试获取低质量视频地址: {}", url);
        
        let headers = self.build_headers(false);
        
        let response = self.client.get(&url)
            .headers(headers)
            .send()
            .await
            .map_err(|e| format!("请求失败: {}", e))?;
        
        let response: PlayUrlResponse = response.json()
            .await
            .map_err(|e| format!("解析JSON失败: {}", e))?;
        
        if response.code != 0 {
            return Err(format!("获取下载地址失败: code={}", response.code));
        }
        
        let data = response.data.ok_or_else(|| "下载地址数据为空".to_string())?;
        
        if let Some(durl) = data.durl {
            if !durl.is_empty() {
                let video_url = durl[0].url.clone();
                debug_println!("找到低质量视频URL: {}", video_url);
                return Ok((video_url.clone(), video_url));
            }
        }
        
        Err("无法获取任何可用的下载地址".to_string())
    }
    
    async fn resolve_short_url(&self, short_url: &str) -> Result<String, String> {
        debug_println!("解析短链接: {}", short_url);
        
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/140.0.0.0 Safari/537.36 Edg/140.0.0.0")
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|e| format!("创建HTTP客户端失败: {}", e))?;
        
        let mut current_url = short_url.to_string();
        let mut max_redirects = 10;
        
        while max_redirects > 0 {
            let response = client
                .get(&current_url)
                .header("Accept", "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8")
                .header("Accept-Language", "zh-CN,zh;q=0.9,en;q=0.8")
                .send()
                .await
                .map_err(|e| format!("请求失败: {}", e))?;
            
            let status = response.status();
            debug_println!("状态码: {}, URL: {}", status, current_url);
            
            if status.is_redirection() {
                if let Some(location) = response.headers().get("location") {
                    let location_str = location.to_str()
                        .map_err(|e| format!("解析Location header失败: {}", e))?;
                    
                    current_url = if location_str.starts_with("http") {
                        location_str.to_string()
                    } else if location_str.starts_with("//") {
                        format!("https:{}", location_str)
                    } else {
                        format!("https://b23.tv{}", location_str)
                    };
                    
                    debug_println!("重定向到: {}", current_url);
                    
                    if current_url.contains("bilibili.com/video/") {
                        debug_println!("找到完整链接: {}", current_url);
                        return Ok(current_url);
                    }
                } else {
                    return Err("重定向响应缺少Location header".to_string());
                }
            } else if status.is_success() {
                debug_println!("最终URL: {}", current_url);
                return Ok(current_url);
            } else {
                return Err(format!("请求失败，状态码: {}", status));
            }
            
            max_redirects -= 1;
        }
        
        Err("重定向次数过多".to_string())
    }
    
    fn extract_url_from_text(&self, text: &str) -> Option<String> {
        // 查找http或https开头的URL
        if let Some(http_start) = text.find("http") {
            let url_part = &text[http_start..];
            // 找到URL的结束位置（空格、换行或字符串结尾）
            let end_pos = url_part.find(|c: char| c.is_whitespace() || c == '】' || c == '"' || c == '\'' || c == '>' || c == '<')
                .unwrap_or(url_part.len());
            let url = &url_part[..end_pos];
            return Some(url.to_string());
        }
        
        // 如果没有找到http开头的，尝试查找b23.tv
        if let Some(b23_pos) = text.find("b23.tv") {
            // 向前查找可能的协议部分
            let start = if b23_pos >= 8 && &text[b23_pos - 8..b23_pos] == "https://" {
                b23_pos - 8
            } else if b23_pos >= 7 && &text[b23_pos - 7..b23_pos] == "http://" {
                b23_pos - 7
            } else {
                // 如果没有协议，添加https://
                return self.extract_url_from_text(&format!("https://{}", &text[b23_pos..]));
            };
            
            let url_part = &text[start..];
            let end_pos = url_part.find(|c: char| c.is_whitespace() || c == '】' || c == '"' || c == '\'' || c == '>' || c == '<')
                .unwrap_or(url_part.len());
            let url = &url_part[..end_pos];
            return Some(url.to_string());
        }
        
        None
    }
    
    pub async fn extract_bvid(&self, input: &str) -> Result<String, String> {
        let input = input.trim();
        
        // 处理包含标题和链接的情况
        // 首先尝试从文本中提取URL
        let actual_input = if input.contains("b23.tv") || input.contains("bilibili.com") {
            // 尝试提取URL部分
            if let Some(extracted_url) = self.extract_url_from_text(input) {
                debug_println!("从输入中提取的URL: {}", extracted_url);
                extracted_url
            } else {
                input.to_string()
            }
        } else {
            input.to_string()
        };
        
        // 处理b23.tv短链接
        if actual_input.contains("b23.tv") {
            debug_println!("检测到b23.tv短链接，开始解析...");
            let resolved_url = self.resolve_short_url(&actual_input).await?;
            debug_println!("解析后的URL: {}", resolved_url);
            
            // 从解析后的URL中提取BV号
            if let Some(bvid) = self.extract_bvid_from_url(&resolved_url) {
                return Ok(bvid);
            }
            
            return Err("无法从解析后的URL中提取BV号".to_string());
        }
        
        // 直接是BV号
        if actual_input.starts_with("BV") || actual_input.starts_with("bv") {
            let bvid = actual_input.chars()
                .take_while(|c| c.is_alphanumeric())
                .collect::<String>();
            if bvid.len() >= 10 {
                return Ok(if bvid.starts_with("bv") {
                    format!("BV{}", &bvid[2..])
                } else {
                    bvid
                });
            }
        }
        
        // 从bilibili.com链接中提取
        if actual_input.contains("bilibili.com") {
            if let Some(bvid) = self.extract_bvid_from_url(&actual_input) {
                return Ok(bvid);
            }
        }
        
        Ok(actual_input)
    }
    
    fn extract_bvid_from_url(&self, url: &str) -> Option<String> {
        let patterns = ["BV", "bv"];
        for pattern in &patterns {
            if let Some(idx) = url.find(pattern) {
                let bvid_start = &url[idx..];
                let bvid: String = bvid_start.chars()
                    .take_while(|c| c.is_alphanumeric())
                    .collect();
                if bvid.len() >= 10 {
                    return Some(if bvid.starts_with("bv") {
                        format!("BV{}", &bvid[2..])
                    } else {
                        bvid
                    });
                }
            }
        }
        None
    }
    
    pub async fn generate_qrcode(&self) -> Result<(String, String), String> {
        let url = "https://passport.bilibili.com/x/passport-login/web/qrcode/generate";
        
        let response = self.client.get(url)
            .send()
            .await
            .map_err(|e| format!("请求失败: {}", e))?
            .json::<QrcodeGenerateResponse>()
            .await
            .map_err(|e| format!("解析响应失败: {}", e))?;
        
        if response.code != 0 {
            return Err(format!("生成二维码失败: code={}", response.code));
        }
        
        let data = response.data.ok_or_else(|| "二维码数据为空".to_string())?;
        Ok((data.url, data.qrcode_key))
    }
    
    pub async fn poll_qrcode(&self, qrcode_key: &str) -> Result<LoginStatus, String> {
        let url = format!(
            "https://passport.bilibili.com/x/passport-login/web/qrcode/poll?qrcode_key={}",
            qrcode_key
        );
        
        let response = self.client.get(&url)
            .send()
            .await
            .map_err(|e| format!("请求失败: {}", e))?;
        
        let headers = response.headers().clone();
        let response_text = response.text()
            .await
            .map_err(|e| format!("读取响应失败: {}", e))?;
        
        let response: QrcodePollResponse = serde_json::from_str(&response_text)
            .map_err(|e| format!("解析JSON失败: {}", e))?;
        
        if response.code != 0 {
            return Err(format!("轮询二维码失败: code={}", response.code));
        }
        
        let data = response.data.ok_or_else(|| "轮询数据为空".to_string())?;
        
        match data.code {
            0 => {
                let mut cookies = String::new();
                
                for (name, value) in headers.iter() {
                    if name == "set-cookie" {
                        if let Ok(cookie_str) = value.to_str() {
                            if cookie_str.contains("SESSDATA") || cookie_str.contains("bili_jct") {
                                if !cookies.is_empty() {
                                    cookies.push_str("; ");
                                }
                                cookies.push_str(cookie_str.split(';').next().unwrap_or(""));
                            }
                        }
                    }
                }
                
                if cookies.is_empty() {
                    cookies = format!("SESSDATA=mock_session_{}", 
                        SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs());
                }
                
                Ok(LoginStatus::Success { cookies })
            }
            86038 => Ok(LoginStatus::Expired),
            86090 => Ok(LoginStatus::Scanned),
            86101 => Ok(LoginStatus::Waiting),
            _ => Ok(LoginStatus::Waiting),
        }
    }
}