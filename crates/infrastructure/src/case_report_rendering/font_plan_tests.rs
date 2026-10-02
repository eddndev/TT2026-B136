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

fn reference_texts() -> Vec<String> {
    let accents = "\u{00c1}rea jur\u{00ed}dica: ni\u{00f1}ez, acci\u{00f3}n, a\u{0301}\u{0327}.";
    let ligatures = "office affinity efficient afflict fi ffi fl";
    vec![
        String::new(),
        "AV To office".into(),
        accents.into(),
        ligatures.into(),
        format!("{accents} {ligatures}; expediente 0042. ").repeat(48),
    ]
}

fn reference_hex(text: &str) -> String {
    text.as_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn shape_reference(font: &Font<'_>) -> String {
    use std::fmt::Write;
    let bounds = font.metrics.bounds.expect("pinned font bounds");
    let mut out = format!(
        "metrics:{},{},{},{},{},{},{},{}\n",
        font.metrics.units_per_em,
        font.metrics.ascent,
        font.metrics.descent,
        font.metrics.cap_height.unwrap_or(font.metrics.ascent),
        bounds.x_min,
        bounds.y_min,
        bounds.x_max,
        bounds.y_max
    );
    for text in reference_texts() {
        let shaped = font.shaped_buffer(&text);
        write!(out, "text:{}|", reference_hex(&text)).unwrap();
        for (info, position) in shaped.glyph_infos().iter().zip(shaped.glyph_positions()) {
            let width = font
                .glyph_metrics
                .advance_width(skrifa::GlyphId::new(info.glyph_id))
                .unwrap();
            write!(
                out,
                "{},{},{},{},{},{},{};",
                info.glyph_id,
                info.cluster,
                position.x_advance,
                position.y_advance,
                position.x_offset,
                position.y_offset,
                width
            )
            .unwrap();
        }
        out.push('\n');
        for line in font
            .lines(&text, 10.0, 440.0)
            .expect("bounded reference lines")
        {
            write!(out, "line:{};", reference_hex(&line)).unwrap();
        }
        out.push('\n');
    }
    out
}

#[test]
fn maintained_shaping_preserves_pinned_metrics_glyphs_clusters_positions_and_lines() {
    let expected = [
        include_str!("font-reference/regular-shaping.txt"),
        include_str!("font-reference/bold-shaping.txt"),
    ];
    for (bytes, expected) in font_bytes().into_iter().zip(expected) {
        let font = Font::new(bytes).expect("bundled font");
        assert_eq!(shape_reference(&font), expected);
        for text in reference_texts() {
            let shaped = font.shaped_buffer(&text);
            let rendered = font.shape(&text).expect("supported text");
            assert_eq!(rendered.len(), shaped.glyph_infos().len());
            let mut clusters: Vec<usize> = shaped
                .glyph_infos()
                .iter()
                .map(|info| info.cluster as usize)
                .collect();
            clusters.push(text.len());
            clusters.sort_unstable();
            clusters.dedup();
            for ((glyph, info), position) in rendered
                .iter()
                .zip(shaped.glyph_infos())
                .zip(shaped.glyph_positions())
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
        let ligatures = "office affinity efficient afflict fi ffi fl";
        assert!(font.shaped_buffer(ligatures).glyph_infos().len() < ligatures.chars().count());
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
