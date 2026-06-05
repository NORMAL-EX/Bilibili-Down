#[cfg(target_os = "windows")]
fn main() {
    let mut res = winresource::WindowsResource::new();
    res.set_icon("assets/bilibili.ico");
    res.compile().unwrap();
}

#[cfg(not(target_os = "windows"))]
fn main() {}