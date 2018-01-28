use lester::{PdfDocument, RenderOptions, Backdrop};
use std::env;
use std::fs::File;
use std::io::Write;
use victor::pdf::Document;
use victor::primitives::{Size, TextRun, Length, point, RGBA, rect};
use victor::fonts2::{BITSTREAM_VERA_SANS, LazyStaticFont, FontError};

static AHEM: LazyStaticFont = include_font!("fonts/ahem/ahem.ttf");
static NOTO: LazyStaticFont = include_font!("fonts/noto/NotoSansLinearB-Regular.ttf");

fn doc() -> Result<Vec<u8>, FontError> {
    let vera = BITSTREAM_VERA_SANS.get()?;
    let noto = NOTO.get()?;
    let ahem = AHEM.get()?;
    let mut doc = Document::new();
    doc.add_page(Size::new(140., 50.))
        .show_text(&TextRun {
            glyph_ids: vera.to_glyph_ids("Têst→iimm")?,
            font: vera,
            font_size: Length::new(15.),
            origin: point(10., 20.),
        })?
        .show_text(&TextRun {
            glyph_ids: ahem.to_glyph_ids("pÉX")?,
            font: ahem,
            font_size: Length::new(15.),
            origin: point(10., 40.),
        })?
        .show_text(&TextRun {
            glyph_ids: noto.to_glyph_ids("𐁉 𐁁𐀓𐀠𐀴𐀍")?,
            font: noto,
            font_size: Length::new(15.),
            origin: point(65., 40.),
        })?;
    doc.add_page(Size::new(4., 4.))
        .set_color(&RGBA(0., 0., 1., 1.))
        .paint_rectangle(&rect(0., 1., 4., 3.))

        .set_color(&RGBA(1., 0., 0., 0.5))
        .paint_rectangle(&rect(0., 0., 1., 2.));

    Ok(doc.write_to_pdf_bytes())
}

#[test]
fn pdf() {
    let pdf_bytes = doc().unwrap();

    if env::var("VICTOR_WRITE_TO_TMP").is_ok() {
        File::create("/tmp/victor.pdf").unwrap().write_all(&pdf_bytes).unwrap();
    }
    if env::var("VICTOR_PRINT").is_ok() {
        println!("{}", String::from_utf8_lossy(&pdf_bytes));
    }
    let doc = PdfDocument::from_bytes(&pdf_bytes).unwrap();
    assert_eq!(doc.producer().unwrap().to_str().unwrap(),
               "Victor <https://github.com/SimonSapin/victor>");

    let pages: Vec<_> = doc.pages().collect();
    assert_eq!(pages[0].size_in_css_px(), (140., 50.));
    assert_eq!(pages[1].size_in_css_px(), (4., 4.));

    // FIXME: find a way to round-trip code points without a glyph like '→'
    assert_eq!(pages[0].text().to_str().unwrap(), "Têst iimm\npÉX 𐁉 𐁁𐀓𐀠𐀴𐀍");
    assert_eq!(pages[1].text().to_str().unwrap(), "");

    if env::var("VICTOR_WRITE_TO_TMP").is_ok() {
        pages[0].render_with_dppx(3.).unwrap()
                .write_to_png_file("/tmp/victor.png").unwrap()
    }
    let mut surface = pages[1].render().unwrap();
    const RED_: u32 = 0x8080_0000;
    const BLUE: u32 = 0xFF00_00FF;
    const BOTH: u32 = 0xFF80_007F;
    const ____: u32 = 0x0000_0000;
    assert_pixels_eq!(surface.pixels().buffer, &[
        RED_, ____, ____, ____,
        BOTH, BLUE, BLUE, BLUE,
        BLUE, BLUE, BLUE, BLUE,
        BLUE, BLUE, BLUE, BLUE,
    ]);

    let mut surface = pages[1].render_with_options(RenderOptions {
        dppx_x: 2.0,
        dppx_y: 3.0,
        backdrop: Backdrop::White,
        ..RenderOptions::default()
    }).unwrap();
    let pixels = surface.pixels();
    assert_eq!((pixels.width, pixels.height), (8, 12));
    {
        const RED_: u32 = 0xFFFF_7F7F;
        const ____: u32 = 0xFFFF_FFFF;
        assert_pixels_eq!(pixels.buffer, &[
            RED_, RED_, ____, ____, ____, ____, ____, ____,
            RED_, RED_, ____, ____, ____, ____, ____, ____,
            RED_, RED_, ____, ____, ____, ____, ____, ____,
            BOTH, BOTH, BLUE, BLUE, BLUE, BLUE, BLUE, BLUE,
            BOTH, BOTH, BLUE, BLUE, BLUE, BLUE, BLUE, BLUE,
            BOTH, BOTH, BLUE, BLUE, BLUE, BLUE, BLUE, BLUE,
            BLUE, BLUE, BLUE, BLUE, BLUE, BLUE, BLUE, BLUE,
            BLUE, BLUE, BLUE, BLUE, BLUE, BLUE, BLUE, BLUE,
            BLUE, BLUE, BLUE, BLUE, BLUE, BLUE, BLUE, BLUE,
            BLUE, BLUE, BLUE, BLUE, BLUE, BLUE, BLUE, BLUE,
            BLUE, BLUE, BLUE, BLUE, BLUE, BLUE, BLUE, BLUE,
            BLUE, BLUE, BLUE, BLUE, BLUE, BLUE, BLUE, BLUE,
        ][..]);
    }

    assert!(pdf_bytes == include_bytes!("expected.pdf").as_ref());
}
