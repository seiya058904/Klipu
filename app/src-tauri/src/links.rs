fn opened(url: &str) -> bool {
    #[cfg(windows)]
    {
        cp_win_sys::files::open_link(url)
    }
}

#[tauri::command(async)]
pub fn open_web(url: String) -> Result<(), String> {
    if !cp_core::linkunbound::opens_the_web(&url) {
        return Err(url);
    }
    if opened(&url) {
        return Ok(());
    }
    Err(url)
}

#[tauri::command(async)]
pub fn linkunbound_here() -> bool {
    #[cfg(windows)]
    {
        cp_win_sys::files::linkunbound_here()
    }
}
