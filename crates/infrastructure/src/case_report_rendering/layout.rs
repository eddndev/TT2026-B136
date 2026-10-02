use super::{font::Font, *};
use pdf_writer::{Content, Name};

pub(super) const PAGE_WIDTH: f32 = 595.0;
pub(super) const PAGE_HEIGHT: f32 = 842.0;
const LEFT: f32 = 40.0;
const WIDTH: f32 = PAGE_WIDTH - LEFT * 2.0;
const PAGE_TOP: f32 = 777.0;
const PAGE_BOTTOM: f32 = 55.0;
const PAGE_ROOM: f32 = PAGE_TOP - PAGE_BOTTOM;
const CONTENT_BUDGET: usize = MAX_REPORT_ARTIFACT_BYTES - 2 * 1024 * 1024;

struct Paragraph {
    lines: Vec<String>,
    bold: bool,
    size: f32,
}
impl Paragraph {
    fn line_height(&self) -> f32 {
        self.size * 1.45
    }
    fn height(&self) -> f32 {
        self.lines.len() as f32 * self.line_height()
    }
}

pub(super) struct Layout<'a> {
    pub fonts: [Font<'a>; 2],
    pub pages: Vec<Vec<u8>>,
    content: Content,
    y: f32,
    bytes: usize,
    glyphs: usize,
    section: String,
}

impl<'a> Layout<'a> {
    pub fn new(regular: &'a [u8], bold: &'a [u8]) -> Result<Self, ApplicationError> {
        let mut result = Self {
            fonts: [Font::new(regular)?, Font::new(bold)?],
            pages: Vec::new(),
            content: Content::new(),
            y: 765.0,
            bytes: 0,
            glyphs: 0,
            section: "Captura del informe".into(),
        };
        result.header()?;
        Ok(result)
    }

    fn draw(
        &mut self,
        text: &str,
        bold: bool,
        size: f32,
        x: f32,
        y: f32,
        width: f32,
    ) -> Result<(), ApplicationError> {
        // Check before shaping and writing, retaining bounded page/object overhead.
        if self.bytes + self.content.len() + text.len() * 160 + 4096 > CONTENT_BUDGET
            || self.glyphs + text.len() > bounds::MAX_GLYPHS
        {
            return Err(capacity());
        }
        let index = usize::from(bold);
        let name = Name(if bold { b"F2" } else { b"F1" });
        self.glyphs +=
            self.fonts[index].draw(&mut self.content, text, name, size, (x, y), width)?;
        if self.bytes + self.content.len() > CONTENT_BUDGET {
            return Err(capacity());
        }
        Ok(())
    }

    fn header(&mut self) -> Result<(), ApplicationError> {
        self.content.set_fill_rgb(0.10, 0.19, 0.25);
        self.draw(
            "QADRA  |  Informe de expedientes",
            true,
            11.0,
            LEFT,
            811.0,
            WIDTH,
        )?;
        let section = self.section.clone();
        self.draw(&section, false, 8.0, LEFT, 796.0, WIDTH)?;
        self.content
            .set_fill_rgb(0.14, 0.22, 0.25)
            .rect(LEFT, 790.0, WIDTH, 0.7)
            .fill_nonzero();
        self.content.set_fill_rgb(0.12, 0.15, 0.18);
        Ok(())
    }

    fn finish_page(&mut self) -> Result<(), ApplicationError> {
        if self.pages.len() >= bounds::MAX_PAGES {
            return Err(capacity());
        }
        self.content.set_fill_rgb(0.30, 0.34, 0.38);
        self.draw(
            &format!("P\u{e1}gina {}", self.pages.len() + 1),
            false,
            8.0,
            LEFT,
            31.0,
            WIDTH,
        )?;
        self.draw(
            "Observaci\u{f3}n administrativa; no acredita efectos jur\u{ed}dicos.",
            false,
            8.0,
            140.0,
            31.0,
            WIDTH - 100.0,
        )?;
        let bytes = std::mem::replace(&mut self.content, Content::new())
            .finish()
            .into_vec();
        self.bytes += bytes.len();
        self.pages.push(bytes);
        Ok(())
    }

    fn next_page(&mut self) -> Result<(), ApplicationError> {
        self.finish_page()?;
        if self.pages.len() >= bounds::MAX_PAGES {
            return Err(capacity());
        }
        self.y = PAGE_TOP;
        self.header()
    }

    fn room(&mut self, height: f32) -> Result<(), ApplicationError> {
        if self.y - height < PAGE_BOTTOM {
            self.next_page()?;
        }
        Ok(())
    }

    fn prepare(&self, text: &str, bold: bool, size: f32) -> Result<Paragraph, ApplicationError> {
        Ok(Paragraph {
            lines: self.fonts[usize::from(bold)].lines(text, size, WIDTH)?,
            bold,
            size,
        })
    }

    fn line(&mut self, text: &str, paragraph: &Paragraph) -> Result<(), ApplicationError> {
        self.draw(text, paragraph.bold, paragraph.size, LEFT, self.y, WIDTH)?;
        self.y -= paragraph.line_height();
        Ok(())
    }

    pub fn gap(&mut self, points: f32) {
        self.y -= points;
    }

    pub fn paragraph(&mut self, text: &str, bold: bool, size: f32) -> Result<(), ApplicationError> {
        let lines = self.fonts[usize::from(bold)].lines(text, size, WIDTH)?;
        for line in lines {
            self.room(size * 1.45)?;
            self.draw(&line, bold, size, LEFT, self.y, WIDTH)?;
            self.y -= size * 1.45;
        }
        Ok(())
    }

    pub fn section(&mut self, title: &str) -> Result<(), ApplicationError> {
        self.room(45.0)?;
        self.section = title.into();
        self.gap(10.0);
        self.paragraph(title, true, 13.0)?;
        self.gap(5.0);
        Ok(())
    }

    fn row_heading(&mut self, heading: &Paragraph) -> Result<(), ApplicationError> {
        self.content
            .set_fill_rgb(0.91, 0.95, 0.94)
            .rect(
                LEFT,
                self.y - heading.height() + 10.5,
                WIDTH,
                heading.height() + 3.5,
            )
            .fill_nonzero();
        self.content.set_fill_rgb(0.10, 0.24, 0.23);
        for line in &heading.lines {
            self.line(line, heading)?;
        }
        self.content.set_fill_rgb(0.12, 0.15, 0.18);
        self.gap(4.0);
        Ok(())
    }

    pub fn row(
        &mut self,
        heading: &str,
        paragraphs: Vec<(String, bool, f32)>,
    ) -> Result<(), ApplicationError> {
        // Wrap once: the same prepared lines determine fit and are later drawn.
        let heading = self.prepare(heading, true, 10.0)?;
        let label = format!("{} (continuacion)", heading.lines.join(" "));
        let continuation = self.prepare(&label, true, 10.0)?;
        let paragraphs = paragraphs
            .into_iter()
            .map(|(text, bold, size)| self.prepare(&text, bold, size))
            .collect::<Result<Vec<_>, _>>()?;
        let height = heading.height() + 4.0 + paragraphs.iter().map(Paragraph::height).sum::<f32>();
        // A small slack avoids moving the final line because of float rounding.
        if height + 1.0 <= PAGE_ROOM {
            self.room(height + 1.0)?;
        } else {
            self.room(
                heading.height() + 4.0 + paragraphs.first().map_or(0.0, Paragraph::height) + 1.0,
            )?;
        }
        self.row_heading(&heading)?;
        let fresh_body = PAGE_ROOM - continuation.height() - 5.0;
        for paragraph in &paragraphs {
            // Keep an assignment or metadata field together when it fits a page.
            if paragraph.height() <= fresh_body && self.y - paragraph.height() - 1.0 < PAGE_BOTTOM {
                self.next_page()?;
                self.row_heading(&continuation)?;
            }
            for line in &paragraph.lines {
                if self.y - paragraph.line_height() < PAGE_BOTTOM {
                    self.next_page()?;
                    self.row_heading(&continuation)?;
                }
                self.line(line, paragraph)?;
            }
        }
        Ok(())
    }

    pub fn finish(mut self) -> Result<Self, ApplicationError> {
        self.finish_page()?;
        Ok(self)
    }
}
