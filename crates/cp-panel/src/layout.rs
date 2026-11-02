use cp_core::kind::Kind;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Layout {
    Everything,
    Image,
}

impl Layout {
    pub fn as_str(self) -> &'static str {
        match self {
            Layout::Everything => "everything",
            Layout::Image => "image",
        }
    }
}

pub fn layout_for(kinds: &[Kind]) -> Layout {
    if !kinds.is_empty() && kinds.iter().all(|kind| *kind == Kind::Image) {
        return Layout::Image;
    }
    Layout::Everything
}

#[cfg(test)]
#[path = "layout_test.rs"]
mod tests;
