use super::*;

use cp_core::search::{Excerpt, Segment};
use cp_store::{FoundIn, Snippet};

fn row(kind: Option<Kind>) -> Listed {
    Listed {
        id: 7,
        modified_at: 1_000_000,
        created_at: 1_000_000,
        kind,
        preview: "hola mundo".into(),
        app: Some("Mail".into()),
        label: None,
        color: 0,
        thumb_path: None,
        paste_count: 3,
        last_used_at: None,
        broken_since: None,
        pinned: true,
        group: String::new(),
        snippet: None,
    }
}

#[test]
fn a_row_becomes_the_card_the_panel_draws() {
    let card = card_of(&row(Some(Kind::Json)), 1_000_000 + 9 * 60_000, None, 2);
    assert_eq!(card.id, 7);
    assert_eq!(card.kind.as_str(), "json");
    assert_eq!(card.title.as_str(), "JSON");
    assert_eq!(card.source.as_str(), "Mail");
    assert_eq!(card.times.as_str(), "3");
    assert_eq!(card.age.as_str(), "9 分钟");
    assert_eq!(card.body.as_str(), "hola mundo");
    assert!(card.mono);
    assert!(!card.has_thumb);
    assert!(card.pinned);
}

#[test]
fn a_search_hit_shows_its_excerpt_instead_of_the_preview() {
    let mut hit = row(Some(Kind::Text));
    hit.snippet = Some(Snippet {
        found_in: FoundIn::Text,
        excerpt: Excerpt {
            segments: vec![
                Segment {
                    text: "…la ".into(),
                    matched: false,
                },
                Segment {
                    text: "reunión".into(),
                    matched: true,
                },
                Segment {
                    text: " del jueves…".into(),
                    matched: false,
                },
            ],
        },
    });
    let card = card_of(&hit, 2_000_000, None, 2);
    assert_eq!(card.body.as_str(), "…la reunión del jueves…");
    assert!(!card.mono);
    assert_eq!(card.times.as_str(), "3");
}

#[test]
fn what_was_pasted_once_or_never_carries_no_count() {
    let mut once = row(None);
    once.paste_count = 1;
    once.app = None;
    let card = card_of(&once, 1_000_000, None, 2);
    assert_eq!(card.times.as_str(), "");
    assert_eq!(
        card.source.as_str(),
        "",
        "a card from nowhere says nothing, because a dash beside a separator reads as noise"
    );
    assert_eq!(card.under.as_str(), "");
    assert_eq!(card.title.as_str(), "文本");
}

fn excerpt_of(parts: &[(&str, bool)]) -> Excerpt {
    Excerpt {
        segments: parts
            .iter()
            .map(|(text, matched)| Segment {
                text: (*text).into(),
                matched: *matched,
            })
            .collect(),
    }
}

#[test]
fn the_excerpt_breaks_into_what_goes_before_the_hit_and_what_follows() {
    let excerpt = excerpt_of(&[
        ("confirmamos la ", false),
        ("reunión", true),
        (" del jueves a las 16:30", false),
    ]);
    assert_eq!(
        parts_of(&excerpt),
        (
            "confirmamos la ".into(),
            "reunión".into(),
            " del jueves a las 16:30".into()
        )
    );
}

#[test]
fn an_excerpt_with_nothing_matched_is_all_text_and_no_hit() {
    let (lead, hit, tail) = parts_of(&excerpt_of(&[("sin coincidencias", false)]));
    assert_eq!(lead, "sin coincidencias");
    assert!(hit.is_empty(), "with no hit there is nothing to highlight");
    assert!(tail.is_empty());
}

#[test]
fn a_long_lead_is_cut_from_the_left_so_the_hit_does_not_fall_off() {
    let long = "una entrada muy larga que empuja el término lejos del principio ";
    let (lead, hit, _) = parts_of(&excerpt_of(&[(long, false), ("término", true)]));
    assert!(lead.starts_with('…'), "it is cut from the left: {lead}");
    assert!(lead.chars().count() <= LEAD + 1);
    assert!(lead.ends_with("lejos del principio "));
    assert_eq!(hit, "término");
}

#[test]
fn a_row_that_matched_carries_its_three_parts() {
    let mut row = row(Some(Kind::Text));
    row.snippet = Some(Snippet {
        found_in: FoundIn::Text,
        excerpt: excerpt_of(&[("antes de ", false), ("esto", true), (" y después", false)]),
    });
    let card = card_of(&row, 1_000_000, None, 2);
    assert!(card.found);
    assert_eq!(card.lead.as_str(), "antes de ");
    assert_eq!(card.hit.as_str(), "esto");
    assert_eq!(card.tail.as_str(), " y después");
}

#[test]
fn a_card_asks_for_as_many_lines_as_its_text_needs() {
    assert_eq!(
        lines_of("#FF8800"),
        2,
        "what is short does not grow when it opens"
    );
    assert_eq!(lines_of(""), 2);
    assert_eq!(lines_of(&"a".repeat(57)), 2);
    assert_eq!(
        lines_of(&"a".repeat(58)),
        2,
        "two lines are still the floor"
    );
    assert_eq!(lines_of(&"a".repeat(57 * 3)), 3);
    assert_eq!(lines_of(&"a".repeat(57 * 20)), 7, "and there is a ceiling");
    assert_eq!(
        lines_of(
            "uno
dos
tres
cuatro"
        ),
        4,
        "los saltos cuentan"
    );
}

#[test]
fn an_empty_list_explains_itself_in_both_tongues() {
    for english in [false, true] {
        let (title, hint) = empty_in(english, "", false, false);
        assert!(!title.is_empty() && !hint.is_empty());
        let (asked, _) = empty_in(english, "perdido", false, false);
        assert!(asked.contains("perdido"), "{asked}");
    }
    assert_ne!(
        empty_in(false, "", true, false).0,
        empty_in(true, "", true, false).0
    );
}

#[test]
fn the_count_reads_the_way_each_tongue_counts() {
    assert_eq!(count_in(false, 0), "暂无内容");
    assert_eq!(count_in(false, 1), "1 项");
    assert_eq!(count_in(false, 7), "7 项");
    assert_eq!(count_in(true, 0), "nothing kept");
    assert_eq!(count_in(true, 1), "1 item");
    assert_eq!(count_in(true, 7), "7 items");
}

#[test]
fn an_empty_list_says_why_it_is_empty() {
    assert_eq!(
        empty_of("", false, false),
        ("这里还是空的".into(), "复制的内容会出现在这里".into())
    );
    assert_eq!(empty_of("  ", true, false).0, "还没有置顶内容");
    assert_eq!(empty_of("", false, true).0, "没有这一类内容");

    let (title, hint) = empty_of(",", false, false);
    assert_eq!(title, "没有找到“,”");
    assert!(hint.contains("标点"), "a comma is not a word");

    let (title, hint) = empty_of("reunion", false, true);
    assert_eq!(title, "没有找到“reunion”");
    assert!(hint.contains("移除筛选"));

    assert_eq!(empty_of("reunion", false, false).1, "换个更短的词试试");
    assert_eq!(
        empty_of(
            "una consulta larguísima que no cabe en la tarjeta",
            false,
            false
        )
        .0,
        "没有找到“una consulta larguísima…”"
    );
}

#[test]
fn big_counts_are_shortened_so_the_pill_stays_a_pill() {
    assert_eq!(compact(999), "999");
    assert_eq!(compact(1_250), "1.2k");
    assert_eq!(compact(50_000), "50k");
    assert_eq!(count_text(0), "暂无内容");
    assert_eq!(count_text(1), "1 项");
    assert_eq!(count_text(50_000), "50000 项");
}

#[test]
fn a_closed_card_shows_content_where_the_indentation_was() {
    let pretty = "{\n  \"annotations\": [\n    {\n      \"id\": 1,\n      \"note\": \"revisar\"\n    }\n  ]\n}";
    let squeezed = squeezed_of(pretty);
    assert!(
        squeezed.starts_with("{ \"annotations\": [ { \"id\": 1, \"note\": \"revisar\" }"),
        "«{squeezed}»"
    );
    assert!(!squeezed.contains('\n'));
    assert!(!squeezed.contains("  "), "no run of spaces survives");
}

#[test]
fn squeezing_leaves_a_single_line_alone() {
    assert_eq!(
        squeezed_of("ya viene en una linea"),
        "ya viene en una linea"
    );
    assert_eq!(squeezed_of(""), "");
    assert_eq!(squeezed_of("   \n\t  "), "");
}

#[test]
fn squeezing_keeps_the_words_and_their_order() {
    let said = "primero\n\n\tsegundo   tercero\r\ncuarto ";
    assert_eq!(squeezed_of(said), "primero segundo tercero cuarto");
}

#[test]
fn a_colour_card_carries_the_colour_it_is() {
    let mut said = row(Some(Kind::Color));
    said.preview = "#FF8800".into();
    let card = card_of(&said, 1_000_000, None, 2);
    assert!(card.paints, "the only type whose content is its appearance");
    assert_eq!(card.paint.red(), 0xFF);
    assert_eq!(card.paint.green(), 0x88);
    assert_eq!(card.paint.blue(), 0x00);
    assert_eq!(card.paint.alpha(), 0xFF);
}

#[test]
fn nothing_else_pretends_to_be_a_colour() {
    for kind in [Kind::Text, Kind::Json, Kind::Link, Kind::File, Kind::Token] {
        let mut said = row(Some(kind));
        said.preview = "#FF8800".into();
        assert!(
            !card_of(&said, 1_000_000, None, 2).paints,
            "{kind:?} holds text that looks like a colour, and is not one"
        );
    }
    let mut unreadable = row(Some(Kind::Color));
    unreadable.preview = "not a colour at all".into();
    assert!(
        !card_of(&unreadable, 1_000_000, None, 2).paints,
        "and a colour nobody can parse draws no swatch"
    );
}

#[test]
fn a_file_card_knows_its_name_apart_from_its_folder() {
    let mut said = row(Some(Kind::File));
    said.preview = r"D:\Mario\Downloads\b774f629-9376-419f-9a7b-d390e7f775c2.pdf".into();
    let card = card_of(&said, 1_000_000, None, 2);
    assert_eq!(
        card.folder_name.as_str(),
        "b774f629-9376-419f-9a7b-d390e7f775c2.pdf",
        "the name is what says which file it is, so it cannot be the part that gets cut"
    );
    assert_eq!(card.folder_parent.as_str(), r"D:\Mario\Downloads\");
    assert_eq!(card.papers_format.as_str(), "PDF");
}

#[test]
fn a_file_with_no_extension_still_says_which_file_it_is() {
    let mut said = row(Some(Kind::File));
    said.preview = r"D:\Mario\LICENSE".into();
    let card = card_of(&said, 1_000_000, None, 2);
    assert_eq!(card.folder_name.as_str(), "LICENSE");
    assert_eq!(
        card.papers_format.as_str(),
        "",
        "there is no format to show, so the card draws a glyph instead of an empty box"
    );
}

#[test]
fn the_second_line_says_what_each_type_already_knows() {
    let mut json = row(Some(Kind::Json));
    json.preview = r#"{"a": 1, "b": {"c": 2}}"#.into();
    let said = card_of(&json, 1_000_000, None, 2).under.to_string();
    assert!(
        !said.is_empty(),
        "JSON knows its keys and its depth: «{said}»"
    );

    let mut file = row(Some(Kind::File));
    file.preview = r"D:\Mario\Downloads\one.yml".into();
    let said = card_of(&file, 1_000_000, None, 2).under.to_string();
    assert!(said.contains("YML"), "«{said}»");
    assert!(said.contains("Downloads"), "«{said}»");
    assert!(
        !said.ends_with('\\') && !said.ends_with('/'),
        "the trailing separator is noise: «{said}»"
    );

    let mut folder = row(Some(Kind::Folder));
    folder.preview = r"D:\Mario\Downloads".into();
    let said = card_of(&folder, 1_000_000, None, 2).under.to_string();
    assert!(said.contains("Mario"), "the folder it lives in: «{said}»");
}

#[test]
fn the_first_line_of_a_file_is_its_name_not_its_path() {
    let mut said = row(Some(Kind::File));
    said.preview = r"D:\Mario\Downloads\a-very-long-name.yml".into();
    assert_eq!(
        card_of(&said, 1_000_000, None, 2).headline.as_str(),
        "a-very-long-name.yml"
    );
    let mut folder = row(Some(Kind::Folder));
    folder.preview = r"D:\Mario\Downloads".into();
    assert_eq!(
        card_of(&folder, 1_000_000, None, 2).headline.as_str(),
        "Downloads"
    );
}

#[test]
fn a_text_of_one_line_says_nothing_more_and_a_long_one_says_how_much_more() {
    let mut one = row(Some(Kind::Text));
    one.preview = "just one line".into();
    assert_eq!(
        card_of(&one, 1_000_000, None, 2).under.as_str(),
        "Mail · ×3",
        "there is nothing the card is hiding, so only where it came from and how often it was used"
    );
    let mut many = row(Some(Kind::Text));
    many.preview = "first\nsecond\nthird".into();
    let said = card_of(&many, 1_000_000, None, 2).under.to_string();
    assert!(said.contains('2'), "two more lines are waiting: «{said}»");
}

#[test]
fn what_needs_no_second_line_gets_none() {
    for kind in [Kind::Uuid, Kind::Ip, Kind::Email, Kind::Phone, Kind::Color] {
        let mut said = row(Some(kind));
        said.preview = "something".into();
        assert_eq!(
            card_of(&said, 1_000_000, None, 2).under.as_str(),
            "Mail · ×3",
            "{kind:?}: the glyph already says what it is, so only the app and the count are left"
        );
    }
}

#[test]
fn a_name_of_only_spaces_clears_the_name() {
    assert_eq!(super::name_worth_keeping("   	  "), None);
    assert_eq!(super::name_worth_keeping(""), None);
}

#[test]
fn a_name_is_one_line_however_it_was_pasted() {
    assert_eq!(
        super::name_worth_keeping(
            "  claves   de
  produccion 
"
        ),
        Some("claves de produccion".to_owned()),
        "a label reads on one line, so the line breaks of a paste do not survive it"
    );
}

#[test]
fn a_name_longer_than_the_row_is_cut_to_the_room_there_is() {
    let said = "a".repeat(super::NAME_ROOM + 40);
    let kept = super::name_worth_keeping(&said).expect("a name");
    assert_eq!(kept.chars().count(), super::NAME_ROOM);
}

#[test]
fn a_name_is_cut_by_letters_and_not_by_bytes() {
    let said = "ñ".repeat(super::NAME_ROOM + 5);
    let kept = super::name_worth_keeping(&said).expect("a name");
    assert_eq!(
        kept.chars().count(),
        super::NAME_ROOM,
        "cutting a two byte letter in half would not even be text"
    );
}

#[test]
fn a_card_with_nothing_to_quote_puts_what_it_knows_on_the_first_line() {
    let (first, second) = super::top_line(String::new(), "476×576".to_owned(), "图片");
    assert_eq!(first, "476×576");
    assert_eq!(
        second, "",
        "una captura no tiene texto que citar, y una primera linea vacia se lee como una tarjeta rota"
    );
}

#[test]
fn a_card_with_something_to_quote_keeps_both_lines_where_they_were() {
    let (first, second) =
        super::top_line("git add -A".to_owned(), "+3 líneas más".to_owned(), "文本");
    assert_eq!(first, "git add -A");
    assert_eq!(second, "+3 líneas más");
}

#[test]
fn a_first_line_of_only_spaces_counts_as_empty() {
    let (first, second) = super::top_line(
        "   
 "
        .to_owned(),
        "2 claves".to_owned(),
        "JSON",
    );
    assert_eq!(first, "2 claves");
    assert_eq!(second, "");
}

#[test]
fn the_second_line_keeps_the_app_and_what_the_card_says() {
    assert_eq!(super::under_line("476×576", "Orca", ""), "476×576 · Orca");
}

#[test]
fn a_card_from_nowhere_does_not_drag_a_dangling_separator() {
    assert_eq!(
        super::under_line("476×576", "", ""),
        "476×576",
        "a card with no application behind it used to read «476×576 · —», and a dash is not a name"
    );
    assert_eq!(super::under_line("", "Orca", ""), "Orca");
    assert_eq!(super::under_line("  ", "  ", ""), "");
}

#[test]
fn a_named_card_still_says_what_it_holds() {
    let under = super::under_line("cargo build --workspace", "Orca", "");
    assert!(
        under.starts_with("cargo build --workspace"),
        "naming a card must not hide what is in it: {under}"
    );
}

#[test]
fn the_age_a_card_shows_is_the_one_the_list_is_sorted_by() {
    let mut said = row(Some(Kind::Image));
    said.modified_at = 1_000;
    said.last_used_at = Some(500_000);
    assert_eq!(
        super::touched_of(&said),
        500_000,
        "the list orders by MAX(modified_at, last_used_at), so a card that shows only modified_at \
         sits above one that reads newer and the order looks made up"
    );

    said.last_used_at = None;
    assert_eq!(super::touched_of(&said), 1_000);

    said.last_used_at = Some(1);
    assert_eq!(
        super::touched_of(&said),
        1_000,
        "pasting something long after copying it must not make it look older"
    );
}

#[test]
fn what_can_be_opened_is_a_path_or_a_link_and_not_everything_else() {
    let mut file = row(Some(Kind::File));
    file.preview = r"D:\Mario\uno.yml".into();
    assert!(card_of(&file, 1_000_000, None, 2).can_open);

    let mut link = row(Some(Kind::Link));
    link.preview = "https://ejemplo.cl/a".into();
    assert!(card_of(&link, 1_000_000, None, 2).can_open);

    let mut plain = row(Some(Kind::Text));
    plain.preview = "nada que abrir".into();
    assert!(
        !card_of(&plain, 1_000_000, None, 2).can_open,
        "a button that cannot do anything is worse than no button"
    );
}

#[test]
fn each_type_puts_its_own_fact_on_the_second_line() {
    let mut video = row(Some(Kind::Video));
    video.preview = "clip.mp4".into();
    let meta = [(crate::media::DURATION.to_owned(), "90000".to_owned())]
        .into_iter()
        .collect::<super::MetaOfOne>();
    let said = card_of(&video, 1_000_000, Some(&meta), 2).under.to_string();
    assert!(said.contains("1:30"), "a video says its clock: «{said}»");

    let mut image = row(Some(Kind::Image));
    image.preview = String::new();
    let meta = [
        (crate::media::WIDTH.to_owned(), "800".to_owned()),
        (crate::media::HEIGHT.to_owned(), "600".to_owned()),
    ]
    .into_iter()
    .collect::<super::MetaOfOne>();
    let said = card_of(&image, 1_000_000, Some(&meta), 2);
    assert!(
        said.headline.as_str().contains("800"),
        "an image has nothing to quote, so its measures take the first line: «{}»",
        said.headline
    );
}

#[test]
fn a_card_pasted_more_than_once_carries_its_count_on_the_second_line() {
    assert_eq!(
        super::under_line("476×576", "Orca", "3"),
        "476×576 · Orca · ×3",
        "the count only shows for what was pasted again, so it earns its place"
    );
    assert_eq!(super::under_line("", "", "7"), "×7");
    assert_eq!(
        super::under_line("476×576", "Orca", ""),
        "476×576 · Orca",
        "pasted once or never says nothing"
    );
}

#[test]
fn the_count_reaches_the_general_list_and_not_only_the_views_by_kind() {
    let mut said = row(Some(Kind::Text));
    said.preview = "algo que pegas mucho".into();
    said.paste_count = 4;
    let card = card_of(&said, 1_000_000, None, 2);
    assert_eq!(card.times.as_str(), "4");
    assert!(
        card.under.as_str().contains("×4"),
        "the general list has no head row, so a count that lives only there is a count nobody \
         sees: «{}»",
        card.under
    );
}

#[test]
fn a_folder_says_how_many_things_are_in_it() {
    let mut said = row(Some(Kind::Folder));
    said.preview = r"D:\Mario\Downloads".into();
    let meta = [(crate::folder::ENTRIES.to_owned(), "12".to_owned())]
        .into_iter()
        .collect::<super::MetaOfOne>();
    let under = card_of(&said, 1_000_000, Some(&meta), 2).under.to_string();
    assert!(
        under.contains("12"),
        "the general list asks for this number on every page, so throwing it away was paying for \
         nothing: «{under}»"
    );

    let without = card_of(&said, 1_000_000, None, 2).under.to_string();
    assert!(
        !without.contains("12"),
        "a folder nobody counted says nothing about its contents: «{without}»"
    );
}

#[test]
fn a_name_is_cut_where_a_letter_ends_and_not_where_a_byte_does() {
    let family = "\u{1F468}\u{200D}\u{1F469}\u{200D}\u{1F467}";
    let said = family.repeat(super::NAME_ROOM + 5);
    let kept = super::name_worth_keeping(&said).expect("a name");
    assert!(
        kept.ends_with(family),
        "cutting between the people of a family leaves half a person on the card: {kept:?}"
    );
    assert!(!kept.ends_with('\u{200D}'), "no joiner left hanging");

    let flag = "\u{1F1E8}\u{1F1F1}";
    let kept = super::name_worth_keeping(&flag.repeat(super::NAME_ROOM + 2)).expect("a name");
    assert!(
        kept.ends_with(flag),
        "half a flag is two stray letters: {kept:?}"
    );
}

#[test]
fn a_hit_inside_a_text_of_many_lines_is_shown_on_one() {
    let (lead, hit, tail) = parts_of(&excerpt_of(&[
        ("barra lateral\n  del Finder: el ", false),
        ("mismo", true),
        (" gesto que en o…\n</article>\n", false),
    ]));
    assert_eq!(lead, "barra lateral del Finder: el ");
    assert_eq!(hit, "mismo");
    assert_eq!(tail, " gesto que en o… </article> ");
    assert!(![lead, hit, tail].concat().contains('\n'));
}

#[test]
fn a_colour_says_its_channels_and_anything_else_says_nothing() {
    let mut colour = row(Some(Kind::Color));
    colour.preview = "#FF8800".into();
    assert_eq!(paint_said(&colour), "rgb(255, 136, 0)");
    colour.preview = "#FF880080".into();
    assert_eq!(paint_said(&colour), "rgba(255, 136, 0, 0.50)");
    assert_eq!(paint_said(&row(Some(Kind::Text))), "");
}

#[test]
fn only_what_the_eye_can_see_counts_as_text() {
    for blank in [
        "",
        " ",
        "\n\t\r\n",
        "\u{0}\u{7}",
        "\u{200B}\u{FEFF}",
        "\u{A0}\u{2060}",
    ] {
        assert!(!has_visible_text(blank), "{blank:?} shows nothing");
    }
    for seen in ["a", "  .  ", "\n1\n", "\u{200B}x", "—", "🙂"] {
        assert!(has_visible_text(seen), "{seen:?} shows something");
    }
}

#[test]
fn a_text_with_nothing_to_see_says_so_instead_of_staying_blank() {
    let mut empty = row(Some(Kind::Text));
    empty.preview = " \n\t ".into();
    assert!(says_nothing(&empty));
    let said = nothing_visible_in(crate::say::in_english());
    assert_eq!(body_of(&empty), said);
    let card = card_of(&empty, 1_000_000, None, 2);
    assert!(card.blank);
    assert_eq!(card.squeezed.as_str(), said);
    assert_eq!(card.opened.as_str(), said);
    assert_eq!(card.face.as_str(), "words");
    let mut unknown = empty.clone();
    unknown.kind = None;
    assert!(says_nothing(&unknown));
}

#[test]
fn a_text_with_words_or_a_thing_that_is_not_text_is_not_blank() {
    let card = card_of(&row(Some(Kind::Text)), 1_000_000, None, 2);
    assert!(!card.blank);
    assert_eq!(card.body.as_str(), "hola mundo");
    for kind in [
        Kind::Image,
        Kind::File,
        Kind::Folder,
        Kind::Audio,
        Kind::Video,
    ] {
        let mut quiet = row(Some(kind));
        quiet.preview = String::new();
        assert!(!says_nothing(&quiet), "{kind:?} shows itself another way");
        assert_eq!(body_of(&quiet), "");
    }
    let mut found = row(Some(Kind::Text));
    found.preview = String::new();
    found.snippet = Some(Snippet {
        found_in: FoundIn::Text,
        excerpt: excerpt_of(&[("hit", true)]),
    });
    assert!(!says_nothing(&found));
}

#[test]
fn the_blank_line_is_said_in_both_tongues() {
    assert_eq!(nothing_visible_in(false), "无可见文本");
    assert_eq!(nothing_visible_in(true), "No visible text");
}

#[test]
fn a_picture_waiting_for_its_thumbnail_already_wears_the_picture_face() {
    let mut picture = row(Some(Kind::Image));
    picture.preview = String::new();
    let card = card_of(&picture, 1_000_000, None, 2);
    assert!(!card.has_thumb);
    assert_eq!(card.face.as_str(), "thumb");
    assert_eq!(
        face_for(&picture, false, 2).shut_px(),
        face_for(&picture, true, 2).shut_px(),
        "the card keeps its size when the thumbnail lands"
    );
}

#[test]
fn direction_marks_and_soft_hyphens_alone_show_nothing() {
    for blank in [
        "\u{200E}",
        "\u{200F}\u{202B}",
        "\u{00AD}",
        "\u{2066}\u{2069}",
        "\u{2061}",
    ] {
        assert!(!has_visible_text(blank), "{blank:?} shows nothing");
    }
}

#[test]
fn a_preview_cut_at_the_limit_is_never_called_empty() {
    let mut long = row(Some(Kind::Text));
    long.preview = " ".repeat(cp_store::PREVIEW_CHARS);
    assert!(
        !says_nothing(&long),
        "the visible part may come after the cut"
    );
}

#[test]
fn empty_lines_alone_never_promise_more_lines() {
    let mut blank = row(Some(Kind::Text));
    blank.preview = "\n\n\n".into();
    assert_eq!(more_than_shown(&blank), "");
}

#[test]
fn a_long_text_card_shows_the_lines_the_chosen_height_allows() {
    let mut long = row(Some(Kind::Text));
    long.preview = "palabra ".repeat(60);
    assert_eq!(card_of(&long, 1_000_000, None, 1).shut_lines, 1);
    assert_eq!(card_of(&long, 1_000_000, None, 3).shut_lines, 3);
    let shut = |most| face_for(&long, false, most).shut_px();
    assert!(shut(3) > shut(2) && shut(2) > shut(1));
}

#[test]
fn the_filters_are_always_the_same_four_and_add_up_their_kinds() {
    let facets = [
        Facet {
            kind: Kind::Text,
            count: 3,
        },
        Facet {
            kind: Kind::Code,
            count: 2,
        },
        Facet {
            kind: Kind::Video,
            count: 4,
        },
    ];
    let chips = chips_of(&facets, &["file".to_owned()]);
    let keys: Vec<&str> = chips.iter().map(|one| one.key.as_str()).collect();
    assert_eq!(keys, ["text", "image", "file", "link"]);
    assert_eq!(chips[0].count.as_str(), "5");
    assert_eq!(chips[1].count.as_str(), "0");
    assert_eq!(chips[2].count.as_str(), "4");
    assert!(chips[2].selected && !chips[0].selected);
}

#[test]
fn every_kind_belongs_to_exactly_one_filter() {
    for kind in Kind::ALL {
        let homes = GROUPS
            .iter()
            .filter(|group| kinds_of_group(group).contains(&kind))
            .count();
        assert_eq!(homes, 1, "{kind:?}");
    }
}

#[test]
fn a_finished_hash_word_becomes_a_filter_in_either_language() {
    assert_eq!(
        harvest("#图片 猫 "),
        (vec!["image".to_owned()], "猫 ".to_owned())
    );
    assert_eq!(
        harvest("#file report "),
        (vec!["file".to_owned()], "report ".to_owned())
    );
    assert_eq!(harvest("#图片"), (vec![], "#图片".to_owned()));
    assert_eq!(harvest("#nada x "), (vec![], "#nada x ".to_owned()));
}

#[test]
fn a_kind_no_longer_shown_apart_still_gets_a_card() {
    for kind in [Kind::Token, Kind::Json, Kind::Audio, Kind::Video] {
        let card = card_of(&row(Some(kind)), 1_000_000, None, 2);
        assert_eq!(card.kind.as_str(), kind.as_str());
        assert!(!card.title.is_empty());
    }
}
