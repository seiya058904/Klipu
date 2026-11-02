use crate::keys::Role;
use cp_config::Config;
use std::path::PathBuf;

pub const NOTICES: &str = include_str!("../../../THIRD-PARTY-BUNDLED.md");

pub const LICENCES: &str = include_str!("../../../THIRD-PARTY-LICENSES.md");

#[tauri::command]
pub fn notices() -> &'static str {
    NOTICES
}

#[tauri::command]
pub fn licences() -> &'static str {
    LICENCES
}

pub fn folder() -> Option<PathBuf> {
    #[cfg(windows)]
    {
        cp_win_sys::paths::data_dir()
    }
}

pub fn nowhere() -> String {
    "the folder where CoPanda keeps its own was not found".to_owned()
}

pub fn history() -> Result<PathBuf, String> {
    Ok(folder().ok_or_else(nowhere)?.join("history.db"))
}

pub fn open_history() -> Result<cp_store::Store, String> {
    cp_store::Store::open(&history()?).map_err(|why| why.to_string())
}

pub fn history_if_any() -> Result<Option<cp_store::Store>, String> {
    if !history()?.exists() {
        return Ok(None);
    }
    open_history().map(Some)
}

pub fn settle() -> bool {
    let Some(dir) = folder() else {
        return false;
    };
    let path = cp_config::at(&dir);
    if path.exists() {
        return false;
    }
    match cp_config::write(&path, &Config::default()) {
        Ok(()) => true,
        Err(why) => {
            crate::note::note(&format!("the settings could not be started off: {why}"));
            false
        }
    }
}

#[tauri::command]
pub fn settings() -> Result<Config, String> {
    let dir = folder().ok_or_else(nowhere)?;
    read_or_reset(&cp_config::at(&dir)).map_err(|why| why.to_string())
}

fn read_or_reset(path: &std::path::Path) -> Result<Config, cp_config::Error> {
    let at_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |since| since.as_millis() as i64);
    cp_config::read_or_reset(path, at_ms)
}

#[tauri::command]
pub fn looks() -> cp_config::look::Choices {
    cp_config::look::choices_here()
}

#[tauri::command]
pub fn keep(app: tauri::AppHandle, config: Config) -> Result<Config, String> {
    let said = written(&app, config);
    crate::looks::wear(&app);
    if let Ok(now) = settings() {
        let _ = tauri::Emitter::emit(&app, "kept", now);
    }
    said
}

fn written(app: &tauri::AppHandle, config: Config) -> Result<Config, String> {
    let dir = folder().ok_or_else(nowhere)?;
    let path = cp_config::at(&dir);
    let was = read_or_reset(&path).ok();
    let config = Config {
        welcomed: was.as_ref().and_then(|one| one.welcomed.clone()),
        ..config
    };
    cp_config::write(&path, &config).map_err(|why| why.to_string())?;
    let landed = cp_config::read(&path).map_err(|why| why.to_string())?;
    let moved: Vec<Role> = Role::ALL
        .into_iter()
        .filter(|role| was.as_ref().map(|one| role.of(one)) != Some(role.of(&landed)))
        .collect();
    let Err(why) = rebound(app, &moved, &landed) else {
        return Ok(landed);
    };
    crate::note::note(&format!("the new shortcut could not be taken: {why}"));
    let mut back = landed;
    let before = was.unwrap_or_else(crate::keys::unread);
    for role in &moved {
        role.set(&mut back, role.of(&before).to_owned());
    }
    if let Err(twice) = rebound(app, &moved, &back) {
        crate::note::note(&format!(
            "and the ones before it did not all come back either: {twice}"
        ));
    }
    let _ = cp_config::write(&path, &back);
    Err(why)
}

fn rebound(app: &tauri::AppHandle, moved: &[Role], config: &Config) -> Result<(), String> {
    for role in moved {
        crate::keys::release(app, *role);
    }
    let mut refused = Ok(());
    for role in moved {
        if let Err(why) = crate::keys::bind(app, *role, role.of(config)) {
            refused = refused.and(Err(why));
        }
    }
    refused
}

#[tauri::command(async)]
pub fn storage_used() -> Result<i64, String> {
    let Some(store) = history_if_any()? else {
        return Ok(0);
    };
    Ok(store.usage().map_err(|why| why.to_string())?.bytes)
}

#[tauri::command]
pub fn where_it_lives() -> Result<String, String> {
    let dir = folder().ok_or_else(nowhere)?;
    Ok(dir.to_string_lossy().into_owned())
}
