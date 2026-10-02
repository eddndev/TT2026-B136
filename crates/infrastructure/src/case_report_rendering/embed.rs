use super::{font::Font, *};
use pdf_writer::{
    types::{CidFontType, FontFlags, SystemInfo, UnicodeCmap},
    Finish, Name, Pdf, Rect, Ref, Str,
};

pub(super) fn font(
    pdf: &mut Pdf,
    font: &Font<'_>,
    first: i32,
    bold: bool,
) -> Result<(), ApplicationError> {
    let name = Name(if bold {
        b"NotoSans-Bold"
    } else {
        b"NotoSans-Regular"
    });
    let type0 = Ref::new(first);
    let cid = Ref::new(first + 1);
    let descriptor = Ref::new(first + 2);
    let data = Ref::new(first + 3);
    let unicode = Ref::new(first + 4);
    let map = Ref::new(first + 5);
    let info = SystemInfo {
        registry: Str(b"Adobe"),
        ordering: Str(b"Identity"),
        supplement: 0,
    };
    pdf.type0_font(type0)
        .base_font(name)
        .encoding_predefined(Name(b"Identity-H"))
        .descendant_font(cid)
        .to_unicode(unicode);
    let mut descendant = pdf.cid_font(cid);
    descendant
        .subtype(CidFontType::Type2)
        .base_font(name)
        .system_info(info)
        .font_descriptor(descriptor)
        .default_width(0.0)
        .cid_to_gid_map_stream(map);
    descendant
        .widths()
        .consecutive(1, font.glyphs.iter().map(|glyph| glyph.width));
    descendant.finish();
    let scale = 1000.0 / font.metrics.units_per_em as f32;
    let bounds = font.metrics.bounds.ok_or_else(unavailable)?;
    pdf.font_descriptor(descriptor)
        .name(name)
        .flags(FontFlags::NON_SYMBOLIC)
        .bbox(Rect::new(
            bounds.x_min * scale,
            bounds.y_min * scale,
            bounds.x_max * scale,
            bounds.y_max * scale,
        ))
        .italic_angle(0.0)
        .ascent(font.metrics.ascent * scale)
        .descent(font.metrics.descent * scale)
        .cap_height(font.metrics.cap_height.unwrap_or(font.metrics.ascent) * scale)
        .stem_v(if bold { 120.0 } else { 80.0 })
        .font_file2(data);
    pdf.stream(data, font.bytes)
        .pair(Name(b"Length1"), font.bytes.len() as i32);
    let mut cmap = UnicodeCmap::<u16>::new(Name(b"ReportUnicode"), info);
    let mut mapping = vec![0, 0];
    for (index, glyph) in font.glyphs.iter().enumerate() {
        mapping.extend_from_slice(&glyph.glyph.to_be_bytes());
        if !glyph.original.is_empty() {
            cmap.pair_with_multiple((index + 1) as u16, glyph.original.chars());
        }
    }
    pdf.stream(unicode, &cmap.finish());
    pdf.stream(map, &mapping);
    if pdf.len() > MAX_REPORT_ARTIFACT_BYTES {
        return Err(capacity());
    }
    Ok(())
}
