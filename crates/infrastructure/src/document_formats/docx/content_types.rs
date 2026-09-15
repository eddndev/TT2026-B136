use super::{budget::DocxBudget, paths::part_name, rejected, xml, Result};
use std::collections::HashMap;

pub(super) const RELS: &str = "application/vnd.openxmlformats-package.relationships+xml";
pub(super) const MAIN: &str =
    "application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml";
const NS: &str = "http://schemas.openxmlformats.org/package/2006/content-types";
pub(super) struct ContentTypes {
    defaults: HashMap<String, String>,
    overrides: HashMap<String, String>,
}
impl ContentTypes {
    pub fn parse(
        bytes: &[u8],
        budget: &mut DocxBudget,
        parts: &HashMap<String, usize>,
    ) -> Result<Self> {
        let mut result = Self {
            defaults: HashMap::new(),
            overrides: HashMap::new(),
        };
        xml::parse(bytes, budget, true, |element, depth| {
            if element.name.ns != NS {
                return rejected();
            }
            if depth == 1 {
                if element.name.local != "Types" {
                    return rejected();
                }
                return element.only_attributes(&[]);
            }
            if depth != 2 {
                return rejected();
            }
            let value = element.attribute("ContentType")?;
            if !valid_type(value) || banned_type(value) {
                return rejected();
            }
            let (map, key) = match element.name.local.as_str() {
                "Default" => {
                    element.only_attributes(&["Extension", "ContentType"])?;
                    let extension = element.attribute("Extension")?;
                    if extension.is_empty()
                        || !extension
                            .bytes()
                            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
                    {
                        return rejected();
                    }
                    (&mut result.defaults, extension.to_ascii_lowercase())
                }
                "Override" => {
                    element.only_attributes(&["PartName", "ContentType"])?;
                    let name = element
                        .attribute("PartName")?
                        .strip_prefix('/')
                        .ok_or(super::Error::StageSupportFormatRejected)?;
                    let name = part_name(name)?;
                    if !parts.contains_key(&name) || name == "[content_types].xml" {
                        return rejected();
                    }
                    (&mut result.overrides, name)
                }
                _ => return rejected(),
            };
            if map.insert(key, value.to_ascii_lowercase()).is_some() {
                return rejected();
            }
            Ok(())
        })?;
        Ok(result)
    }
    pub fn get(&self, part: &str) -> Result<&str> {
        self.overrides
            .get(part)
            .or_else(|| {
                part.rsplit_once('.')
                    .and_then(|(_, ext)| self.defaults.get(ext))
            })
            .map(String::as_str)
            .ok_or(super::Error::StageSupportFormatRejected)
    }
}
fn valid_type(value: &str) -> bool {
    let Some((main, sub)) = value.split_once('/') else {
        return false;
    };
    !main.is_empty()
        && !sub.is_empty()
        && [main, sub].iter().all(|part| {
            part.bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"!#$&^_.+-".contains(&b))
        })
}
pub(super) fn banned_type(value: &str) -> bool {
    let value = value.to_ascii_lowercase();
    [
        "macroenabled",
        "vbaproject",
        "activex",
        "oleobject",
        "ms-office.active",
        "ms-package",
        "wordprocessingml.template",
        "text/html",
    ]
    .iter()
    .any(|part| value.contains(part))
}
