use super::*;
use pdf_writer::{Content, Finish, Name, Str, TextStr};
use std::collections::BTreeMap;

#[cfg(test)]
#[path = "font_plan_tests.rs"]
mod tests;

pub(super) struct EncodedGlyph {
    pub glyph: u16,
    pub original: String,
    pub width: f32,
}

pub(super) struct Font<'a> {
    pub bytes: &'a [u8],
    pub face: rustybuzz::Face<'a>,
    pub glyphs: Vec<EncodedGlyph>,
    map: BTreeMap<(u16, String), u16>,
    plan: rustybuzz::ShapePlan,
}

struct Shaped {
    glyph: u16,
    start: usize,
    end: usize,
    advance: f32,
    x: f32,
    y: f32,
}

impl<'a> Font<'a> {
    pub fn new(bytes: &'a [u8]) -> Result<Self, ApplicationError> {
        let face = rustybuzz::Face::from_slice(bytes, 0).ok_or_else(unavailable)?;
        if face.units_per_em() == 0 {
            return Err(unavailable());
        }
        let plan = shape_plan(&face);
        Ok(Self {
            bytes,
            face,
            glyphs: Vec::new(),
            map: BTreeMap::new(),
            plan,
        })
    }

    fn shaped_buffer(&self, text: &str) -> rustybuzz::GlyphBuffer {
        let mut buffer = rustybuzz::UnicodeBuffer::new();
        buffer.push_str(text);
        buffer.set_direction(rustybuzz::Direction::LeftToRight);
        buffer.set_script(rustybuzz::script::LATIN);
        buffer.set_cluster_level(rustybuzz::BufferClusterLevel::MonotoneGraphemes);
        rustybuzz::shape_with_plan(&self.face, &self.plan, buffer)
    }

    fn shape(&self, text: &str) -> Result<Vec<Shaped>, ApplicationError> {
        if text.len() > bounds::MAX_FIELD_BYTES {
            return Err(capacity());
        }
        let mut marks = 0;
        let mut base = false;
        for ch in text.chars() {
            let code = ch as u32;
            let combining = (0x300..=0x36f).contains(&code);
            let allowed = matches!(code, 0x20..=0x7e | 0xa0..=0x2af | 0x300..=0x36f
                | 0x2010..=0x2027 | 0x2030..=0x205e | 0x20a0..=0x20cf | 0x2212);
            if !allowed || ch.is_control() || self.face.glyph_index(ch).is_none() {
                return Err(failed());
            }
            if combining {
                marks += 1;
                if !base || marks > 16 {
                    return Err(failed());
                }
            } else {
                marks = 0;
                base = ch.is_alphabetic();
            }
        }
        let shaped = self.shaped_buffer(text);
        let infos = shaped.glyph_infos();
        if infos.len() > bounds::MAX_FIELD_BYTES * 2 {
            return Err(capacity());
        }
        let mut ends: Vec<usize> = infos.iter().map(|info| info.cluster as usize).collect();
        ends.push(text.len());
        ends.sort_unstable();
        ends.dedup();
        let mut result = Vec::with_capacity(infos.len());
        for (info, position) in infos.iter().zip(shaped.glyph_positions()) {
            let start = info.cluster as usize;
            let index = ends.binary_search(&start).map_err(|_| failed())?;
            let end = *ends.get(index + 1).ok_or_else(failed)?;
            if info.glyph_id == 0
                || info.glyph_id > u16::MAX as u32
                || !text.is_char_boundary(start)
                || !text.is_char_boundary(end)
            {
                return Err(failed());
            }
            result.push(Shaped {
                glyph: info.glyph_id as u16,
                start,
                end,
                advance: position.x_advance as f32,
                x: position.x_offset as f32,
                y: position.y_offset as f32,
            });
        }
        Ok(result)
    }

    pub fn lines(
        &self,
        text: &str,
        size: f32,
        width: f32,
    ) -> Result<Vec<String>, ApplicationError> {
        let shaped = self.shape(text)?;
        if shaped.is_empty() {
            return Ok(vec![String::new()]);
        }
        let scale = size / self.face.units_per_em() as f32;
        let mut clusters: Vec<(usize, usize, f32)> = Vec::new();
        for glyph in shaped {
            if let Some(last) = clusters.last_mut().filter(|last| last.0 == glyph.start) {
                last.2 += glyph.advance * scale;
            } else {
                clusters.push((glyph.start, glyph.end, glyph.advance * scale));
            }
        }
        let mut lines = Vec::new();
        let mut start = 0;
        while start < clusters.len() {
            let mut end = start;
            let mut used = 0.0;
            let mut space = None;
            while end < clusters.len() && used + clusters[end].2 <= width - 6.0 {
                used += clusters[end].2;
                if text[clusters[end].0..clusters[end].1]
                    .chars()
                    .all(char::is_whitespace)
                {
                    space = Some(end + 1);
                }
                end += 1;
            }
            if end == start {
                return Err(failed());
            }
            if end < clusters.len() {
                if let Some(boundary) = space {
                    end = boundary;
                }
            }
            lines.push(text[clusters[start].0..clusters[end - 1].1].to_owned());
            if lines.len() > 2048 {
                return Err(capacity());
            }
            start = end;
        }
        Ok(lines)
    }

    fn cid(&mut self, glyph: u16, source: &str) -> Result<u16, ApplicationError> {
        let key = (glyph, source.to_owned());
        if let Some(cid) = self.map.get(&key) {
            return Ok(*cid);
        }
        let cid = u16::try_from(self.glyphs.len() + 1).map_err(|_| capacity())?;
        let width = self
            .face
            .glyph_hor_advance(ttf_parser::GlyphId(glyph))
            .ok_or_else(failed)? as f32
            * 1000.0
            / self.face.units_per_em() as f32;
        self.glyphs.push(EncodedGlyph {
            glyph,
            original: source.to_owned(),
            width,
        });
        self.map.insert(key, cid);
        Ok(cid)
    }

    pub fn draw(
        &mut self,
        content: &mut Content,
        text: &str,
        name: Name<'_>,
        size: f32,
        origin: (f32, f32),
        width: f32,
    ) -> Result<usize, ApplicationError> {
        let (x, y) = origin;
        let shaped = self.shape(text)?;
        let scale = size / self.face.units_per_em() as f32;
        let advance: f32 = shaped.iter().map(|glyph| glyph.advance * scale).sum();
        if !advance.is_finite() || advance > width {
            return Err(failed());
        }
        let mut cids = Vec::with_capacity(shaped.len());
        let mut previous = None;
        for glyph in &shaped {
            let source = if previous == Some(glyph.start) {
                ""
            } else {
                &text[glyph.start..glyph.end]
            };
            cids.push(self.cid(glyph.glyph, source)?);
            previous = Some(glyph.start);
        }
        content
            .begin_marked_content_with_properties(Name(b"Span"))
            .properties()
            .actual_text(TextStr(text));
        content
            .begin_text()
            .set_font(name, size)
            .set_text_matrix([1.0, 0.0, 0.0, 1.0, x, y]);
        if shaped.iter().all(|glyph| glyph.x == 0.0 && glyph.y == 0.0) {
            let mut position = content.show_positioned();
            let mut items = position.items();
            let mut pending = Vec::new();
            for (glyph, cid) in shaped.iter().zip(&cids) {
                pending.extend_from_slice(&cid.to_be_bytes());
                let nominal = self.glyphs[*cid as usize - 1].width;
                let adjustment = nominal - glyph.advance * 1000.0 / self.face.units_per_em() as f32;
                if adjustment.abs() > 0.01 {
                    items.show(Str(&pending)).adjust(adjustment);
                    pending.clear();
                }
            }
            if !pending.is_empty() {
                items.show(Str(&pending));
            }
            items.finish();
            position.finish();
        } else {
            let mut cursor = x;
            for (glyph, cid) in shaped.iter().zip(&cids) {
                content
                    .set_text_matrix([
                        1.0,
                        0.0,
                        0.0,
                        1.0,
                        cursor + glyph.x * scale,
                        y + glyph.y * scale,
                    ])
                    .show(Str(&cid.to_be_bytes()));
                cursor += glyph.advance * scale;
            }
        }
        content.end_text().end_marked_content();
        Ok(shaped.len())
    }
}

fn shape_plan(face: &rustybuzz::Face<'_>) -> rustybuzz::ShapePlan {
    // Every report buffer uses Latin, left-to-right text and default features.
    let plan = rustybuzz::ShapePlan::new(
        face,
        rustybuzz::Direction::LeftToRight,
        Some(rustybuzz::script::LATIN),
        None,
        &[],
    );
    #[cfg(test)]
    tests::record_plan_build();
    plan
}
