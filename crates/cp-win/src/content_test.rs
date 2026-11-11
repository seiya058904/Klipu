use super::*;
use cp_core::item::Format;
use cp_core::kind::Kind;
use cp_win_sys::writing::utf16_of;

fn inline(id: &str, bytes: &[u8]) -> Format {
    Format {
        id: id.into(),
        payload: Payload::Inline(bytes.to_vec()),
    }
}

fn unicode(text: &str) -> Format {
    inline("CF_UNICODETEXT", &utf16_of(text))
}

#[test]
fn plain_rtf_alone_is_rich_and_a_blob_reads_like_an_inline_payload() {
    let item = Item {
        kind: Some(Kind::Text),
        formats: vec![
            inline("Rich Text Format", b"{\\rtf1 hola}"),
            Format {
                id: "CF_UNICODETEXT".into(),
                payload: Payload::Blob(utf16_of("hola")),
            },
        ],
    };
    let content = content_of(&item, None);
    assert_eq!(
        content.text.as_deref(),
        Some("hola"),
        "un blob se lee igual que un inline"
    );
}

#[test]
fn a_bitmap_only_copy_is_the_png_we_transcoded_from_it() {
    let item = Item {
        kind: Some(Kind::Image),
        formats: vec![
            Format {
                id: "CF_DIBV5".into(),
                payload: Payload::Announced { size: Some(4) },
            },
            inline("CF_DIB", &[40, 0, 0, 0]),
            inline(SYNTHETIC_IMAGE, &[137, 80, 78, 71]),
        ],
    };
    let content = content_of(&item, None);
    assert_eq!(
        content.image,
        Some(&[137u8, 80, 78, 71][..]),
        "the raw DIB is not offered: the image is the synthetic PNG"
    );
    let both = Item {
        kind: Some(Kind::Image),
        formats: vec![inline(SYNTHETIC_IMAGE, &[1]), inline("PNG", &[2])],
    };
    assert_eq!(
        content_of(&both, None).image,
        Some(&[2u8][..]),
        "el PNG de la fuente manda si lo hay"
    );
}

#[test]
fn what_the_store_wrote_itself_is_read_back_the_same_way() {
    let edited = Item::plain("editado a mano");
    assert_eq!(
        content_of(&edited, None).text.as_deref(),
        Some("editado a mano")
    );
    let synthetic = Item {
        kind: Some(Kind::Image),
        formats: vec![inline(SYNTHETIC_IMAGE, &[1, 2, 3])],
    };
    assert_eq!(content_of(&synthetic, None).image, Some(&[1u8, 2, 3][..]));
}

fn jpeg() -> Vec<u8> {
    let pixels = image::RgbImage::from_pixel(16, 8, image::Rgb([180, 50, 20]));
    let mut out = std::io::Cursor::new(Vec::new());
    image::DynamicImage::ImageRgb8(pixels)
        .write_to(&mut out, image::ImageFormat::Jpeg)
        .expect("encoded");
    out.into_inner()
}

#[test]
fn captured_and_legacy_jpegs_supply_their_original_bytes_to_image_features() {
    let jpeg = jpeg();
    for id in ["JFIF", SYNTHETIC_JPEG] {
        let item = Item {
            kind: Some(Kind::Image),
            formats: vec![inline(id, &jpeg)],
        };
        let bytes = content_of(&item, None).image.expect("readable image");
        assert_eq!(bytes, jpeg);
        let thumb = cp_core::thumbnail::of_image(bytes, 8).expect("a thumbnail");
        assert_eq!(
            cp_core::thumbnail::size_of(&thumb),
            Some(cp_core::thumbnail::Size {
                width: 8,
                height: 4
            })
        );
        assert!(cp_core::dib::from_jpeg(bytes).is_some());
    }
}

#[test]
fn a_kept_png_still_wins_and_an_announced_png_does_not_hide_a_jpeg() {
    let jpeg = jpeg();
    let mut item = Item {
        kind: Some(Kind::Image),
        formats: vec![inline("JFIF", &jpeg), inline(SYNTHETIC_IMAGE, b"png")],
    };
    assert_eq!(content_of(&item, None).image, Some(b"png".as_slice()));
    item.formats[1].payload = Payload::Announced { size: None };
    assert_eq!(content_of(&item, None).image, Some(jpeg.as_slice()));
}

#[test]
fn a_broken_jpeg_is_kept_for_restore_but_cannot_make_a_thumbnail() {
    let item = Item {
        kind: Some(Kind::Image),
        formats: vec![inline("JFIF", b"not a jpeg")],
    };
    let bytes = content_of(&item, None).image.expect("the stored bytes");
    assert_eq!(bytes, b"not a jpeg");
    assert_eq!(cp_core::thumbnail::of_image(bytes, 8), None);
}

#[test]
fn the_system_text_wins_over_the_synthetic_one() {
    let item = Item {
        kind: Some(Kind::Text),
        formats: vec![inline(SYNTHETIC_TEXT, b"sintetico"), unicode("del sistema")],
    };
    assert_eq!(content_of(&item, None).text.as_deref(), Some("del sistema"));
}

#[test]
fn bytes_that_are_not_text_do_not_pretend_to_be() {
    let lone_surrogate = [0x00, 0xD8, 0x00, 0x00];
    let item = Item {
        kind: Some(Kind::Text),
        formats: vec![
            inline("CF_UNICODETEXT", &lone_surrogate),
            inline("HTML Format", &[0xff, 0xfe, 0x00]),
        ],
    };
    let content = content_of(&item, None);
    assert_eq!(content.text, None);
}
