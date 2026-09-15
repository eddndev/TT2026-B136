use super::{budget::DocxBudget, paths, rejected, xml, xml_names::ncname, Error, Result};
use std::collections::{HashMap, HashSet};
const NS: &str = "http://schemas.openxmlformats.org/package/2006/relationships";
pub(super) const TRANSITIONAL: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/";
pub(super) const STRICT: &str = "http://purl.oclc.org/ooxml/officeDocument/relationships/";

pub(super) fn source(name: &str, parts: &HashMap<String, usize>) -> Result<String> {
    if name == "_rels/.rels" {
        return Ok(String::new());
    }
    let (parent, file) = name
        .rsplit_once('/')
        .ok_or(Error::StageSupportFormatRejected)?;
    let parent = parent
        .strip_suffix("_rels")
        .ok_or(Error::StageSupportFormatRejected)?;
    if !parent.is_empty() && !parent.ends_with('/') {
        return rejected();
    }
    let file = file
        .strip_suffix(".rels")
        .ok_or(Error::StageSupportFormatRejected)?;
    let source = format!("{parent}{file}");
    if !parts.contains_key(&source) || source.ends_with(".rels") {
        return rejected();
    }
    Ok(source)
}

pub(super) fn validate(
    bytes: &[u8],
    budget: &mut DocxBudget,
    source: &str,
    parts: &HashMap<String, usize>,
    family: &mut Option<bool>,
) -> Result<Option<(String, bool)>> {
    let mut identifiers = HashSet::new();
    let mut main = None;
    xml::parse(bytes, budget, true, |element, depth| {
        if element.name.ns != NS {
            return rejected();
        }
        if depth == 1 {
            if element.name.local != "Relationships" {
                return rejected();
            }
            return element.only_attributes(&[]);
        }
        if depth != 2 || element.name.local != "Relationship" {
            return rejected();
        }
        element.only_attributes(&["Id", "Type", "Target", "TargetMode"])?;
        let id = element.attribute("Id")?;
        ncname(id)?;
        if !identifiers.insert(id.to_string()) {
            return rejected();
        }
        let kind = element.attribute("Type")?;
        if !absolute_uri(kind) {
            return rejected();
        }
        if kind.starts_with(TRANSITIONAL) || kind.starts_with(STRICT) {
            let strict = kind.starts_with(STRICT);
            if family.is_some_and(|previous| previous != strict) {
                return rejected();
            }
            *family = Some(strict);
        }
        let target = element.attribute("Target")?;
        let suffix = kind
            .rsplit('/')
            .next()
            .unwrap_or_default()
            .to_ascii_lowercase();
        if [
            "vbaproject",
            "vbadata",
            "oleobject",
            "package",
            "control",
            "activexcontrolbinary",
            "attachedtemplate",
            "afchunk",
        ]
        .contains(&suffix.as_str())
        {
            return rejected();
        }
        let external = match element.optional("TargetMode") {
            None | Some("Internal") => false,
            Some("External") => true,
            _ => return rejected(),
        };
        if external {
            if kind != format!("{TRANSITIONAL}hyperlink") && kind != format!("{STRICT}hyperlink") {
                return rejected();
            }
            if !["http://", "https://", "mailto:"]
                .iter()
                .any(|prefix| target.starts_with(prefix))
                || !absolute_uri(target)
            {
                return rejected();
            }
            return Ok(());
        }
        let key = paths::target(source, target)?;
        if !parts.contains_key(&key) || key == "[content_types].xml" || key.ends_with(".rels") {
            return rejected();
        }
        if kind == format!("{TRANSITIONAL}officeDocument")
            || kind == format!("{STRICT}officeDocument")
        {
            if !source.is_empty() || main.is_some() {
                return rejected();
            }
            main = Some((key, kind.starts_with(STRICT)));
        }
        Ok(())
    })?;
    Ok(main)
}
fn absolute_uri(value: &str) -> bool {
    let Some((scheme, rest)) = value.split_once(':') else {
        return false;
    };
    !rest.is_empty()
        && scheme.starts_with(|c: char| c.is_ascii_alphabetic())
        && scheme
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"+-.".contains(&b))
        && value
            .bytes()
            .all(|b| b > 32 && b < 127 && !b"<>\\\"{}|^`".contains(&b))
}
