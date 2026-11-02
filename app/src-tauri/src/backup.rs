use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Default)]
pub struct Crossing(AtomicBool);

pub struct Alone<'a>(&'a AtomicBool);

impl Crossing {
    fn claim(&self) -> Option<Alone<'_>> {
        self.0
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
            .then(|| Alone(&self.0))
    }

    pub fn underway(&self) -> bool {
        self.0.load(Ordering::Acquire)
    }
}

impl Drop for Alone<'_> {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Release);
    }
}

#[derive(serde::Serialize)]
pub struct Saved {
    path: String,
    items: i64,
    missing: i64,
    bytes: u64,
}

#[derive(serde::Serialize)]
pub struct Brought {
    added: i64,
    already: i64,
    refused: i64,
    #[serde(rename = "fromElsewhere")]
    from_elsewhere: bool,
}

#[tauri::command(async)]
pub fn save_backup(path: String, at: i64) -> Result<Saved, String> {
    let store = crate::settings::open_history()?;
    let to = PathBuf::from(&path);
    let made = cp_store::backup::write(&store, &to, at).map_err(|why| why.to_string())?;
    crate::note::note(&format!("a copy written to {path}: {} items", made.items));
    Ok(Saved {
        path,
        items: made.items,
        missing: made.missing,
        bytes: made.bytes,
    })
}

#[tauri::command(async)]
pub fn peek_backup(path: String) -> Result<Saved, String> {
    let taken = cp_store::backup::read(&PathBuf::from(&path)).map_err(|why| why.to_string())?;
    Ok(Saved {
        path,
        items: taken.items,
        missing: 0,
        bytes: taken.bytes,
    })
}

#[tauri::command(async)]
pub fn load_backup(
    app: tauri::AppHandle,
    alone: tauri::State<'_, Crossing>,
    path: String,
    at: i64,
) -> Result<Brought, String> {
    let _busy = alone
        .inner()
        .claim()
        .ok_or_else(|| "the history is already being written to".to_owned())?;
    let from = PathBuf::from(&path);
    cp_store::backup::read(&from).map_err(|why| why.to_string())?;
    crate::panel::quit(&app);
    let landed = bring(&from, at);
    if let Err(why) = crate::panel::relight(&app) {
        crate::note::note(&format!(
            "the panel did not come back after the copy: {why}"
        ));
    }
    let brought = landed?;
    crate::note::note(&format!(
        "{} items came from {path} and {} were already here",
        brought.added, brought.already
    ));
    Ok(Brought {
        added: brought.added,
        already: brought.already,
        refused: brought.refused,
        from_elsewhere: brought.from_elsewhere,
    })
}

fn bring(from: &std::path::Path, at: i64) -> Result<cp_store::Brought, String> {
    let store = crate::settings::open_history()?;
    cp_store::backup::bring(from, &store, at, &|what| crate::note::note(what))
        .map_err(|why| why.to_string())
}

#[cfg(test)]
#[path = "backup_test.rs"]
mod tests;
