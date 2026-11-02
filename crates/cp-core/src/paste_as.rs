use crate::kind::Kind;
use std::borrow::Cow;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Content<'a> {
    pub kind: Option<Kind>,
    pub text: Option<Cow<'a, str>>,
    pub image: Option<&'a [u8]>,
    pub paths: Vec<String>,
    pub ocr: Option<&'a str>,
}

pub fn plainest(content: &Content) -> Option<String> {
    let read = content
        .ocr
        .map(str::trim)
        .filter(|ocr| !ocr.is_empty())
        .map(str::to_owned);
    if content.image.is_some() && read.is_some() {
        return read;
    }
    if !content.paths.is_empty() {
        return Some(content.paths.join("\n"));
    }
    if let Some(text) = content.text.as_deref().filter(|text| !text.is_empty()) {
        return Some(text.to_owned());
    }
    read
}

#[cfg(test)]
#[path = "paste_as_test.rs"]
mod tests;
