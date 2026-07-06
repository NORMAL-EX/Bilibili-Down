use libloading::{Library, Symbol};
use std::ffi::CString;
use std::os::raw::c_char;
use std::path::{Path, PathBuf};

type AudioToMp3Fn = unsafe extern "C" fn(
    input_path: *const c_char,
    output_path: *const c_char,
    bitrate_kbps: u32,
) -> i32;

pub fn audio_to_mp3(input_path: &Path, output_path: &Path, bitrate_kbps: u32) -> Result<(), String> {
    let dll_path = find_media_library().ok_or_else(|| {
        "未找到媒体处理 DLL，请把 ffmpeg.dll 放到程序目录或 tools 目录".to_string()
    })?;

    let input = path_to_cstring(input_path)?;
    let output = path_to_cstring(output_path)?;

    unsafe {
        let library = Library::new(&dll_path)
            .map_err(|e| format!("加载媒体处理 DLL {:?} 失败: {}", dll_path, e))?;
        let audio_to_mp3: Symbol<AudioToMp3Fn> = library
            .get(b"bilibili_audio_to_mp3\0")
            .map_err(|e| format!("DLL 缺少 bilibili_audio_to_mp3 导出函数: {}", e))?;

        let code = audio_to_mp3(input.as_ptr(), output.as_ptr(), bitrate_kbps);
        if code == 0 {
            Ok(())
        } else {
            Err(format!("DLL 转换 MP3 失败，错误码: {}", code))
        }
    }
}

fn find_media_library() -> Option<PathBuf> {
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|path| path.parent().map(|parent| parent.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."));

    #[cfg(target_os = "windows")]
    let library_name = "ffmpeg.dll";
    #[cfg(target_os = "macos")]
    let library_name = "libffmpeg.dylib";
    #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
    let library_name = "libffmpeg.so";

    [
        exe_dir.join("tools").join(library_name),
        exe_dir.join(library_name),
    ]
    .into_iter()
    .find(|path| path.exists())
}

fn path_to_cstring(path: &Path) -> Result<CString, String> {
    CString::new(path.to_string_lossy().as_bytes())
        .map_err(|_| format!("路径包含 NUL 字符: {:?}", path))
}
