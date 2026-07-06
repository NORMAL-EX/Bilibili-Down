use std::os::raw::c_char;

#[no_mangle]
pub extern "C" fn bilibili_audio_to_mp3(
    _input_path: *const c_char,
    _output_path: *const c_char,
    _bitrate_kbps: u32,
) -> i32 {
    -1
}
