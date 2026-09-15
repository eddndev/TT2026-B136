use std::io::{Cursor, Write};
use zip::{write::SimpleFileOptions, CompressionMethod, ZipWriter};

pub const WORD: &str = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";
pub struct Part {
    pub name: String,
    pub bytes: Vec<u8>,
}
impl Part {
    pub fn text(name: &str, content: &str) -> Self {
        Self {
            name: name.into(),
            bytes: content.as_bytes().to_vec(),
        }
    }
}
pub fn document(namespace: &str, body: &str) -> String {
    format!("<?xml version='1.0' encoding='UTF-8'?><w:document xmlns:w='{namespace}'><w:body>{body}</w:body></w:document>")
}
pub fn relationships(body: &str) -> String {
    format!("<Relationships xmlns='http://schemas.openxmlformats.org/package/2006/relationships'>{body}</Relationships>")
}
pub fn normal(main: &str, extras: &[Part]) -> Vec<u8> {
    package(
        "word/document.xml",
        main,
        "http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument",
        extras,
    )
}
pub fn package(path: &str, main: &str, relation: &str, extras: &[Part]) -> Vec<u8> {
    let types = format!("<Types xmlns='http://schemas.openxmlformats.org/package/2006/content-types'><Default Extension='rels' ContentType='application/vnd.openxmlformats-package.relationships+xml'/><Default Extension='xml' ContentType='application/xml'/><Default Extension='bin' ContentType='application/octet-stream'/><Override PartName='/{path}' ContentType='application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml'/></Types>");
    let relations = relationships(&format!(
        "<Relationship Id='r1' Type='{relation}' Target='{path}'/>"
    ));
    let parts = [
        Part::text("[Content_Types].xml", &types),
        Part::text("_rels/.rels", &relations),
        Part::text(path, main),
    ];
    zip(parts.iter().chain(extras))
}
pub fn zip<'a>(parts: impl Iterator<Item = &'a Part>) -> Vec<u8> {
    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    for part in parts {
        writer
            .start_file(
                &part.name,
                SimpleFileOptions::default().compression_method(CompressionMethod::Deflated),
            )
            .unwrap();
        writer.write_all(&part.bytes).unwrap();
    }
    writer.finish().unwrap().into_inner()
}
pub fn signature(bytes: &[u8], magic: &[u8]) -> usize {
    bytes
        .windows(magic.len())
        .position(|window| window == magic)
        .unwrap()
}
