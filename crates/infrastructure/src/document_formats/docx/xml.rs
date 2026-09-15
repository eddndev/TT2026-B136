use super::{
    budget::DocxBudget,
    rejected,
    xml_names::{legal_character, ncname, qname, whitespace},
    Error, Result,
};
use quick_xml::{
    events::{BytesStart, Event},
    name::ResolveResult,
    reader::NsReader,
    XmlVersion,
};
use std::collections::HashSet;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(super) struct Name {
    pub ns: String,
    pub local: String,
}
pub(super) struct Element {
    pub name: Name,
    pub attrs: Vec<(Name, String)>,
}
impl Element {
    pub fn attribute(&self, key: &str) -> Result<&str> {
        self.attrs
            .iter()
            .find(|(name, _)| name.ns.is_empty() && name.local == key)
            .map(|(_, value)| value.as_str())
            .ok_or(Error::StageSupportFormatRejected)
    }
    pub fn optional(&self, key: &str) -> Option<&str> {
        self.attrs
            .iter()
            .find(|(name, _)| name.ns.is_empty() && name.local == key)
            .map(|(_, value)| value.as_str())
    }
    pub fn only_attributes(&self, names: &[&str]) -> Result<()> {
        if self
            .attrs
            .iter()
            .any(|(name, _)| !name.ns.is_empty() || !names.contains(&name.local.as_str()))
        {
            return rejected();
        }
        Ok(())
    }
}

pub(super) fn parse(
    bytes: &[u8],
    budget: &mut DocxBudget,
    only_whitespace: bool,
    mut visit: impl FnMut(&Element, usize) -> Result<()>,
) -> Result<Name> {
    let raw = std::str::from_utf8(bytes).map_err(|_| Error::StageSupportFormatRejected)?;
    let raw = raw.strip_prefix('\u{feff}').unwrap_or(raw);
    if !raw.chars().all(legal_character) {
        return rejected();
    }
    let mut reader = NsReader::from_str(raw);
    reader.config_mut().check_end_names = true;
    reader.config_mut().check_comments = true;
    reader.config_mut().allow_dangling_amp = false;
    reader.config_mut().allow_unmatched_ends = false;
    let mut depth = 0usize;
    let mut root = None;
    let mut first = true;
    loop {
        let event = reader
            .read_event()
            .map_err(|_| Error::StageSupportFormatRejected)?;
        budget.event()?;
        match event {
            Event::Start(ref start) | Event::Empty(ref start) => {
                let empty = matches!(event, Event::Empty(_));
                if depth == 128 {
                    return Err(Error::StageSupportValidationLimit);
                }
                qname(start.name().as_ref())?;
                if start.name().as_ref().starts_with("xmlns:") {
                    return rejected();
                }
                let (ns, local) = reader.resolver().resolve_element(start.name());
                let name = Name {
                    ns: namespace(ns)?,
                    local: local.as_ref().into(),
                };
                if depth == 0 {
                    if root.is_some() {
                        return rejected();
                    }
                    root = Some(name.clone());
                }
                let attrs = attributes(start, &reader)?;
                visit(&Element { name, attrs }, depth + 1)?;
                if !empty {
                    depth += 1;
                }
            }
            Event::End(_) => {
                depth = depth
                    .checked_sub(1)
                    .ok_or(Error::StageSupportFormatRejected)?;
            }
            Event::Text(text) => {
                if text.contains("]]>") || ((depth == 0 || only_whitespace) && !whitespace(&text)) {
                    return rejected();
                }
            }
            Event::CData(text) => {
                if depth == 0 || (only_whitespace && !whitespace(&text)) {
                    return rejected();
                }
            }
            Event::GeneralRef(reference) => {
                if depth == 0 || only_whitespace {
                    return rejected();
                }
                if let Some(c) = reference
                    .resolve_char_ref()
                    .map_err(|_| Error::StageSupportFormatRejected)?
                {
                    if !legal_character(c) {
                        return rejected();
                    }
                } else if !["lt", "gt", "amp", "apos", "quot"].contains(&reference.as_ref()) {
                    return rejected();
                }
            }
            Event::Decl(decl) => {
                if !first
                    || decl
                        .version()
                        .map_err(|_| Error::StageSupportFormatRejected)?
                        != "1.0"
                {
                    return rejected();
                }
                let declaration = BytesStart::from_content(decl.as_ref(), 3);
                let mut position = 0;
                for attr in declaration.attributes() {
                    let attr = attr.map_err(|_| Error::StageSupportFormatRejected)?;
                    let current = match attr.key.as_ref() {
                        "version" => 1,
                        "encoding" => 2,
                        "standalone" => 3,
                        _ => return rejected(),
                    };
                    if current <= position {
                        return rejected();
                    }
                    position = current;
                    let value = attr.value.as_ref();
                    if (current == 1 && value != "1.0")
                        || (current == 2 && !value.eq_ignore_ascii_case("UTF-8"))
                        || (current == 3 && value != "yes" && value != "no")
                    {
                        return rejected();
                    }
                }
            }
            Event::PI(pi) => {
                ncname(pi.target())?;
                if pi.target().eq_ignore_ascii_case("xml") {
                    return rejected();
                }
            }
            Event::DocType(_) => return rejected(),
            Event::Eof => {
                if depth != 0 {
                    return rejected();
                }
                return root.ok_or(Error::StageSupportFormatRejected);
            }
            Event::Comment(_) => {}
        }
        first = false;
    }
}

fn namespace(ns: ResolveResult<'_>) -> Result<String> {
    match ns {
        ResolveResult::Unbound => Ok(String::new()),
        ResolveResult::Unknown(_) => rejected(),
        ResolveResult::Bound(ns) => {
            let normalized = quick_xml::escape::unescape(ns.as_ref())
                .map_err(|_| Error::StageSupportFormatRejected)?;
            if !normalized.chars().all(legal_character)
                || normalized.chars().any(char::is_whitespace)
            {
                return rejected();
            }
            Ok(normalized.into_owned())
        }
    }
}
fn attributes(start: &BytesStart<'_>, reader: &NsReader<&[u8]>) -> Result<Vec<(Name, String)>> {
    let mut output = Vec::new();
    let mut names = HashSet::new();
    for attr in start.attributes() {
        let attr = attr.map_err(|_| Error::StageSupportFormatRejected)?;
        qname(attr.key.as_ref())?;
        if attr.value.contains('<') {
            return rejected();
        }
        let value = attr
            .normalized_value(XmlVersion::Implicit1_0)
            .map_err(|_| Error::StageSupportFormatRejected)?;
        if !value.chars().all(legal_character) {
            return rejected();
        }
        let raw_name = attr.key.as_ref();
        if raw_name == "xmlns" || raw_name.starts_with("xmlns:") {
            const XML: &str = "http://www.w3.org/XML/1998/namespace";
            const XMLNS: &str = "http://www.w3.org/2000/xmlns/";
            if value.chars().any(char::is_whitespace)
                || raw_name == "xmlns:xmlns"
                || value == XMLNS
                || (raw_name == "xmlns:xml" && value != XML)
                || (raw_name != "xmlns:xml" && value == XML)
                || (raw_name.starts_with("xmlns:") && value.is_empty())
            {
                return rejected();
            }
            continue;
        }
        let (ns, local) = reader.resolver().resolve_attribute(attr.key);
        let name = Name {
            ns: namespace(ns)?,
            local: local.as_ref().into(),
        };
        if !names.insert(name.clone()) {
            return rejected();
        }
        output.push((name, value.into_owned()));
    }
    Ok(output)
}
