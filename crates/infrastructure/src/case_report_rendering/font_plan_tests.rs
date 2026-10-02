use super::*;
use std::cell::Cell;

thread_local! {
    static PLAN_BUILDS: Cell<usize> = const { Cell::new(0) };
}

pub(super) fn record_plan_build() {
    PLAN_BUILDS.with(|count| count.set(count.get() + 1));
}

fn font_bytes() -> [&'static [u8]; 2] {
    [
        include_bytes!("../../assets/case-reports/NotoSans-Regular.ttf"),
        include_bytes!("../../assets/case-reports/NotoSans-Bold.ttf"),
    ]
}

fn reference(face: &rustybuzz::Face<'_>, text: &str) -> rustybuzz::GlyphBuffer {
    let mut buffer = rustybuzz::UnicodeBuffer::new();
    buffer.push_str(text);
    buffer.set_direction(rustybuzz::Direction::LeftToRight);
    buffer.set_script(rustybuzz::script::LATIN);
    buffer.set_cluster_level(rustybuzz::BufferClusterLevel::MonotoneGraphemes);
    rustybuzz::shape(face, &[], buffer)
}

fn glyphs(buffer: &rustybuzz::GlyphBuffer) -> Vec<(u32, u32, i32, i32, i32, i32)> {
    buffer
        .glyph_infos()
        .iter()
        .zip(buffer.glyph_positions())
        .map(|(info, position)| {
            (
                info.glyph_id,
                info.cluster,
                position.x_advance,
                position.y_advance,
                position.x_offset,
                position.y_offset,
            )
        })
        .collect()
}

#[test]
fn reused_plan_preserves_glyphs_clusters_and_all_positions() {
    let accents = "\u{00c1}rea jur\u{00ed}dica: ni\u{00f1}ez, acci\u{00f3}n, a\u{0301}\u{0327}.";
    let ligatures = "office affinity efficient afflict fi ffi fl";
    let long = format!("{accents} {ligatures}; expediente 0042. ").repeat(48);
    assert!(long.len() > 4096 && long.len() < bounds::MAX_FIELD_BYTES);
    for bytes in font_bytes() {
        let font = Font::new(bytes).expect("bundled font");
        for text in ["", "AV To office", accents, ligatures, long.as_str()] {
            let expected = reference(&font.face, text);
            let actual = font.shaped_buffer(text);
            assert_eq!(glyphs(&actual), glyphs(&expected));
            let rendered = font.shape(text).expect("supported text");
            assert_eq!(rendered.len(), expected.glyph_infos().len());
            let mut clusters: Vec<usize> = expected
                .glyph_infos()
                .iter()
                .map(|info| info.cluster as usize)
                .collect();
            clusters.push(text.len());
            clusters.sort_unstable();
            clusters.dedup();
            for ((glyph, info), position) in rendered
                .iter()
                .zip(expected.glyph_infos())
                .zip(expected.glyph_positions())
            {
                assert_eq!(u32::from(glyph.glyph), info.glyph_id);
                assert_eq!(glyph.start, info.cluster as usize);
                let index = clusters.binary_search(&glyph.start).expect("cluster");
                assert_eq!(glyph.end, clusters[index + 1]);
                assert_eq!(glyph.advance, position.x_advance as f32);
                assert_eq!(glyph.x, position.x_offset as f32);
                assert_eq!(glyph.y, position.y_offset as f32);
            }
        }
        let shaped = reference(&font.face, ligatures);
        assert!(shaped.glyph_infos().len() < ligatures.chars().count());
    }
}

#[test]
fn measuring_and_drawing_many_lines_builds_only_one_plan_per_font() {
    PLAN_BUILDS.with(|count| count.set(0));
    let mut fonts: Vec<Font<'_>> = font_bytes()
        .into_iter()
        .map(|bytes| Font::new(bytes).expect("bundled font"))
        .collect();
    let paragraph =
        "\u{00c1}rea de acci\u{00f3}n, a\u{0301}\u{0327}, office affinity; expediente 0042. "
            .repeat(24);
    let mut drawn_lines = 0;
    for _ in 0..8 {
        for font in &mut fonts {
            let lines = font.lines(&paragraph, 10.0, 440.0).expect("wrapped text");
            assert_eq!(lines.concat(), paragraph);
            let mut content = Content::new();
            for line in lines {
                let count = font
                    .draw(&mut content, &line, Name(b"F1"), 10.0, (40.0, 750.0), 440.0)
                    .expect("draw wrapped line");
                assert!(count > 0);
                drawn_lines += 1;
            }
        }
    }
    assert!(drawn_lines > 100);
    assert_eq!(
        PLAN_BUILDS.with(Cell::get),
        2,
        "measuring and drawing must reuse one construction per font"
    );
}
