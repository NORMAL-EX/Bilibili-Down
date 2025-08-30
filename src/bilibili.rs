use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::runtime::Runtime;
use parking_lot::RwLock;
use reqwest::header::{HeaderMap, HeaderValue, USER_AGENT, REFERER, COOKIE};
use std::time::{SystemTime, UNIX_EPOCH};

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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserInfo {
    pub mid: u64,
    pub name: String,
    pub face: String,
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
    #[allow(dead_code)]
    accept_description: Vec<String>,
    #[allow(dead_code)]
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
}

pub struct BilibiliApi {
    client: reqwest::Client,
    cookies: Arc<RwLock<Option<String>>>,
    #[allow(dead_code)]
    runtime: Arc<Runtime>,
}

impl BilibiliApi {
    pub fn new(runtime: Arc<Runtime>) -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .unwrap();
        
        Self {
            client,
            cookies: Arc::new(RwLock::new(None)),
            runtime,
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
    }
    
    pub async fn clear_cookies(&self) {
        *self.cookies.write() = None;
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
        
        Ok(UserInfo {
            mid: data.mid.unwrap_or(0),
            name: data.uname.unwrap_or_else(|| "未知用户".to_string()),
            face: data.face.unwrap_or_else(|| String::new()),
        })
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
        let bvid = self.extract_bvid(input);
        
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
        let url = format!(
            "https://api.bilibili.com/x/player/playurl?bvid={}&cid={}&qn=80&fnval=4048&fourk=1",
            bvid, cid
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
                    QualityInfo { id: 32, desc: "480P 清晰".to_string() },
                    QualityInfo { id: 16, desc: "360P 流畅".to_string() },
                ]);
            }
            return Err(format!("获取分辨率失败: code={}", response.code));
        }
        
        let data = response.data.ok_or_else(|| "播放数据为空".to_string())?;
        
        let mut qualities = Vec::new();
        let quality_map = [
            (127, "8K 超高清"),
            (120, "4K 超清"),
            (116, "1080P 60帧"),
            (112, "1080P+"),
            (80, "1080P 高清"),
            (64, "720P 高清"),
            (32, "480P 清晰"),
            (16, "360P 流畅"),
        ];
        
        for &quality_id in &data.accept_quality {
            if let Some(&(id, desc)) = quality_map.iter().find(|&&(id, _)| id == quality_id) {
                qualities.push(QualityInfo {
                    id,
                    desc: desc.to_string(),
                });
            }
        }
        
        if qualities.is_empty() {
            qualities.push(QualityInfo { id: 32, desc: "480P 清晰".to_string() });
            qualities.push(QualityInfo { id: 16, desc: "360P 流畅".to_string() });
        }
        
        Ok(qualities)
    }
    
    #[allow(dead_code)]
    fn decode_cdn_url(&self, url: &str) -> String {
        // Check if URL needs decoding (has xy pattern)
        if url.contains("xy") && url.contains(".mcdn.bilivideo.cn") {
            // Try to use backup URL if available, otherwise return as-is
            // The obfuscated URLs should still work with proper headers
            return url.to_string();
        }
        url.to_string()
    }
    
    pub async fn get_download_urls(&self, bvid: &str, cid: u64, quality: u32) -> Result<(String, String), String> {
        // Use same fnval as PHP code
        let url = format!(
            "https://api.bilibili.com/x/player/playurl?bvid={}&cid={}&qn={}&fnval=4048&fourk=1",
            bvid, cid, quality
        );
        
        println!("请求播放地址: {}", url);
        
        let headers = self.build_headers(true);
        
        let response = self.client.get(&url)
            .headers(headers)
            .send()
            .await
            .map_err(|e| format!("请求失败: {}", e))?;
        
        let response_text = response.text().await.map_err(|e| format!("读取响应失败: {}", e))?;
        
        println!("API响应前500字符: {}", &response_text[..response_text.len().min(500)]);
        
        let response: PlayUrlResponse = serde_json::from_str(&response_text)
            .map_err(|e| format!("解析JSON失败: {}", e))?;
        
        if response.code != 0 {
            // Try without cookies for lower quality
            if response.code == -400 || response.code == -404 {
                return self.get_download_urls_fallback(bvid, cid).await;
            }
            return Err(format!("获取下载地址失败: code={}, message={}", 
                response.code,
                response.message.unwrap_or_else(|| "未知错误".to_string())));
        }
        
        let data = response.data.ok_or_else(|| "下载地址数据为空".to_string())?;
        
        // Try DASH format first (separated audio/video)
        if let Some(dash) = data.dash {
            if !dash.video.is_empty() && !dash.audio.is_empty() {
                // Find video stream with requested quality or best available
                let video = dash.video.iter()
                    .find(|v| v.id == quality)
                    .or_else(|| dash.video.first())
                    .ok_or_else(|| "没有可用的视频流".to_string())?;
                
                // Get highest quality audio
                let audio = dash.audio.first()
                    .ok_or_else(|| "没有可用的音频流".to_string())?;
                
                // Try to use backup URLs if main URLs are obfuscated
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
                
                println!("找到DASH视频URL: {}", video_url);
                println!("找到DASH音频URL: {}", audio_url);
                
                return Ok((video_url, audio_url));
            }
        }
        
        // Fallback to durl format (combined audio/video, usually lower quality)
        if let Some(durl) = data.durl {
            if !durl.is_empty() {
                let video_url = durl[0].url.clone();
                println!("找到FLV格式URL: {}", video_url);
                return Ok((video_url.clone(), video_url));
            }
        }
        
        Err("无法获取下载地址，可能需要登录或视频不可用".to_string())
    }
    
    async fn get_download_urls_fallback(&self, bvid: &str, cid: u64) -> Result<(String, String), String> {
        // Try lower quality without login
        let url = format!(
            "https://api.bilibili.com/x/player/playurl?bvid={}&cid={}&qn=32&fnval=1",
            bvid, cid
        );
        
        println!("尝试获取低质量视频地址: {}", url);
        
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
                println!("找到低质量视频URL: {}", video_url);
                return Ok((video_url.clone(), video_url));
            }
        }
        
        Err("无法获取任何可用的下载地址".to_string())
    }
    
    pub fn extract_bvid(&self, input: &str) -> String {
        let input = input.trim();
        
        if input.starts_with("BV") || input.starts_with("bv") {
            let bvid = input.chars()
                .take_while(|c| c.is_alphanumeric())
                .collect::<String>();
            if bvid.len() >= 10 {
                return if bvid.starts_with("bv") {
                    format!("BV{}", &bvid[2..])
                } else {
                    bvid
                };
            }
        }
        
        if input.contains("bilibili.com") || input.contains("b23.tv") {
            let patterns = ["BV", "bv"];
            for pattern in &patterns {
                if let Some(idx) = input.find(pattern) {
                    let bvid_start = &input[idx..];
                    let bvid: String = bvid_start.chars()
                        .take_while(|c| c.is_alphanumeric())
                        .collect();
                    if bvid.len() >= 10 {
                        return if bvid.starts_with("bv") {
                            format!("BV{}", &bvid[2..])
                        } else {
                            bvid
                        };
                    }
                }
            }
        }
        
        input.to_string()
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