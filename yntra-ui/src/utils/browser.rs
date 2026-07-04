pub fn open_in_system_browser(_url: &str) {
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        let escaped_url = format!("\"{}\"", _url);
        let _ = std::process::Command::new("cmd")
            .args(&["/C", "start", "", &escaped_url])
            .creation_flags(0x08000000) // CREATE_NO_WINDOW
            .spawn();
    }
    #[cfg(target_os = "macos")]
    let _ = std::process::Command::new("open").arg(_url).spawn();
    #[cfg(target_os = "linux")]
    let _ = std::process::Command::new("xdg-open").arg(_url).spawn();
}
