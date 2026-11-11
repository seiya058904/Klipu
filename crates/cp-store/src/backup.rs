use crate::{Blobs, Error, Result, Store};
use rusqlite::Connection;
use std::path::Path;

pub const FORMAT: u32 = 1;
pub const EXTENSION: &str = "klipubak";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Made {
    pub items: i64,
    pub blobs: i64,
    pub missing: i64,
    pub bytes: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Brought {
    pub added: i64,
    pub already: i64,
    pub refused: i64,
    pub partial: i64,
    pub repaired: i64,
    pub blobs: i64,
    pub from_elsewhere: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Taken {
    pub format: u32,
    pub schema: u32,
    pub items: i64,
    pub written_at: i64,
    pub bytes: u64,
    pub platform: Option<String>,
}

pub fn write(store: &Store, to: &Path, at: i64) -> Result<Made> {
    if let Some(parent) = to.parent() {
        std::fs::create_dir_all(parent).map_err(Error::Io)?;
    }
    if lives_at(store.raw()).is_some_and(|live| same_file(&live, to)) {
        return Err(Error::OntoItself);
    }
    let landing = sidecar(to, ".part");
    for path in [
        landing.clone(),
        sidecar(&landing, "-wal"),
        sidecar(&landing, "-shm"),
    ] {
        remove(&path)?;
    }
    let made = written_at(store, &landing, at);
    if made.is_err() {
        for path in [
            landing.clone(),
            sidecar(&landing, "-wal"),
            sidecar(&landing, "-shm"),
        ] {
            let _ = remove(&path);
        }
        return made;
    }
    std::fs::rename(&landing, to).map_err(Error::Io)?;
    crate::store::restrict(to, 0o600)?;
    made.map(|made| Made {
        bytes: weighed(to),
        ..made
    })
}

fn written_at(store: &Store, to: &Path, at: i64) -> Result<Made> {
    store
        .raw()
        .execute("VACUUM INTO ?1", [&to.to_string_lossy()])?;
    crate::store::restrict(to, 0o600)?;

    let copy = Connection::open(to)?;
    copy.execute_batch(
        "CREATE TABLE backup_blobs (digest TEXT PRIMARY KEY, bytes BLOB NOT NULL);
         CREATE TABLE backup_note (key TEXT PRIMARY KEY, value TEXT NOT NULL);",
    )?;

    let blobs = store.blobs();
    let mut kept = 0;
    let mut missing = 0;
    if let Some(blobs) = blobs {
        let digests = wanted(&copy)?;
        let mut insert =
            copy.prepare("INSERT OR IGNORE INTO backup_blobs (digest, bytes) VALUES (?1, ?2)")?;
        for digest in digests {
            let Some(bytes) = blobs.get(&digest)? else {
                missing += 1;
                continue;
            };
            insert.execute(rusqlite::params![digest, bytes])?;
            kept += 1;
        }
    }

    let items: i64 = copy.query_row(
        "SELECT COUNT(*) FROM items WHERE deleted_at IS NULL",
        [],
        |row| row.get(0),
    )?;
    for (key, value) in [
        ("format", FORMAT.to_string()),
        ("schema", crate::SCHEMA_VERSION.to_string()),
        ("written_at", at.to_string()),
        ("items", items.to_string()),
        ("platform", WHERE_IT_WAS_MADE.to_owned()),
    ] {
        copy.execute(
            "INSERT OR REPLACE INTO backup_note (key, value) VALUES (?1, ?2)",
            rusqlite::params![key, value],
        )?;
    }
    copy.execute_batch("PRAGMA journal_mode = DELETE; VACUUM;")?;
    drop(copy);
    for side in [sidecar(to, "-wal"), sidecar(to, "-shm")] {
        remove(&side)?;
    }

    Ok(Made {
        items,
        blobs: kept,
        missing,
        bytes: weighed(to),
    })
}

#[cfg(target_os = "windows")]
pub const WHERE_IT_WAS_MADE: &str = "windows";

pub fn read(from: &Path) -> Result<Taken> {
    let copy = Connection::open_with_flags(from, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    let format = u32::try_from(noted(&copy, "format")?.ok_or(Error::NotABackup)?)
        .map_err(|_| Error::NotABackup)?;
    if format > FORMAT {
        return Err(Error::BackupFromTheFuture {
            found: format,
            supported: FORMAT,
        });
    }
    Ok(Taken {
        format,
        schema: u32::try_from(noted(&copy, "schema")?.unwrap_or(0)).unwrap_or(0),
        items: noted(&copy, "items")?.unwrap_or(0),
        written_at: noted(&copy, "written_at")?.unwrap_or(0),
        bytes: weighed(from),
        platform: said(&copy, "platform")?,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Preview {
    pub added: i64,
    pub already: i64,
    pub refused: i64,
    pub partial: i64,
    pub repaired: i64,
    pub expires: i64,
    pub bytes_after: i64,
    pub quota: Option<i64>,
    pub keep_for: Option<i64>,
    pub from_elsewhere: bool,
}

impl Preview {
    pub fn needs_review_after(&self, previous: &Self) -> bool {
        self.keep_for != previous.keep_for
            || self.quota != previous.quota
            || self.expires > previous.expires
            || self.refused > previous.refused
            || self.partial > previous.partial
            || (self.quota.is_some_and(|quota| self.bytes_after > quota)
                && !previous
                    .quota
                    .is_some_and(|quota| previous.bytes_after > quota))
    }
}

pub struct Prepared {
    source: Store,
    opened: Opened,
}

impl Prepared {
    pub fn open(from: &Path) -> Result<Self> {
        let opened = Opened::of(from)?;
        if opened.taken.schema > crate::SCHEMA_VERSION {
            return Err(Error::FromTheFuture {
                found: opened.taken.schema,
                supported: crate::SCHEMA_VERSION,
            });
        }
        let source = Store::open(&opened.db)?;
        Ok(Self { source, opened })
    }

    pub fn preview(&self, into: &Store, policy: &crate::Policy, at: i64) -> Result<Preview> {
        let mut preview = Preview {
            bytes_after: into.usage()?.bytes,
            quota: policy.bytes_at_most,
            keep_for: policy.keep_for,
            from_elsewhere: self.is_from_elsewhere(),
            ..Default::default()
        };
        let mut known = std::collections::HashSet::new();
        let mut blobs = into
            .raw()
            .prepare("SELECT DISTINCT digest FROM item_formats WHERE digest IS NOT NULL")?
            .query_map([], |row| row.get::<_, String>(0))?
            .collect::<rusqlite::Result<std::collections::HashSet<_>>>()?;
        self.each(&mut |row| {
            let Some((item, _, partial)) = carried_item(&self.source, row)? else {
                preview.refused += 1;
                return Ok(());
            };
            if let Some(id) = into.find_by_hash(&item)? {
                preview.already += 1;
                if let Some(local) = into.item(id)? {
                    preview.repaired += i64::from(into.search_needs_repair(id, &local)?);
                }
                return Ok(());
            }
            if !known.insert(item.fingerprint()) {
                preview.already += 1;
                return Ok(());
            }
            preview.added += 1;
            preview.partial += i64::from(partial);
            if !row.pinned
                && policy.keep_for.is_some_and(|age| {
                    row.modified_at.max(row.last_used_at.unwrap_or(0)) < at.saturating_sub(age)
                })
            {
                preview.expires += 1;
            }
            for format in &item.formats {
                match &format.payload {
                    cp_core::item::Payload::Inline(bytes) => {
                        preview.bytes_after = preview.bytes_after.saturating_add(bytes.len() as i64)
                    }
                    cp_core::item::Payload::Blob(bytes)
                        if blobs.insert(blake3::hash(bytes).to_hex().to_string()) =>
                    {
                        preview.bytes_after =
                            preview.bytes_after.saturating_add(bytes.len() as i64);
                    }
                    _ => {}
                }
            }
            Ok(())
        })?;
        Ok(preview)
    }

    pub fn bring(&self, into: &Store, at: i64, say: &dyn Fn(&str)) -> Result<Brought> {
        let mut brought = Brought {
            added: 0,
            already: 0,
            refused: 0,
            partial: 0,
            repaired: 0,
            blobs: self.opened.blobs,
            from_elsewhere: self.is_from_elsewhere(),
        };
        self.each(&mut |row| {
            match carry(&self.source, into, row, at) {
                Ok(Carried::Added { partial }) => {
                    brought.added += 1;
                    brought.partial += i64::from(partial);
                }
                Ok(Carried::Already { repaired }) => {
                    brought.already += 1;
                    brought.repaired += i64::from(repaired);
                }
                Ok(Carried::Unreadable) => brought.refused += 1,
                Err(why) => {
                    say(&format!(
                        "what was copied on {} could not be brought over: {why}",
                        row.created_at
                    ));
                    brought.refused += 1;
                }
            }
            Ok(())
        })?;
        Ok(brought)
    }

    fn is_from_elsewhere(&self) -> bool {
        self.opened
            .taken
            .platform
            .as_deref()
            .is_some_and(|made| made != WHERE_IT_WAS_MADE)
    }

    fn each(&self, work: &mut impl FnMut(&crate::Listed) -> Result<()>) -> Result<()> {
        let mut after = None;
        loop {
            let page = self.source.list(&everything(), Store::PAGE, after)?;
            if page.rows.is_empty() {
                break;
            }
            for row in &page.rows {
                work(row)?;
            }
            after = page.next;
            if after.is_none() {
                break;
            }
        }
        Ok(())
    }
}

pub fn bring(from: &Path, into: &Store, at: i64, say: &dyn Fn(&str)) -> Result<Brought> {
    Prepared::open(from)?.bring(into, at, say)
}

enum Carried {
    Added { partial: bool },
    Already { repaired: bool },
    Unreadable,
}

fn carried_item(
    source: &Store,
    row: &crate::Listed,
) -> Result<Option<(cp_core::item::Item, String, bool)>> {
    let Some(mut item) = source.item(row.id)? else {
        return Ok(None);
    };
    let preview = source.full_preview(row.id, &item)?;
    if item.formats.is_empty() && !preview.is_empty() {
        item.formats = cp_core::item::Item::plain(&preview).formats;
        item.formats[0].payload = cp_core::item::Payload::stored(preview.as_bytes().to_vec());
    }
    if !item.is_comparable() {
        return Ok(None);
    }
    let partial = source.raw().query_row(
        "SELECT COUNT(*) FROM item_formats WHERE item_id = ?1 AND digest IS NOT NULL",
        [row.id],
        |r| r.get::<_, i64>(0),
    )? > item
        .formats
        .iter()
        .filter(|f| matches!(f.payload, cp_core::item::Payload::Blob(_)))
        .count() as i64;
    Ok(Some((item, preview, partial)))
}

fn carry(source: &Store, into: &Store, row: &crate::Listed, at: i64) -> Result<Carried> {
    let Some((item, preview, partial)) = carried_item(source, row)? else {
        return Ok(Carried::Unreadable);
    };
    if let Some(id) = into.find_by_hash(&item)? {
        let repaired = match into.item(id)? {
            Some(local) => into.repair_item_search(id, &local)?,
            None => false,
        };
        return Ok(Carried::Already { repaired });
    }
    let meta = source.all_meta(row.id)?;
    let nothing: &[&str] = &[];
    let more = crate::More {
        came_at: None,
        modified_at: Some(row.modified_at),
        touched_at: Some(at),
        used_at: row.last_used_at,
        app: row.app.as_deref(),
        label: row.label.as_deref(),
        color: row.color,
        pinned: row.pinned,
        pastes: row.paste_count,
        broken: row.broken_since,
        meta: &meta,
        jobs: row.kind.map_or(nothing, |kind| jobs_for(kind)),
    };
    let id = into.insert_full(
        &free_name(source, into, row.id, at),
        &item,
        &preview,
        row.created_at,
        &more,
    )?;
    if !row.group.is_empty() {
        into.set_group(id, &row.group)?;
    }
    if let Some(text) = source.ocr_text(row.id)? {
        into.set_ocr_text(id, &text, at)?;
    }
    Ok(Carried::Added { partial })
}

fn free_name(source: &Store, into: &Store, id: i64, at: i64) -> String {
    let said = named(source, id, at);
    if !taken(into, &said) {
        return said;
    }
    let mut turn = 1;
    loop {
        let again = format!("{said}-{turn}");
        if !taken(into, &again) {
            return again;
        }
        turn += 1;
    }
}

fn taken(into: &Store, uuid: &str) -> bool {
    into.raw()
        .query_row("SELECT 1 FROM items WHERE uuid = ?1", [uuid], |_| Ok(()))
        .is_ok()
}

fn named(source: &Store, id: i64, at: i64) -> String {
    source
        .raw()
        .query_row("SELECT uuid FROM items WHERE id = ?1", [id], |row| {
            row.get::<_, String>(0)
        })
        .unwrap_or_else(|_| format!("{at:x}-{id:08x}"))
}

struct Opened {
    dir: std::path::PathBuf,
    db: std::path::PathBuf,
    blobs: i64,
    taken: Taken,
}

impl Opened {
    fn of(from: &Path) -> Result<Self> {
        let taken = read(from)?;
        let dir = std::env::temp_dir().join(format!(
            "cp-backup-{}-{}-{:x}",
            std::process::id(),
            TURN.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |since| since.subsec_nanos())
        ));
        std::fs::create_dir_all(&dir).map_err(Error::Io)?;
        crate::store::restrict(&dir, 0o700)?;
        let mut opened = Self {
            db: dir.join("history.db"),
            blobs: 0,
            taken,
            dir,
        };
        std::fs::copy(from, &opened.db).map_err(Error::Io)?;
        opened.taken = read(&opened.db)?;
        let blobs = Blobs::at(&opened.dir.join("blobs"))?;
        let coming = Connection::open(&opened.db)?;
        let mut kept = 0;
        {
            let mut stmt = coming.prepare("SELECT digest, bytes FROM backup_blobs")?;
            let mut rows = stmt.query([])?;
            while let Some(row) = rows.next()? {
                let digest: String = row.get(0)?;
                let bytes: Vec<u8> = row.get(1)?;
                if blobs.put(&bytes)? == digest {
                    kept += 1;
                }
            }
        }
        coming.execute_batch(
            "DROP TABLE IF EXISTS backup_blobs;
             DROP TABLE IF EXISTS backup_note;",
        )?;
        drop(coming);
        opened.blobs = kept;
        Ok(opened)
    }
}

impl Drop for Opened {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

static TURN: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

fn everything() -> crate::Filter {
    crate::Filter {
        broken: crate::Broken::Shown,
        ..Default::default()
    }
}

fn lives_at(db: &Connection) -> Option<std::path::PathBuf> {
    db.query_row(
        "SELECT file FROM pragma_database_list WHERE name = 'main'",
        [],
        |row| row.get::<_, String>(0),
    )
    .ok()
    .filter(|said| !said.is_empty())
    .map(std::path::PathBuf::from)
}

fn same_file(one: &Path, other: &Path) -> bool {
    let settled = |path: &Path| {
        path.canonicalize().unwrap_or_else(|_| {
            path.parent()
                .and_then(|parent| parent.canonicalize().ok())
                .map_or_else(
                    || path.to_path_buf(),
                    |parent| match path.file_name() {
                        Some(name) => parent.join(name),
                        None => parent,
                    },
                )
        })
    };
    settled(one) == settled(other)
}

fn wanted(db: &Connection) -> Result<Vec<String>> {
    let mut stmt =
        db.prepare("SELECT DISTINCT digest FROM item_formats WHERE digest IS NOT NULL")?;
    let found = stmt
        .query_map([], |row| row.get::<_, String>(0))?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    Ok(found)
}

fn noted(db: &Connection, key: &str) -> Result<Option<i64>> {
    Ok(said(db, key)?.and_then(|said| said.parse().ok()))
}

fn said(db: &Connection, key: &str) -> Result<Option<String>> {
    let found: Option<String> = db
        .query_row(
            "SELECT value FROM backup_note WHERE key = ?1",
            [key],
            |row| row.get(0),
        )
        .or_else(|why| match why {
            rusqlite::Error::QueryReturnedNoRows => Ok(None),
            rusqlite::Error::SqliteFailure(_, Some(ref said)) if said.contains("no such table") => {
                Ok(None)
            }
            other => Err(other),
        })?;
    Ok(found)
}

fn sidecar(path: &Path, tail: &str) -> std::path::PathBuf {
    let mut said = path.as_os_str().to_os_string();
    said.push(tail);
    std::path::PathBuf::from(said)
}

fn remove(path: &Path) -> Result<()> {
    match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(why) if why.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(why) => Err(Error::Io(why)),
    }
}

fn weighed(path: &Path) -> u64 {
    std::fs::metadata(path).map_or(0, |it| it.len())
}

fn jobs_for(kind: cp_core::kind::Kind) -> &'static [&'static str] {
    match kind {
        cp_core::kind::Kind::Image => &["thumb", "ocr"],
        cp_core::kind::Kind::File | cp_core::kind::Kind::Folder
            if cp_core::thumbnail::THUMBNAILS_FILES =>
        {
            &["thumb"]
        }
        _ => &[],
    }
}

#[cfg(test)]
#[path = "backup_test.rs"]
mod tests;
