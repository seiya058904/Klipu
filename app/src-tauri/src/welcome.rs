use cp_config::Config;
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager, Runtime, WebviewUrl, WebviewWindowBuilder};

pub const LABEL: &str = "welcome";

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Greeting {
    Tour,
    Keys,
    Nothing,
}

#[derive(Default)]
pub struct Now(Mutex<Option<Greeting>>);

pub fn decide(fresh: bool, welcomed: Option<&str>) -> Greeting {
    let since = welcomed.and_then(|one| semver::Version::parse(one).ok());
    if fresh || since.is_none() {
        return Greeting::Tour;
    }
    Greeting::Nothing
}

pub fn behind(welcomed: Option<&str>, running: &str) -> bool {
    let Ok(now) = semver::Version::parse(running) else {
        return false;
    };
    welcomed
        .and_then(|one| semver::Version::parse(one).ok())
        .is_none_or(|since| since < now)
}

pub fn raise<R: Runtime>(app: &AppHandle<R>, fresh: bool) {
    let kept = crate::settings::settings().ok();
    if kept.is_none() && !fresh {
        app.manage(Now(Mutex::new(Some(Greeting::Nothing))));
        return;
    }
    let welcomed = kept.as_ref().and_then(|one| one.welcomed.clone());
    let running = app.package_info().version.to_string();
    let asked = decide(fresh, welcomed.as_deref());
    let quiet = asked == Greeting::Nothing;
    app.manage(Now(Mutex::new(Some(asked))));
    if !quiet {
        open(app);
        return;
    }
    if behind(welcomed.as_deref(), &running) {
        mark(app);
    }
    crate::ready::open(app);
}

fn mark<R: Runtime>(app: &AppHandle<R>) {
    let Some(dir) = crate::settings::folder() else {
        return;
    };
    let path = cp_config::at(&dir);
    let Ok(kept) = cp_config::read(&path) else {
        return;
    };
    let running = app.package_info().version.to_string();
    if !behind(kept.welcomed.as_deref(), &running) {
        return;
    }
    let written = cp_config::write(
        &path,
        &Config {
            welcomed: Some(running),
            ..kept
        },
    );
    if let Err(why) = written {
        crate::note::note(&format!("the welcome could not be written down: {why}"));
    }
}

pub fn open<R: Runtime>(app: &AppHandle<R>) {
    if let Some(window) = app.get_webview_window(LABEL) {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
        return;
    }
    let built = crate::looks::dressed(WebviewWindowBuilder::new(
        app,
        LABEL,
        WebviewUrl::App("index.html#welcome".into()),
    ))
    .title("Klipu")
    .inner_size(600.0, 460.0)
    .resizable(false)
    .maximizable(false)
    .decorations(false)
    .center()
    .build();
    match built {
        Ok(window) => {
            let handle = app.clone();
            window.on_window_event(move |event| {
                if matches!(event, tauri::WindowEvent::Destroyed) && !only_keys(&handle) {
                    mark(&handle);
                }
            });
            let _ = window.set_focus();
        }
        Err(why) => crate::note::note(&format!("the welcome would not open: {why}")),
    }
}

pub fn remembers(greeting: Option<&Greeting>) -> bool {
    !matches!(greeting, Some(Greeting::Keys))
}

fn only_keys<R: Runtime>(app: &AppHandle<R>) -> bool {
    app.try_state::<Now>()
        .and_then(|state| state.0.lock().ok().map(|held| !remembers(held.as_ref())))
        .unwrap_or(false)
}

fn hold<R: Runtime>(app: &AppHandle<R>, greeting: Greeting) {
    if let Some(state) = app.try_state::<Now>()
        && let Ok(mut held) = state.0.lock()
    {
        *held = Some(greeting);
    }
}

pub fn reopened<R: Runtime>(app: &AppHandle<R>) {
    if app.get_webview_window(LABEL).is_none() {
        hold(app, Greeting::Keys);
    }
    crate::ready::close(app);
    open(app);
    if let Some(window) = app.get_webview_window(LABEL) {
        let _ = window.set_always_on_top(true);
    }
}

pub fn unpin<R: Runtime>(app: &AppHandle<R>) {
    if let Some(window) = app.get_webview_window(LABEL) {
        let _ = window.set_always_on_top(false);
    }
}

#[tauri::command]
pub fn greeting(state: tauri::State<'_, Now>) -> Greeting {
    state
        .0
        .lock()
        .ok()
        .and_then(|held| held.clone())
        .unwrap_or(Greeting::Nothing)
}

#[tauri::command]
pub async fn tour(app: AppHandle) {
    let wanted = Greeting::Tour;
    hold(&app, wanted.clone());
    if app.get_webview_window(LABEL).is_some() {
        let _ = app.emit_to(LABEL, "greeting", wanted);
    }
    open(&app);
}

#[tauri::command]
pub async fn open_settings(app: AppHandle, rail: String) {
    crate::tray::surface_at(&app, Some(&rail));
}

#[cfg(test)]
#[path = "welcome_test.rs"]
mod tests;
