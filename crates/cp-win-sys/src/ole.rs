use windows::Win32::Foundation::HGLOBAL;
use windows::Win32::System::Com::{
    DVASPECT_CONTENT, FORMATETC, IDataObject, IStream, STGMEDIUM, STREAM_SEEK_SET, TYMED,
    TYMED_HGLOBAL, TYMED_ISTREAM,
};
use windows::Win32::System::Memory::{GlobalFlags, GlobalSize};
use windows::Win32::System::Ole::{
    OleGetClipboard, OleInitialize, OleUninitialize, ReleaseStgMedium,
};

use crate::clipboard::global_bytes;

const CHUNK: usize = 64 * 1024;
const GMEM_INVALID_HANDLE: u32 = 0x8000;

pub struct Ole {
    ours: bool,
}

impl Ole {
    pub fn enter() -> Self {
        let entered = unsafe { OleInitialize(None) };
        Self {
            ours: entered.is_ok(),
        }
    }
}

impl Drop for Ole {
    fn drop(&mut self) {
        if self.ours {
            unsafe { OleUninitialize() };
        }
    }
}

pub fn indexed_contents(format: u32, count: usize, up_to: usize) -> Vec<Option<Vec<u8>>> {
    let _ole = Ole::enter();
    let Ok(data) = (unsafe { OleGetClipboard() }) else {
        return vec![None; count];
    };
    contents_of(&data, format, count, up_to)
}

fn contents_of(
    data: &IDataObject,
    format: u32,
    count: usize,
    up_to: usize,
) -> Vec<Option<Vec<u8>>> {
    (0..count)
        .map(|index| {
            let indexed = i32::try_from(index).ok();
            indexed
                .and_then(|lindex| one_of(data, format, lindex, up_to))
                .or_else(|| {
                    (count == 1)
                        .then(|| one_of(data, format, -1, up_to))
                        .flatten()
                })
        })
        .collect()
}

pub(crate) fn one_of(
    data: &IDataObject,
    format: u32,
    lindex: i32,
    up_to: usize,
) -> Option<Vec<u8>> {
    let asked = FORMATETC {
        cfFormat: u16::try_from(format).ok()?,
        ptd: std::ptr::null_mut(),
        dwAspect: DVASPECT_CONTENT.0,
        lindex,
        tymed: (TYMED_HGLOBAL.0 | TYMED_ISTREAM.0) as u32,
    };

    let mut medium: STGMEDIUM = unsafe { data.GetData(&asked) }.ok()?;
    let bytes = match TYMED(medium.tymed as i32) {
        TYMED_HGLOBAL => {
            let global: HGLOBAL = unsafe { medium.u.hGlobal };
            global_contents(global, up_to)
        }
        TYMED_ISTREAM => {
            let stream: Option<IStream> = unsafe { (*medium.u.pstm).clone() };
            stream.and_then(|stream| stream_bytes(&stream, up_to))
        }
        _ => None,
    };

    unsafe { ReleaseStgMedium(&mut medium) };
    bytes
}

fn global_contents(global: HGLOBAL, up_to: usize) -> Option<Vec<u8>> {
    if global.is_invalid() || unsafe { GlobalFlags(global) } == GMEM_INVALID_HANDLE {
        return None;
    }
    if unsafe { GlobalSize(global) } == 0 {
        return Some(Vec::new());
    }
    global_bytes(global, up_to)
}

fn stream_bytes(stream: &IStream, up_to: usize) -> Option<Vec<u8>> {
    let _ = unsafe { stream.Seek(0, STREAM_SEEK_SET, None) };
    drained(
        |chunk| {
            let mut read = 0u32;

            let status =
                unsafe { stream.Read(chunk.as_mut_ptr().cast(), CHUNK as u32, Some(&mut read)) };
            (status.is_err(), read as usize)
        },
        up_to,
    )
}

fn drained(mut read: impl FnMut(&mut [u8]) -> (bool, usize), up_to: usize) -> Option<Vec<u8>> {
    let mut out = Vec::new();
    let mut chunk = vec![0u8; CHUNK];
    loop {
        let (failed, read) = read(&mut chunk);
        if failed && read == 0 {
            return None;
        }
        if read == 0 {
            break;
        }
        out.extend_from_slice(&chunk[..read]);
        if out.len() > up_to {
            return None;
        }
    }
    Some(out)
}

#[cfg(test)]
#[path = "ole_test.rs"]
mod tests;
