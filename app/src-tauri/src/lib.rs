mod backup;
mod emptying;
mod keys;
mod links;
mod looks;
mod note;
mod panel;
mod ready;
mod report;
mod reviving;
mod settings;
mod tray;
mod waking;
mod welcome;

pub fn run() {
    note::catch_panics();
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            welcome::reopened(app);
        }))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .invoke_handler(tauri::generate_handler![
            settings::settings,
            settings::keep,
            settings::looks,
            settings::where_it_lives,
            settings::storage_used,
            settings::notices,
            settings::licences,
            links::open_web,
            waking::waking,
            waking::wake,
            keys::keys,
            keys::spare,
            backup::save_backup,
            backup::peek_backup,
            backup::load_backup,
            backup::cancel_backup,
            backup::repair_search,
            report::save_report,
            report::open_log,
            empty,
            trouble,
            relabel,
            welcome::greeting,
            welcome::tour,
            welcome::open_settings,
            ready::glance,
            ready::show_panel
        ])
        .manage(backup::Crossing::default())
        .setup(|app| {
            let fresh = settings::settle();
            let kept = settings::settings().ok();
            let chinese = tray::chinese(kept.as_ref().and_then(|one| one.locale.as_deref()));
            if tray::raise(app.handle(), chinese).is_none() {
                tray::surface(app.handle());
            }
            waking::settle();
            if fresh {
                waking::on_first_run();
            }
            panel::raise(app.handle());
            keys::raise(app.handle(), &kept.unwrap_or_else(keys::unread));
            welcome::raise(app.handle(), fresh);
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("Klipu could not start");

    app.run(|app, event| match event {
        tauri::RunEvent::ExitRequested {
            code: None, api, ..
        } => api.prevent_exit(),
        tauri::RunEvent::Exit => panel::quit(app),
        tauri::RunEvent::WindowEvent {
            event: tauri::WindowEvent::ThemeChanged(_),
            ..
        } => looks::wear(app),
        _ => {}
    });
}

#[tauri::command(async)]
fn empty(app: tauri::AppHandle) -> Result<i64, String> {
    panel::empty(&app)
}

#[tauri::command]
fn trouble(app: tauri::AppHandle) -> Option<&'static str> {
    panel::trouble(&app).map(cp_core::trouble::Trouble::key)
}

#[tauri::command]
fn relabel(app: tauri::AppHandle, locale: Option<String>) {
    let chinese = tray::chinese(locale.as_deref());
    tray::reword(&app, chinese);
    tray::reword_tip(&app, chinese);
}
