use super::{rejected, Object, Session};
use application::ApplicationError;
use std::collections::HashSet;

pub(super) fn validate(session: &Session<'_>) -> Result<(), ApplicationError> {
    // These accessors return handles owned by the same live native session.
    let (root, trailer) = unsafe {
        (
            (session.api.root)(session.data),
            (session.api.trailer)(session.data),
        )
    };
    let size = integer(session, session.key(trailer, c"/Size"))?;
    if size > 100_001 {
        return Err(ApplicationError::StageSupportValidationLimit);
    }
    if size <= 0 || !session.named(session.key(root, c"/Type"), c"/Catalog") {
        return Err(rejected());
    }
    let mut visited = HashSet::new();
    let pages = session.key(root, c"/Pages");
    if !session.named(session.key(pages, c"/Type"), c"/Pages") {
        return Err(rejected());
    }
    visit(session, pages, None, 0, &mut visited)?;
    session.clean()
}

fn visit(
    session: &Session<'_>,
    object: Object,
    parent: Option<(i32, i32)>,
    depth: usize,
    visited: &mut HashSet<(i32, i32)>,
) -> Result<usize, ApplicationError> {
    if depth > 128 || visited.len() >= 100_000 {
        return Err(ApplicationError::StageSupportValidationLimit);
    }
    let id = session.identity(object);
    // Predicate calls are safe for any object belonging to this session.
    if id.0 <= 0
        || !visited.insert(id)
        || unsafe { (session.api.dictionary)(session.data, object) } == 0
    {
        return Err(rejected());
    }
    let actual_parent = session.key(object, c"/Parent");
    if let Some(parent) = parent {
        if session.identity(actual_parent) != parent {
            return Err(rejected());
        }
    } else if unsafe { (session.api.null)(session.data, actual_parent) } == 0 {
        return Err(rejected());
    }
    let kind = session.key(object, c"/Type");
    let kids = session.key(object, c"/Kids");
    if session.named(kind, c"/Page") {
        if unsafe { (session.api.null)(session.data, kids) } == 0 {
            return Err(rejected());
        }
        return Ok(1);
    }
    if !session.named(kind, c"/Pages") || unsafe { (session.api.array)(session.data, kids) } == 0 {
        return Err(rejected());
    }
    let count = integer(session, session.key(object, c"/Count"))?;
    if count > 4096 {
        return Err(ApplicationError::StageSupportValidationLimit);
    }
    // The type check above establishes the precondition of the array accessors.
    let length = unsafe { (session.api.array_len)(session.data, kids) };
    if count <= 0 || length <= 0 {
        return Err(rejected());
    }
    if length > 100_000 {
        return Err(ApplicationError::StageSupportValidationLimit);
    }
    let mut pages = 0usize;
    for index in 0..length {
        // The index is within the validated array length.
        let child = unsafe { (session.api.array_item)(session.data, kids, index) };
        pages += visit(session, child, Some(id), depth + 1, visited)?;
        if pages > 4096 {
            return Err(ApplicationError::StageSupportValidationLimit);
        }
    }
    if pages as i64 != count {
        return Err(rejected());
    }
    Ok(pages)
}

fn integer(session: &Session<'_>, object: Object) -> Result<i64, ApplicationError> {
    // Validate type before requesting the native integer representation.
    if unsafe { (session.api.integer)(session.data, object) } == 0 {
        return Err(rejected());
    }
    Ok(unsafe { (session.api.integer_value)(session.data, object) })
}
