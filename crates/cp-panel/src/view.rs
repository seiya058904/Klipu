use crate::age::age_text;
use crate::{Card, Chip};
use cp_core::kind::Kind;
use cp_core::search::Excerpt;
use cp_store::{Facet, Listed};

pub type MetaOfOne = std::collections::HashMap<String, String>;

const LEAD: usize = 30;
const PER_LINE: usize = 57;
const SHUT: i32 = 2;
const OPEN_AT_MOST: i32 = 7;

pub fn lines_of(text: &str) -> i32 {
    let wanted = text
        .lines()
        .map(|line| line.chars().count().div_ceil(PER_LINE).max(1) as i32)
        .sum::<i32>();
    wanted.clamp(SHUT, OPEN_AT_MOST)
}

pub fn was_found(row: &Listed) -> bool {
    row.snippet
        .as_ref()
        .is_some_and(|snippet| snippet.excerpt.segments.iter().any(|one| one.matched))
}

pub fn parts_of(excerpt: &Excerpt) -> (String, String, String) {
    let at = excerpt.segments.iter().position(|one| one.matched);
    let Some(at) = at else {
        return (flat(&excerpt.plain()), String::new(), String::new());
    };
    let lead: String = excerpt.segments[..at]
        .iter()
        .map(|one| one.text.as_str())
        .collect();
    let tail: String = excerpt.segments[at + 1..]
        .iter()
        .map(|one| one.text.as_str())
        .collect();
    (
        trim_left(&flat(&lead)),
        flat(&excerpt.segments[at].text),
        flat(&tail),
    )
}

fn flat(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut spaced = false;
    for one in text.chars() {
        if one.is_whitespace() {
            if !spaced {
                out.push(' ');
                spaced = true;
            }
            continue;
        }
        out.push(one);
        spaced = false;
    }
    out
}

fn trim_left(lead: &str) -> String {
    let count = lead.chars().count();
    if count <= LEAD {
        return lead.to_owned();
    }
    let kept: String = lead.chars().skip(count - LEAD).collect();
    format!("…{}", kept.trim_start())
}

pub fn alert_of(row: &Listed) -> &'static str {
    if row.broken_since.is_some() {
        return crate::say::pick("No encontrado", "Not found");
    }
    ""
}

pub fn card_of(row: &Listed, now: i64, meta: Option<&MetaOfOne>, most: i32) -> Card {
    let kind = row.kind.map(Kind::as_str).unwrap_or("text");
    let (clock, measures) = crate::media::said_in(meta);
    let link = (row.kind == Some(Kind::Link))
        .then(|| crate::link::parts_of(&row.preview))
        .flatten();
    let folder = folder_of(row);
    let papers = (row.kind == Some(Kind::File))
        .then(|| crate::papers::papers_of(&row.preview))
        .flatten();
    let body = body_of(row);
    let (lead, hit, tail) = match &row.snippet {
        Some(snippet) => parts_of(&snippet.excerpt),
        None => (String::new(), String::new(), String::new()),
    };
    let headline = headline_of(row, &body, folder.as_ref());
    let knows = Knows {
        folder: folder.as_ref(),
        papers: papers.as_ref(),
        clock: &clock,
        measures: &measures,
    };
    let aside = aside_of(row, &knows, meta);
    let (headline, aside) = top_line(headline, aside, label_of(row.kind));
    let source = row.app.clone().unwrap_or_default();
    let times = if row.paste_count > 1 {
        row.paste_count.to_string()
    } else {
        String::new()
    };
    let face = face_for(row, row.thumb_path.is_some(), most);
    let (line_one, line_two) = crate::face::opening_of(&body);
    let mixed_aside = match row.kind {
        Some(Kind::Folder) => crate::folder::said_in(meta, crate::say::in_english()),
        _ => String::new(),
    };
    let lines_said = match row.kind {
        Some(Kind::Text | Kind::Code) | None => crate::face::lines_said(
            row.preview.lines().count(),
            face.lines(),
            crate::say::in_english(),
        ),
        _ => String::new(),
    };
    let aside_said =
        crate::face::aside_said(&[&mixed_aside, &source, &used_text(&times), &lines_said]);
    let opened = opened_for(row, face);
    Card {
        opened: opened.text.into(),
        open_lines: opened.lines,
        more_said: opened.more.into(),
        face: face.as_str().into(),
        shut_lines: face.lines(),
        line_one: line_one.into(),
        line_two: line_two.into(),
        aside_said: aside_said.into(),
        id: row.id as i32,
        kind: kind.into(),
        under: under_line(&aside, &source, &times).into(),
        under_named: under_line(&headline, &source, &times).into(),
        headline: headline.into(),
        name: row.label.clone().unwrap_or_default().into(),
        title: label_of(row.kind).into(),
        source: source.into(),
        times: times.clone().into(),
        age: age_text(now, touched_of(row)).into(),
        lines: lines_of(&body),
        shut_px: Default::default(),
        open_px: Default::default(),
        body_lines: lines_of(&body),
        squeezed: squeezed_of(&body).into(),
        media_measures: measures.into(),
        papers_format: papers
            .as_ref()
            .map(|one| one.format.clone())
            .unwrap_or_default()
            .into(),
        papers_family: papers.as_ref().map_or("plain", |one| one.family).into(),
        folder_name: folder
            .as_ref()
            .map(|one| one.name.clone())
            .unwrap_or_default()
            .into(),
        folder_parent: folder
            .as_ref()
            .map(|one| one.parent.clone())
            .unwrap_or_default()
            .into(),
        folder_said: crate::folder::said_in(meta, crate::say::in_english()).into(),
        link_domain: link
            .as_ref()
            .map(|one| one.domain.clone())
            .unwrap_or_default()
            .into(),
        heads_group: false,
        group_said: Default::default(),
        link_path: link
            .as_ref()
            .map(|one| one.path.clone())
            .unwrap_or_default()
            .into(),
        body: body.into(),
        found: !hit.is_empty(),
        alert: alert_of(row).into(),
        lead: lead.into(),
        hit: hit.into(),
        tail: tail.into(),
        mono: matches!(row.kind, Some(Kind::Json | Kind::Code | Kind::Token)),
        can_open: crate::opening::can_open(row.kind, &paths_of(row))
            || crate::opening::can_open_link(row.kind, &row.preview),
        can_drag: crate::dragging::can_drag(row.kind, &paths_of(row)),
        paint: shown_as_colour(row).unwrap_or_default(),
        paint_said: paint_said(row).into(),
        paints: shown_as_colour(row).is_some(),
        thumb: slint::Image::default(),
        has_thumb: row.thumb_path.is_some(),
        pinned: row.pinned,
        broken: row.broken_since.is_some(),
        blank: says_nothing(row),
    }
}

pub fn face_for(row: &Listed, thumb: bool, most: i32) -> crate::face::Face {
    crate::face::Face::of(&crate::face::Seen {
        kind: row.kind,
        found: was_found(row),
        thumb,
        paints: shown_as_colour(row).is_some(),
        body: &body_of(row),
        most,
    })
}

pub fn opened_for(row: &Listed, face: crate::face::Face) -> crate::face::Opened {
    crate::face::opened_of(face, &body_of(row), crate::say::in_english())
}

pub fn mixed_open_px(row: &Listed, thumb: bool, room: f32, most: i32) -> f32 {
    let face = face_for(row, thumb, most);
    let opened = opened_for(row, face);
    let body = crate::face::open_body_px(face, &opened, room);
    if row.broken_since.is_some() {
        crate::face::open_lost_px(body)
    } else {
        crate::face::open_px(body)
    }
}

pub fn folder_of(row: &Listed) -> Option<crate::folder::Parts> {
    matches!(row.kind, Some(Kind::Folder | Kind::File))
        .then(|| crate::folder::parts_of(&row.preview))
        .flatten()
}

pub fn headline_of(row: &Listed, body: &str, folder: Option<&crate::folder::Parts>) -> String {
    match row.kind {
        Some(Kind::File | Kind::Folder) => match folder {
            Some(parts) => parts.name.clone(),
            None => squeezed_of(body),
        },
        _ => squeezed_of(body),
    }
}

pub fn touched_of(row: &Listed) -> i64 {
    row.last_used_at.unwrap_or(0).max(row.modified_at)
}

pub fn under_line(said: &str, source: &str, times: &str) -> String {
    [said.trim(), source.trim(), &used_text(times)]
        .into_iter()
        .filter(|one| !one.is_empty())
        .collect::<Vec<_>>()
        .join(" · ")
}

fn used_text(times: &str) -> String {
    if times.trim().is_empty() {
        return String::new();
    }
    format!("×{}", times.trim())
}

pub fn top_line(headline: String, aside: String, label: &str) -> (String, String) {
    if !headline.trim().is_empty() {
        return (headline, aside);
    }
    if !aside.trim().is_empty() {
        return (aside, String::new());
    }
    (label.to_owned(), String::new())
}

pub struct Knows<'a> {
    pub folder: Option<&'a crate::folder::Parts>,
    pub papers: Option<&'a crate::papers::Papers>,
    pub clock: &'a str,
    pub measures: &'a str,
}

pub fn aside_of(row: &Listed, knows: &Knows<'_>, meta: Option<&MetaOfOne>) -> String {
    let Knows {
        folder,
        papers,
        clock,
        measures,
    } = knows;
    let joined = |parts: &[&str]| -> String {
        parts
            .iter()
            .map(|one| one.trim())
            .filter(|one| !one.is_empty())
            .collect::<Vec<_>>()
            .join(" · ")
    };
    match row.kind {
        Some(Kind::Video | Kind::Audio) => joined(&[clock, measures]),
        Some(Kind::Image) => joined(&[measures]),
        Some(Kind::File) => {
            let format = papers.map(|one| one.format.as_str()).unwrap_or_default();
            let parent = folder.map(|one| one.parent.as_str()).unwrap_or_default();
            joined(&[format, parent.trim_end_matches(['\\', '/'])])
        }
        Some(Kind::Folder) => {
            let parent = folder.map(|one| one.parent.as_str()).unwrap_or_default();
            joined(&[
                parent.trim_end_matches(['\\', '/']),
                &crate::folder::said_in(meta, crate::say::in_english()),
            ])
        }
        Some(Kind::Text | Kind::Code) => more_than_shown(row),
        _ => String::new(),
    }
}

fn more_than_shown(row: &Listed) -> String {
    let lines = row.preview.lines().count();
    if lines <= 1 || says_nothing(row) {
        return String::new();
    }
    let rest = lines - 1;
    if crate::say::in_english() {
        format!("+{rest} more lines")
    } else {
        format!("+{rest} líneas más")
    }
}

pub const NAME_ROOM: usize = 80;

pub fn name_worth_keeping(said: &str) -> Option<String> {
    use unicode_segmentation::UnicodeSegmentation;
    let whole = said.split_whitespace().collect::<Vec<_>>().join(" ");
    let name: String = whole.graphemes(true).take(NAME_ROOM).collect();
    (!name.is_empty()).then_some(name)
}

pub fn paths_of(row: &Listed) -> Vec<String> {
    row.preview
        .lines()
        .next()
        .map(|one| vec![one.to_owned()])
        .unwrap_or_default()
}

pub fn shown_as_colour(row: &Listed) -> Option<slint::Color> {
    if row.kind != Some(Kind::Color) {
        return None;
    }
    let said = cp_core::paint::rgba_of(row.preview.lines().next()?)?;
    Some(slint::Color::from_argb_u8(
        said.alpha, said.red, said.green, said.blue,
    ))
}

pub fn paint_said(row: &Listed) -> String {
    let Some(said) = row
        .kind
        .filter(|kind| *kind == Kind::Color)
        .and_then(|_| cp_core::paint::rgba_of(row.preview.lines().next()?))
    else {
        return String::new();
    };
    if said.alpha == 255 {
        format!("rgb({}, {}, {})", said.red, said.green, said.blue)
    } else {
        format!(
            "rgba({}, {}, {}, {:.2})",
            said.red,
            said.green,
            said.blue,
            f32::from(said.alpha) / 255.0
        )
    }
}

pub fn squeezed_of(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut spaced = true;
    for one in text.chars() {
        if one.is_whitespace() {
            if !spaced {
                out.push(' ');
                spaced = true;
            }
            continue;
        }
        out.push(one);
        spaced = false;
    }
    if out.ends_with(' ') {
        out.pop();
    }
    out
}

pub fn body_of(row: &Listed) -> String {
    match &row.snippet {
        Some(snippet) => snippet.excerpt.plain(),
        None if says_nothing(row) => nothing_visible_in(crate::say::in_english()).to_owned(),
        None => row.preview.clone(),
    }
}

pub fn nothing_visible_in(english: bool) -> &'static str {
    crate::say::pick_in(english, "Sin texto visible", "No visible text")
}

fn invisible(one: char) -> bool {
    matches!(
        one,
        '\u{00AD}'
            | '\u{200B}'..='\u{200F}'
            | '\u{202A}'..='\u{202E}'
            | '\u{2060}'..='\u{2064}'
            | '\u{2066}'..='\u{2069}'
            | '\u{FEFF}'
    )
}

pub fn has_visible_text(text: &str) -> bool {
    text.chars()
        .any(|one| !one.is_whitespace() && !one.is_control() && !invisible(one))
}

pub fn says_nothing(row: &Listed) -> bool {
    row.snippet.is_none()
        && !matches!(
            row.kind,
            Some(Kind::Image | Kind::File | Kind::Folder | Kind::Audio | Kind::Video)
        )
        && row.preview.chars().count() < cp_store::PREVIEW_CHARS
        && !has_visible_text(&row.preview)
}

pub fn label_of(kind: Option<Kind>) -> &'static str {
    let (es, en) = label_pair(kind);
    crate::say::pick(es, en)
}

fn label_pair(kind: Option<Kind>) -> (&'static str, &'static str) {
    match kind {
        Some(Kind::Text) | None => ("Texto", "Text"),
        Some(Kind::Code) => ("Código", "Code"),
        Some(Kind::Json) => ("JSON", "JSON"),
        Some(Kind::Link) => ("Enlace", "Link"),
        Some(Kind::Email) => ("Correo", "Email"),
        Some(Kind::Phone) => ("Teléfono", "Phone"),
        Some(Kind::Color) => ("Color", "Colour"),
        Some(Kind::Ip) => ("IP", "IP"),
        Some(Kind::Uuid) => ("UUID", "UUID"),
        Some(Kind::Image) => ("Imagen", "Image"),
        Some(Kind::File) => ("Archivo", "File"),
        Some(Kind::Folder) => ("Carpeta", "Folder"),
        Some(Kind::Audio) => ("Audio", "Audio"),
        Some(Kind::Video) => ("Video", "Video"),
        Some(Kind::Token) => ("Token", "Token"),
    }
}

pub const GROUPS: [&str; 4] = ["text", "image", "file", "link"];

pub fn kinds_of_group(group: &str) -> &'static [Kind] {
    match group {
        "text" => &[
            Kind::Text,
            Kind::Code,
            Kind::Json,
            Kind::Email,
            Kind::Phone,
            Kind::Color,
            Kind::Ip,
            Kind::Uuid,
            Kind::Token,
        ],
        "image" => &[Kind::Image],
        "file" => &[Kind::File, Kind::Folder, Kind::Audio, Kind::Video],
        "link" => &[Kind::Link],
        _ => &[],
    }
}

pub fn label_of_group(group: &str) -> &'static str {
    let kind = match group {
        "image" => Kind::Image,
        "file" => Kind::File,
        "link" => Kind::Link,
        _ => Kind::Text,
    };
    label_of(Some(kind))
}

pub fn group_from_word(word: &str) -> Option<&'static str> {
    match word.to_lowercase().as_str() {
        "文本" | "text" => Some("text"),
        "图片" | "image" => Some("image"),
        "文件" | "file" => Some("file"),
        "链接" | "link" => Some("link"),
        _ => None,
    }
}

pub fn harvest(query: &str) -> (Vec<String>, String) {
    let still_typing = !query.ends_with(char::is_whitespace);
    let tokens: Vec<&str> = query.split_whitespace().collect();
    let mut taken = Vec::new();
    let mut kept = Vec::new();
    for (at, token) in tokens.iter().enumerate() {
        let last = at + 1 == tokens.len();
        match token.strip_prefix('#').and_then(group_from_word) {
            Some(group) if !(last && still_typing) => taken.push(group.to_owned()),
            _ => kept.push(*token),
        }
    }
    let mut rest = kept.join(" ");
    if !rest.is_empty() && !still_typing {
        rest.push(' ');
    }
    (taken, rest)
}

pub fn chips_of(facets: &[Facet], selected: &[String]) -> Vec<Chip> {
    GROUPS
        .into_iter()
        .map(|group| {
            let kinds = kinds_of_group(group);
            let count: i64 = facets
                .iter()
                .filter(|facet| kinds.contains(&facet.kind))
                .map(|facet| facet.count)
                .sum();
            Chip {
                key: group.into(),
                label: label_of_group(group).into(),
                count: compact(count).into(),
                selected: selected.iter().any(|one| one == group),
            }
        })
        .collect()
}

pub fn compact(count: i64) -> String {
    if count >= 10_000 {
        format!("{}k", count / 1_000)
    } else if count >= 1_000 {
        format!("{:.1}k", count as f64 / 1_000.0)
    } else {
        count.to_string()
    }
}

const SHOWN_QUERY: usize = 24;

pub fn empty_of(query: &str, pinned: bool, kinds: bool) -> (String, String) {
    empty_in(crate::say::in_english(), query, pinned, kinds)
}

fn empty_in(english: bool, query: &str, pinned: bool, kinds: bool) -> (String, String) {
    let say = |es: &'static str, en: &'static str| crate::say::pick_in(english, es, en).to_owned();
    let query = query.trim();
    if query.is_empty() {
        return match (pinned, kinds) {
            (true, _) => (
                say("No hay nada anclado", "Nothing is pinned"),
                say(
                    "Ancla lo que uses seguido y se queda a mano",
                    "Pin what you use often and it stays within reach",
                ),
            ),
            (false, true) => (
                say("No hay nada de este tipo", "Nothing of this kind"),
                say(
                    "Quita el filtro para ver todo",
                    "Drop the filter to see everything",
                ),
            ),
            (false, false) => (
                say("Todavía no hay nada", "Nothing here yet"),
                say("Lo que copies aparece aquí", "What you copy shows up here"),
            ),
        };
    }
    let shown = shorten(query);
    let asked = crate::say::pick_in(english, "Nada coincide con «{}»", "Nothing matches “{}”")
        .replace("{}", &shown);
    if !query.chars().any(char::is_alphanumeric) {
        return (
            asked,
            say(
                "La búsqueda va por palabras: los signos solos no se buscan",
                "Search goes by words: punctuation alone is not searched",
            ),
        );
    }
    if pinned || kinds {
        (
            asked,
            say(
                "Prueba con menos letras o quita el filtro",
                "Try fewer letters, or drop the filter",
            ),
        )
    } else {
        (asked, say("Prueba con menos letras", "Try fewer letters"))
    }
}

fn shorten(query: &str) -> String {
    if query.chars().count() <= SHOWN_QUERY {
        return query.to_owned();
    }
    let kept: String = query.chars().take(SHOWN_QUERY).collect();
    format!("{}…", kept.trim_end())
}

pub fn count_text(count: i64) -> String {
    count_in(crate::say::in_english(), count)
}

fn count_in(english: bool, count: i64) -> String {
    match (count, english) {
        (0, false) => "sin elementos".into(),
        (0, true) => "nothing kept".into(),
        (1, false) => "1 elemento".into(),
        (1, true) => "1 item".into(),
        (n, false) => format!("{n} elementos"),
        (n, true) => format!("{n} items"),
    }
}

const FOOTER_ES: &str = "pegar · shift+enter: en plano";
const FOOTER_EN: &str = "paste · shift+enter: plain text";

pub fn dress_words(ui: &crate::Panel) {
    use slint::ComponentHandle;
    let words = ui.global::<crate::Words>();
    words.set_hint(crate::say::pick("Busca o filtra con #", "Search, or filter with #").into());
    words.set_footer(crate::say::pick(FOOTER_ES, FOOTER_EN).into());
    words.set_paste_it(crate::say::pick("Pegar", "Paste").into());
    words.set_paste_plain(crate::say::pick("Plano", "Plain").into());
    words.set_open_it(crate::say::pick("abrir", "open").into());
    words.set_name_it(crate::say::pick("poner nombre", "give it a name").into());
    words.set_name_room(i32::try_from(NAME_ROOM).unwrap_or(i32::MAX));
    words.set_copy_it(crate::say::pick("copiar sin pegar", "copy without pasting").into());
    words.set_pin(crate::say::pick("anclar", "pin").into());
    words.set_unpin(crate::say::pick("desanclar", "unpin").into());
    words.set_remove(crate::say::pick("borrar", "delete").into());
}

#[cfg(test)]
#[path = "view_test.rs"]
mod tests;
