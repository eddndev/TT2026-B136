//! Restricted DOCX package admission inside the isolated format worker.

mod budget;
mod content_types;
mod inflate;
mod paths;
mod relationships;
#[cfg(test)]
mod tests;
mod xml;
mod xml_names;
mod zip_layout;

use application::ApplicationError as Error;
pub(crate) use budget::DocxBudget;
use content_types::{ContentTypes, MAIN, RELS};
use std::{collections::HashMap, io::Cursor};
type Result<T> = std::result::Result<T, Error>;
fn rejected<T>() -> Result<T> {
    Err(Error::StageSupportFormatRejected)
}

pub(crate) fn validate_docx(bytes: &[u8], budget: &mut DocxBudget) -> Result<()> {
    let entries = zip_layout::inspect(bytes)?;
    let parts: HashMap<_, _> = entries
        .iter()
        .enumerate()
        .filter(|(_, e)| !e.directory)
        .map(|(i, e)| (e.key.clone(), i))
        .collect();
    let types_index = *parts
        .get("[content_types].xml")
        .ok_or(Error::StageSupportFormatRejected)?;
    if entries[types_index].name != "[Content_Types].xml" || !parts.contains_key("_rels/.rels") {
        return rejected();
    }
    let archive =
        zip::ZipArchive::new(Cursor::new(bytes)).map_err(|_| Error::StageSupportFormatRejected)?;
    if archive.len() != entries.len() {
        return rejected();
    }
    if archive
        .file_names()
        .zip(&entries)
        .any(|(actual, entry)| actual != entry.name)
    {
        return rejected();
    }
    let types_bytes = inflate::read_part(bytes, &entries[types_index], budget)?;
    let types = ContentTypes::parse(&types_bytes, budget, &parts)?;
    drop(types_bytes);
    let mut main = None;
    let mut roots = HashMap::new();
    let mut family = None;
    for (index, entry) in entries.iter().enumerate() {
        if index == types_index {
            continue;
        }
        let content = inflate::read_part(bytes, entry, budget)?;
        if entry.directory {
            continue;
        }
        if banned_name(&entry.key) || content.starts_with(b"\xd0\xcf\x11\xe0\xa1\xb1\x1a\xe1") {
            return rejected();
        }
        let kind = types.get(&entry.key)?;
        if content_types::banned_type(kind) {
            return rejected();
        }
        if kind == RELS || entry.key.ends_with(".rels") {
            if kind != RELS {
                return rejected();
            }
            let source = relationships::source(&entry.key, &parts)?;
            if let Some(found) =
                relationships::validate(&content, budget, &source, &parts, &mut family)?
            {
                if main.replace(found).is_some() {
                    return rejected();
                }
            }
        } else if kind.ends_with("+xml")
            || kind == "application/xml"
            || kind == "text/xml"
            || entry.key.ends_with(".xml")
        {
            let mut bodies = 0;
            let mut root_family = None;
            let root = xml::parse(&content, budget, false, |element, depth| {
                if depth == 1 {
                    root_family = word_namespace(&element.name.ns);
                }
                if let Some(element_family) = word_namespace(&element.name.ns) {
                    if root_family.is_some_and(|root| root != element_family) {
                        return rejected();
                    }
                    if ["object", "control", "altChunk"].contains(&element.name.local.as_str()) {
                        return rejected();
                    }
                    if depth == 2 && element.name.local == "body" {
                        bodies += 1;
                    }
                }
                Ok(())
            })?;
            roots.insert(entry.key.clone(), (root, bodies));
        } else if content.starts_with(b"PK\x03\x04") {
            return rejected();
        }
    }
    let (main, strict) = main.ok_or(Error::StageSupportFormatRejected)?;
    let (root, bodies) = roots.get(&main).ok_or(Error::StageSupportFormatRejected)?;
    if roots
        .values()
        .any(|(name, _)| word_namespace(&name.ns).is_some_and(|family| family != strict))
    {
        return rejected();
    }
    if types.get(&main)? != MAIN
        || root.local != "document"
        || word_namespace(&root.ns) != Some(strict)
        || *bodies != 1
    {
        return rejected();
    }
    Ok(())
}
fn word_namespace(value: &str) -> Option<bool> {
    match value {
        "http://schemas.openxmlformats.org/wordprocessingml/2006/main" => Some(false),
        "http://purl.oclc.org/ooxml/wordprocessingml/main" => Some(true),
        _ => None,
    }
}
fn banned_name(name: &str) -> bool {
    name.split('/')
        .any(|part| ["embeddings", "activex"].contains(&part))
        || name.ends_with("vbaproject.bin")
        || name.ends_with("vbadata.xml")
}
