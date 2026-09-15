use super::{rejected, Result};

pub(super) fn legal_character(c: char) -> bool {
    matches!(c as u32, 9 | 10 | 13 | 0x20..=0xd7ff | 0xe000..=0xfffd | 0x10000..=0x10ffff)
}
fn initial(c: char) -> bool {
    matches!(c as u32, 0x41..=0x5a | 0x5f | 0x61..=0x7a | 0xc0..=0xd6 | 0xd8..=0xf6
        | 0xf8..=0x2ff | 0x370..=0x37d | 0x37f..=0x1fff | 0x200c..=0x200d
        | 0x2070..=0x218f | 0x2c00..=0x2fef | 0x3001..=0xd7ff | 0xf900..=0xfdcf
        | 0xfdf0..=0xfffd | 0x10000..=0xeffff)
}
pub(super) fn ncname(name: &str) -> Result<()> {
    let mut chars = name.chars();
    if !chars.next().is_some_and(initial) || !chars.all(|c| initial(c)
        || matches!(c as u32, 0x2d | 0x2e | 0x30..=0x39 | 0xb7 | 0x300..=0x36f | 0x203f..=0x2040)) {
        return rejected();
    }
    Ok(())
}
pub(super) fn qname(name: &str) -> Result<()> {
    let mut pieces = name.split(':');
    ncname(pieces.next().unwrap_or_default())?;
    if let Some(local) = pieces.next() {
        ncname(local)?;
    }
    if pieces.next().is_some() {
        return rejected();
    }
    Ok(())
}
pub(super) fn whitespace(value: &str) -> bool {
    value.chars().all(|c| matches!(c, ' ' | '\t' | '\n' | '\r'))
}
