// src/bilibili.rs
use md5;
use parking_lot::RwLock;
use reqwest::header::{HeaderMap, HeaderValue, COOKIE, REFERER, USER_AGENT};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::runtime::Runtime;

#[cfg(debug_assertions)]
macro_rules! debug_println {
    ($($arg:tt)*) => { println!($($arg)*) }
}
#[cfg(not(debug_assertions))]
macro_rules! debug_println {
    ($($arg:tt)*) => {};
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
    wbi_img: Option<WbiImg>,
}

#[derive(Debug, Deserialize)]
struct WbiImg {
    img_url: String,
    sub_url: String,
}

// Wbi 签名
const MIXIN_KEY_ENC_TAB: [usize; 64] = [
    46, 47, 18, 2, 53, 8, 23, 32, 15, 50, 10, 31, 58, 3, 45, 35, 27, 43, 5, 49, 33, 9, 42, 19, 29,
    28, 14, 39, 12, 38, 41, 13, 37, 48, 7, 16, 24, 55, 40, 61, 26, 17, 0, 1, 60, 51, 30, 4, 22, 25,
    54, 21, 56, 59, 6, 63, 57, 62, 11, 36, 20, 34, 44, 52,
];

pub struct BilibiliApi {
    client: reqwest::Client,
    cookies: Arc<RwLock<Option<String>>>,
    #[allow(dead_code)]
    runtime: Arc<Runtime>,
    user_info: Arc<RwLock<Option<UserInfo>>>,
    wbi_keys: Arc<RwLock<Option<(String, String)>>>,
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
            wbi_keys: Arc::new(RwLock::new(None)),
        }
    }

    fn get_mixin_key(orig: &str) -> String {
        let mut s = String::new();
        for &idx in MIXIN_KEY_ENC_TAB.iter() {
            if idx < orig.len() {
                s.push(orig.chars().nth(idx).unwrap());
            }
        }
        s.chars().take(32).collect()
    }

    fn encode_wbi(params: &BTreeMap<String, String>, img_key: &str, sub_key: &str) -> String {
        let mixin_key = Self::get_mixin_key(&format!("{}{}", img_key, sub_key));
        let curr_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let mut query_parts = Vec::new();

        let mut params = params.clone();
        params.insert("wts".to_string(), curr_time.to_string());

        let chr_filter = |c: char| matches!(c, '!' | '\'' | '(' | ')' | '*');

        for (k, v) in params.iter() {
            let filtered_v: String = v.chars().filter(|&c| !chr_filter(c)).collect();

            let encoded_k = urlencoding::encode(k);
            let encoded_v = urlencoding::encode(&filtered_v);
            query_parts.push(format!("{}={}", encoded_k, encoded_v));
        }

        let query = query_parts.join("&");
        let hash_str = format!("{}{}", query, mixin_key);
        let w_rid = format!("{:x}", md5::compute(hash_str));

        format!("{}&w_rid={}", query, w_rid)
    }

    fn extract_wbi_key(url: &str) -> Option<String> {
        url.rsplit('/')
            .next()
            .and_then(|s| s.split('.').next())
            .map(|s| s.to_string())
    }

    async fn update_wbi_keys(&self) -> Result<(String, String), String> {
        if let Some(keys) = self.wbi_keys.read().as_ref() {
            return Ok(keys.clone());
        }

        let url = "https://api.bilibili.com/x/web-interface/nav";
        let headers = self.build_headers(true);
        let response = self
            .client
            .get(url)
            .headers(headers)
            .send()
            .await
            .map_err(|e| format!("Nav请求失败: {}", e))?
            .json::<NavResponse>()
            .await
            .map_err(|e| format!("Nav解析失败: {}", e))?;

        if let Some(data) = response.data {
            if let Some(wbi_img) = data.wbi_img {
                let img_key = Self::extract_wbi_key(&wbi_img.img_url).ok_or("无效的img_key")?;
                let sub_key = Self::extract_wbi_key(&wbi_img.sub_url).ok_or("无效的sub_key")?;

                *self.wbi_keys.write() = Some((img_key.clone(), sub_key.clone()));
                return Ok((img_key, sub_key));
            }
        }

        Err("无法获取Wbi签名密钥".to_string())
    }

    fn build_headers(&self, with_cookie: bool) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(USER_AGENT, HeaderValue::from_static("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36"));
        headers.insert(
            REFERER,
            HeaderValue::from_static("https://www.bilibili.com"),
        );
        headers.insert(
            "Origin",
            HeaderValue::from_static("https://www.bilibili.com"),
        );

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
        *self.wbi_keys.write() = None;
    }

    pub async fn get_user_info(&self) -> Result<UserInfo, String> {
        let url = "https://api.bilibili.com/x/web-interface/nav";

        let headers = self.build_headers(true);

        let response = self
            .client
            .get(url)
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

        if let Some(wbi_img) = &data.wbi_img {
            if let (Some(img), Some(sub)) = (
                Self::extract_wbi_key(&wbi_img.img_url),
                Self::extract_wbi_key(&wbi_img.sub_url),
            ) {
                *self.wbi_keys.write() = Some((img, sub));
            }
        }

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

        let response = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|e| format!("下载头像失败: {}", e))?;

        let bytes = response
            .bytes()
            .await
            .map_err(|e| format!("读取头像数据失败: {}", e))?;

        Ok(bytes.to_vec())
    }

    pub async fn get_video_info(&self, input: &str) -> Result<VideoInfo, String> {
        let bvid = self.extract_bvid(input).await?;

        let url = format!(
            "https://api.bilibili.com/x/web-interface/view?bvid={}",
            bvid
        );

        let headers = self.build_headers(false);

        let response = self
            .client
            .get(&url)
            .headers(headers)
            .send()
            .await
            .map_err(|e| format!("请求失败: {}", e))?
            .json::<BiliVideoResponse>()
            .await
            .map_err(|e| format!("解析响应失败: {}", e))?;

        if response.code != 0 {
            return Err(format!(
                "API返回错误: code={}, message={}",
                response.code,
                response.message.unwrap_or_else(|| "未知错误".to_string())
            ));
        }

        let data = response.data.ok_or_else(|| "视频信息为空".to_string())?;

        // 尝试预加载 Keys
        let _ = self.update_wbi_keys().await;

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

    async fn get_available_qualities(
        &self,
        bvid: &str,
        cid: u64,
    ) -> Result<Vec<QualityInfo>, String> {
        let keys_opt = self.update_wbi_keys().await.ok();

        let (url, signed_query) = if let Some((img_key, sub_key)) = keys_opt {
            let mut params = BTreeMap::new();
            params.insert("bvid".to_string(), bvid.to_string());
            params.insert("cid".to_string(), cid.to_string());
            params.insert("qn".to_string(), "80".to_string());
            params.insert("fnval".to_string(), "4048".to_string());
            params.insert("fourk".to_string(), "1".to_string());
            params.insert("try_look".to_string(), "1".to_string());

            let query = Self::encode_wbi(&params, &img_key, &sub_key);
            (
                format!("https://api.bilibili.com/x/player/wbi/playurl?{}", query),
                None::<egui::Key>,
            )
        } else {
            // Fallback
            (
                format!(
                "https://api.bilibili.com/x/player/playurl?bvid={}&cid={}&qn=80&fnval=4048&fourk=1",
                bvid, cid
            ),
                None,
            )
        };

        let headers = self.build_headers(true);

        let response = self
            .client
            .get(&url)
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
                    QualityInfo {
                        id: 32,
                        desc: "480P 清晰".to_string(),
                        is_available: true,
                    },
                    QualityInfo {
                        id: 16,
                        desc: "360P 流畅".to_string(),
                        is_available: true,
                    },
                ]);
            }
            return Err(format!("获取分辨率失败: code={}", response.code));
        }

        let data = response.data.ok_or_else(|| "播放数据为空".to_string())?;

        // current_quality 是API返回的"推荐"画质，免登录时通常是64
        // 但我们不使用它来判断可用性，因为DASH流中实际包含更高画质
        let _current_quality = data.quality;
        let accept_qualities = data.accept_quality.clone();
        let accept_descriptions = data.accept_description.clone();

        let is_logged_in = self.cookies.read().is_some();
        let is_vip = self
            .user_info
            .read()
            .as_ref()
            .map_or(false, |info| info.is_vip);

        let mut qualities = Vec::new();
        // 更新映射表，标记各画质是否需要登录/大会员
        // 实测：免登录+try_look=1时，DASH流中实际包含1080P(id=80)
        let quality_map = [
            (127, "8K 超高清", true),     // 需要大会员
            (120, "4K 超清", true),       // 需要大会员
            (116, "1080P 60帧", true),    // 需要大会员
            (112, "1080P+", true),        // 需要大会员
            (80, "1080P 高清", false),    // 免登录可用！
            (64, "720P 高清", false),
            (32, "480P 清晰", false),
            (16, "360P 流畅", false),
        ];

        for (i, &quality_id) in accept_qualities.iter().enumerate() {
            let desc = if i < accept_descriptions.len() {
                accept_descriptions[i].clone()
            } else {
                quality_map
                    .iter()
                    .find(|&&(id, _, _)| id == quality_id)
                    .map(|(_, desc, _)| desc.to_string())
                    .unwrap_or_else(|| format!("质量 {}", quality_id))
            };

            // 关键修复：正确判断画质可用性
            // 大会员画质(id > 80): 需要登录且是大会员
            // 非大会员画质(id <= 80): 只要在accept_quality列表中就可用（免登录+try_look）
            let needs_vip = quality_id > 80;
            
            let is_available = if needs_vip {
                // 大会员画质：需要登录且是会员
                is_logged_in && is_vip
            } else {
                // 非大会员画质：只要API返回了这个画质就可用
                // 因为使用了try_look=1参数，DASH流中会包含这些画质
                true
            };

            qualities.push(QualityInfo {
                id: quality_id,
                desc: if !is_available {
                    if needs_vip {
                        format!("{} (需要大会员)", desc)
                    } else {
                        // 非大会员画质理论上都可用，这个分支不应该被执行
                        format!("{} (不可用)", desc)
                    }
                } else {
                    desc
                },
                is_available,
            });
        }

        if qualities.is_empty() {
            qualities.push(QualityInfo {
                id: 32,
                desc: "480P 清晰".to_string(),
                is_available: true,
            });
            qualities.push(QualityInfo {
                id: 16,
                desc: "360P 流畅".to_string(),
                is_available: true,
            });
        }

        Ok(qualities)
    }

    // --- 修改：获取实际画质，使用 Wbi ---
    pub async fn get_actual_quality(
        &self,
        bvid: &str,
        cid: u64,
        requested_quality: u32,
    ) -> Result<u32, String> {
        let keys_opt = self.update_wbi_keys().await.ok();

        let url = if let Some((img_key, sub_key)) = keys_opt {
            let mut params = BTreeMap::new();
            params.insert("bvid".to_string(), bvid.to_string());
            params.insert("cid".to_string(), cid.to_string());
            params.insert("qn".to_string(), requested_quality.to_string());
            params.insert("fnval".to_string(), "4048".to_string());
            params.insert("fourk".to_string(), "1".to_string());
            params.insert("try_look".to_string(), "1".to_string()); // 关键

            let query = Self::encode_wbi(&params, &img_key, &sub_key);
            format!("https://api.bilibili.com/x/player/wbi/playurl?{}", query)
        } else {
            format!(
                "https://api.bilibili.com/x/player/playurl?bvid={}&cid={}&qn={}&fnval=4048&fourk=1",
                bvid, cid, requested_quality
            )
        };

        debug_println!("检查实际可获取的画质: {}", requested_quality);

        let headers = self.build_headers(true);

        let response = self
            .client
            .get(&url)
            .headers(headers)
            .send()
            .await
            .map_err(|e| format!("请求失败: {}", e))?;

        let response_text = response
            .text()
            .await
            .map_err(|e| format!("读取响应失败: {}", e))?;

        let response: PlayUrlResponse =
            serde_json::from_str(&response_text).map_err(|e| format!("解析JSON失败: {}", e))?;

        if response.code != 0 {
            return Err(format!("获取画质失败: code={}", response.code));
        }

        let data = response.data.ok_or_else(|| "播放数据为空".to_string())?;

        let actual_quality = data.quality;

        debug_println!(
            "请求画质: {}, 实际获得画质: {}",
            requested_quality,
            actual_quality
        );

        Ok(actual_quality)
    }

    // --- 修改：获取下载链接，使用 Wbi ---
    // 关键修复：免登录1080P支持
    // API返回的quality字段可能是64(720P)，但DASH数据中实际包含更高画质的流(如80=1080P)
    // 所以我们保留用户请求的画质，优先在DASH流中查找
    pub async fn get_download_urls(
        &self,
        bvid: &str,
        cid: u64,
        quality: u32,
    ) -> Result<(String, String), String> {
        // 保留用户请求的画质，用于后续在DASH流中查找
        let requested_quality = quality;
        // 获取API返回的"官方"画质（通常免登录返回64，但DASH中可能有80）
        let actual_quality = self.get_actual_quality(bvid, cid, quality).await?;
        
        debug_println!("请求画质: {}, API返回画质: {}", requested_quality, actual_quality);

        let keys_opt = self.update_wbi_keys().await.ok();

        // 使用请求的画质来构造URL，API会返回所有可用的DASH流
        let url = if let Some((img_key, sub_key)) = keys_opt {
            let mut params = BTreeMap::new();
            params.insert("bvid".to_string(), bvid.to_string());
            params.insert("cid".to_string(), cid.to_string());
            params.insert("qn".to_string(), requested_quality.to_string()); // 使用请求的画质
            params.insert("fnval".to_string(), "4048".to_string());
            params.insert("fourk".to_string(), "1".to_string());
            params.insert("try_look".to_string(), "1".to_string()); // 关键：免登录高画质

            let query = Self::encode_wbi(&params, &img_key, &sub_key);
            format!("https://api.bilibili.com/x/player/wbi/playurl?{}", query)
        } else {
            format!(
                "https://api.bilibili.com/x/player/playurl?bvid={}&cid={}&qn={}&fnval=4048&fourk=1&try_look=1",
                bvid, cid, requested_quality // 使用请求的画质，并添加try_look参数
            )
        };

        debug_println!("请求播放地址，请求画质: {}", requested_quality);

        let headers = self.build_headers(true);

        let response = self
            .client
            .get(&url)
            .headers(headers)
            .send()
            .await
            .map_err(|e| format!("请求失败: {}", e))?;

        let response_text = response
            .text()
            .await
            .map_err(|e| format!("读取响应失败: {}", e))?;

        debug_println!(
            "API响应前500字符: {}",
            &response_text[..response_text.len().min(500)]
        );

        let response: PlayUrlResponse =
            serde_json::from_str(&response_text).map_err(|e| format!("解析JSON失败: {}", e))?;

        if response.code != 0 {
            if response.code == -400 || response.code == -404 {
                return self.get_download_urls_fallback(bvid, cid).await;
            }
            return Err(format!(
                "获取下载地址失败: code={}, message={}",
                response.code,
                response.message.unwrap_or_else(|| "未知错误".to_string())
            ));
        }

        let data = response
            .data
            .ok_or_else(|| "下载地址数据为空".to_string())?;

        if let Some(dash) = data.dash {
            if !dash.video.is_empty() && !dash.audio.is_empty() {
                // 关键修复：优先使用用户请求的画质，而不是API返回的quality
                // 因为免登录时API返回quality=64，但DASH中实际有id=80(1080P)的流
                // 同时优先选择AVC编码（兼容性更好）
                let video = dash
                    .video
                    .iter()
                    .filter(|v| v.id == requested_quality)
                    .find(|v| v.codecs.starts_with("avc")) // 优先AVC编码
                    .or_else(|| dash.video.iter().find(|v| v.id == requested_quality)) // 或任意编码的requested_quality
                    .or_else(|| dash.video.iter().filter(|v| v.id == actual_quality).find(|v| v.codecs.starts_with("avc")))
                    .or_else(|| dash.video.iter().find(|v| v.id == actual_quality))
                    .or_else(|| {
                        // 最后按画质从高到低找第一个可用的AVC流
                        let mut sorted: Vec<_> = dash.video.iter()
                            .filter(|v| v.codecs.starts_with("avc"))
                            .collect();
                        sorted.sort_by(|a, b| b.id.cmp(&a.id));
                        sorted.first().copied()
                    })
                    .or_else(|| {
                        // 如果没有AVC，就选最高画质的任意流
                        let mut sorted: Vec<_> = dash.video.iter().collect();
                        sorted.sort_by(|a, b| b.id.cmp(&a.id));
                        sorted.first().copied()
                    })
                    .ok_or_else(|| "没有可用的视频流".to_string())?;
                
                debug_println!("选择的视频流: id={}, codec={}, {}x{}", video.id, video.codecs, video.width, video.height);

                let audio = dash
                    .audio
                    .first()
                    .ok_or_else(|| "没有可用的音频流".to_string())?;

                let video_url = if video.base_url.contains("xy") {
                    video
                        .backup_url
                        .as_ref()
                        .and_then(|urls| urls.first())
                        .unwrap_or(&video.base_url)
                        .clone()
                } else {
                    video.base_url.clone()
                };

                let audio_url = if audio.base_url.contains("xy") {
                    audio
                        .backup_url
                        .as_ref()
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

    async fn get_download_urls_fallback(
        &self,
        bvid: &str,
        cid: u64,
    ) -> Result<(String, String), String> {
        // Fallback 也可以尝试使用 Wbi，但这里为了保持逻辑简单，保留原来的低画质请求作为最后的救命稻草
        let url = format!(
            "https://api.bilibili.com/x/player/playurl?bvid={}&cid={}&qn=32&fnval=1",
            bvid, cid
        );

        debug_println!("尝试获取低质量视频地址: {}", url);

        let headers = self.build_headers(false);

        let response = self
            .client
            .get(&url)
            .headers(headers)
            .send()
            .await
            .map_err(|e| format!("请求失败: {}", e))?;

        let response: PlayUrlResponse = response
            .json()
            .await
            .map_err(|e| format!("解析JSON失败: {}", e))?;

        if response.code != 0 {
            return Err(format!("获取下载地址失败: code={}", response.code));
        }

        let data = response
            .data
            .ok_or_else(|| "下载地址数据为空".to_string())?;

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
                .header(
                    "Accept",
                    "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8",
                )
                .header("Accept-Language", "zh-CN,zh;q=0.9,en;q=0.8")
                .send()
                .await
                .map_err(|e| format!("请求失败: {}", e))?;

            let status = response.status();
            debug_println!("状态码: {}, URL: {}", status, current_url);

            if status.is_redirection() {
                if let Some(location) = response.headers().get("location") {
                    let location_str = location
                        .to_str()
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
            let end_pos = url_part
                .find(|c: char| {
                    c.is_whitespace() || c == '】' || c == '"' || c == '\'' || c == '>' || c == '<'
                })
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
            let end_pos = url_part
                .find(|c: char| {
                    c.is_whitespace() || c == '】' || c == '"' || c == '\'' || c == '>' || c == '<'
                })
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
            let bvid = actual_input
                .chars()
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
                let bvid: String = bvid_start
                    .chars()
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

        let response = self
            .client
            .get(url)
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

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("请求失败: {}", e))?;

        let headers = response.headers().clone();
        let response_text = response
            .text()
            .await
            .map_err(|e| format!("读取响应失败: {}", e))?;

        let response: QrcodePollResponse =
            serde_json::from_str(&response_text).map_err(|e| format!("解析JSON失败: {}", e))?;

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
                    cookies = format!(
                        "SESSDATA=mock_session_{}",
                        SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .unwrap()
                            .as_secs()
                    );
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
