use super::{Store, head_of};
use crate::Result;
use cp_core::item::{Item, Payload, SYNTHETIC_TEXT};
use cp_core::search::fold;
use rusqlite::params;

impl Store {
    pub(crate) fn full_preview(&self, id: i64, item: &Item) -> Result<String> {
        if let Some(text) = text_of(item) {
            return Ok(text);
        }
        Ok(self.db.query_row(
            "SELECT preview_text FROM items WHERE id = ?1",
            [id],
            |row| row.get(0),
        )?)
    }

    pub fn repair_search(&self) -> Result<usize> {
        let ids = self.ids_where(
            "deleted_at IS NULL AND EXISTS (SELECT 1 FROM item_formats f WHERE f.item_id = items.id
             AND f.format IN ('CF_UNICODETEXT', 'text/plain', 'public.utf8-plain-text', 'public.utf16-plain-text'))",
            &[],
        )?;
        let mut repaired = 0;
        for id in ids {
            if let Some(item) = self.item(id)? {
                repaired += usize::from(self.repair_item_search(id, &item)?);
            }
        }
        Ok(repaired)
    }

    pub(crate) fn repair_item_search(&self, id: i64, item: &Item) -> Result<bool> {
        let Some(text) = text_of(item) else {
            return Ok(false);
        };
        Ok(self.db.execute(
            "UPDATE items SET preview_text = ?2, search_text = ?3
             WHERE id = ?1 AND deleted_at IS NULL AND content_hash = ?4
               AND (preview_text <> ?2 OR search_text <> ?3)",
            params![id, head_of(&text), fold(&text), item.fingerprint() as i64],
        )? > 0)
    }

    pub(crate) fn search_needs_repair(&self, id: i64, item: &Item) -> Result<bool> {
        let Some(text) = text_of(item) else {
            return Ok(false);
        };
        Ok(self.db.query_row(
            "SELECT preview_text <> ?2 OR search_text <> ?3 FROM items WHERE id = ?1",
            params![id, head_of(&text), fold(&text)],
            |row| row.get(0),
        )?)
    }
}

fn text_of(item: &Item) -> Option<String> {
    let bytes = |id| {
        item.format(id).and_then(|format| match &format.payload {
            Payload::Inline(bytes) | Payload::Blob(bytes) => Some(bytes.as_slice()),
            _ => None,
        })
    };
    if let Some(bytes) = bytes("CF_UNICODETEXT") {
        let units: Vec<u16> = bytes
            .as_chunks::<2>()
            .0
            .iter()
            .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
            .take_while(|unit| *unit != 0)
            .collect();
        if let Ok(text) = String::from_utf16(&units) {
            return Some(text);
        }
    }
    for id in [SYNTHETIC_TEXT, "public.utf8-plain-text"] {
        if let Some(bytes) = bytes(id)
            && let Ok(text) = std::str::from_utf8(bytes)
        {
            return Some(text.to_owned());
        }
    }
    let bytes = bytes("public.utf16-plain-text")?;
    if bytes.len() % 2 != 0 {
        return None;
    }
    let big = bytes.starts_with(&[0xfe, 0xff]);
    let units: Vec<u16> = bytes
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| {
            if big {
                u16::from_be_bytes([pair[0], pair[1]])
            } else {
                u16::from_le_bytes([pair[0], pair[1]])
            }
        })
        .collect();
    String::from_utf16(units.strip_prefix(&[0xfeff]).unwrap_or(&units)).ok()
}

#[cfg(test)]
#[path = "store_text_test.rs"]
mod tests;
