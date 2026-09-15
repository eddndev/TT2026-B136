//! PDF structure admission with recovery disabled and warnings rejected.

mod api;
mod tree;

use api::{Api, Data, Object};
use application::ApplicationError;
use std::{ffi::CStr, path::Path};

/// Reviewed native PDF parser, to be used only inside the isolated worker.
pub(super) struct PdfLibrary {
    api: Api,
}

impl PdfLibrary {
    /// Loads the exact native version from an operator-supplied absolute path.
    pub(super) fn open(path: &Path) -> Result<Self, ApplicationError> {
        Ok(Self {
            api: Api::open(path)?,
        })
    }

    /// Checks structure without rewriting the file or interpreting legal meaning.
    /// Resource limits must surround this call in the format worker process.
    pub(super) fn validate(&self, bytes: &[u8]) -> Result<(), ApplicationError> {
        if bytes.len() > 16 * 1024 * 1024 {
            return Err(ApplicationError::StageSupportTooLarge);
        }
        let session = Session::new(&self.api)?;
        // Bytes and the static description remain alive until session cleanup.
        let status = unsafe {
            (self.api.read)(
                session.data,
                c"stage-support".as_ptr(),
                bytes.as_ptr().cast(),
                bytes.len() as u64,
                std::ptr::null(),
            )
        };
        if status != 0 {
            return Err(rejected());
        }
        session.clean()?;
        // A successful empty-password decryption still counts as encrypted input.
        if unsafe { (self.api.encrypted)(session.data) } != 0 {
            return Err(rejected());
        }
        tree::validate(&session)?;
        // The C API catches parser errors; queued warnings are rejected as well.
        if unsafe { (self.api.check)(session.data) } != 0 {
            return Err(rejected());
        }
        session.clean()
    }
}

struct Session<'a> {
    api: &'a Api,
    data: Data,
}

impl<'a> Session<'a> {
    fn new(api: &'a Api) -> Result<Self, ApplicationError> {
        // A fresh native handle belongs exclusively to this session and thread.
        let data = unsafe { (api.init)() };
        if data.is_null() {
            return Err(ApplicationError::StageSupportValidationLimit);
        }
        let session = Self { api, data };
        // Configuration is set before parsing; errors never include document text.
        unsafe {
            (api.silence)(data);
            (api.suppress)(data, 1);
            (api.recovery)(data, 0);
        }
        session.clean()?;
        Ok(session)
    }

    fn clean(&self) -> Result<(), ApplicationError> {
        // The handle is live; inspecting flags does not consume the native error.
        let dirty =
            unsafe { (self.api.error)(self.data) != 0 || (self.api.warnings)(self.data) != 0 };
        if dirty {
            Err(rejected())
        } else {
            Ok(())
        }
    }

    fn key(&self, object: Object, key: &CStr) -> Object {
        // All handles are obtained from this same live session; keys are C strings.
        unsafe { (self.api.key)(self.data, object, key.as_ptr()) }
    }

    fn identity(&self, object: Object) -> (i32, i32) {
        // Both accessors inspect a handle from the same native object cache.
        unsafe {
            (
                (self.api.object_id)(self.data, object),
                (self.api.generation)(self.data, object),
            )
        }
    }

    fn named(&self, object: Object, name: &CStr) -> bool {
        // The name comparison accepts any valid native object handle.
        unsafe { (self.api.name_equals)(self.data, object, name.as_ptr()) != 0 }
    }
}

impl Drop for Session<'_> {
    fn drop(&mut self) {
        // Consume any opaque error to prevent native cleanup from printing it.
        // Cleanup owns every object handle and runs before the library unloads.
        unsafe {
            (self.api.clear_error)(self.data);
            (self.api.cleanup)(&mut self.data);
        }
    }
}

fn rejected() -> ApplicationError {
    ApplicationError::StageSupportFormatRejected
}

#[cfg(test)]
mod tests;
