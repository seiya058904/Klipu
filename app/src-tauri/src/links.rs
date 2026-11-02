fn opens_the_web(url: &str) -> bool {
    let said = url.trim_start();
    let Some((scheme, rest)) = said.split_once("://") else {
        return false;
    };
    !rest.is_empty()
        && (scheme.eq_ignore_ascii_case("http") || scheme.eq_ignore_ascii_case("https"))
}

#[tauri::command(async)]
pub fn open_web(url: String) -> Result<(), String> {
    if opens_the_web(&url) && cp_win_sys::files::open_link(&url) {
        return Ok(());
    }
    Err(url)
}
