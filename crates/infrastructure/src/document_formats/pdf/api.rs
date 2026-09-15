use application::ApplicationError;
use libloading::Library;
use std::{
    ffi::{c_char, c_int, c_uint, c_void, CStr},
    path::Path,
};

pub(super) type Data = *mut c_void;
pub(super) type Object = c_uint;

macro_rules! api {
    ($($field:ident: $ty:ty = $symbol:literal),+ $(,)?) => {
        pub(super) struct Api {
            $(pub $field: $ty,)+
            _library: Library,
        }
        impl Api {
            pub fn open(path: &Path) -> Result<Self, ApplicationError> {
                if !path.is_absolute() {
                    return Err(configuration());
                }
                // The path belongs to operator configuration, never document input.
                let library = unsafe { Library::new(path) }.map_err(|_| configuration())?;
                $(
                    // Each fixed signature matches the supported qpdf C ABI. The
                    // library stays alive for the full lifetime of these pointers.
                    let $field = unsafe {
                        *library.get::<$ty>($symbol).map_err(|_| configuration())?
                    };
                )+
                let api = Self { $($field,)+ _library: library };
                // The C API guarantees a static, NUL-terminated version string.
                let version = unsafe { (api.version)() };
                if version.is_null() || unsafe { CStr::from_ptr(version) }.to_bytes() != b"12.4.1" {
                    return Err(configuration());
                }
                Ok(api)
            }
        }
    };
}

api! {
    version: unsafe extern "C" fn() -> *const c_char = b"qpdf_get_qpdf_version\0",
    init: unsafe extern "C" fn() -> Data = b"qpdf_init\0",
    cleanup: unsafe extern "C" fn(*mut Data) = b"qpdf_cleanup\0",
    silence: unsafe extern "C" fn(Data) = b"qpdf_silence_errors\0",
    suppress: unsafe extern "C" fn(Data, c_int) = b"qpdf_set_suppress_warnings\0",
    recovery: unsafe extern "C" fn(Data, c_int) = b"qpdf_set_attempt_recovery\0",
    read: unsafe extern "C" fn(Data, *const c_char, *const c_char, u64, *const c_char) -> c_int = b"qpdf_read_memory\0",
    check: unsafe extern "C" fn(Data) -> c_int = b"qpdf_check_pdf\0",
    error: unsafe extern "C" fn(Data) -> c_int = b"qpdf_has_error\0",
    clear_error: unsafe extern "C" fn(Data) -> Data = b"qpdf_get_error\0",
    warnings: unsafe extern "C" fn(Data) -> c_int = b"qpdf_more_warnings\0",
    encrypted: unsafe extern "C" fn(Data) -> c_int = b"qpdf_is_encrypted\0",
    root: unsafe extern "C" fn(Data) -> Object = b"qpdf_get_root\0",
    trailer: unsafe extern "C" fn(Data) -> Object = b"qpdf_get_trailer\0",
    key: unsafe extern "C" fn(Data, Object, *const c_char) -> Object = b"qpdf_oh_get_key\0",
    dictionary: unsafe extern "C" fn(Data, Object) -> c_int = b"qpdf_oh_is_dictionary\0",
    array: unsafe extern "C" fn(Data, Object) -> c_int = b"qpdf_oh_is_array\0",
    integer: unsafe extern "C" fn(Data, Object) -> c_int = b"qpdf_oh_is_integer\0",
    null: unsafe extern "C" fn(Data, Object) -> c_int = b"qpdf_oh_is_null\0",
    integer_value: unsafe extern "C" fn(Data, Object) -> i64 = b"qpdf_oh_get_int_value\0",
    name_equals: unsafe extern "C" fn(Data, Object, *const c_char) -> c_int = b"qpdf_oh_is_name_and_equals\0",
    array_len: unsafe extern "C" fn(Data, Object) -> c_int = b"qpdf_oh_get_array_n_items\0",
    array_item: unsafe extern "C" fn(Data, Object, c_int) -> Object = b"qpdf_oh_get_array_item\0",
    object_id: unsafe extern "C" fn(Data, Object) -> c_int = b"qpdf_oh_get_object_id\0",
    generation: unsafe extern "C" fn(Data, Object) -> c_int = b"qpdf_oh_get_generation\0",
}

fn configuration() -> ApplicationError {
    ApplicationError::InvalidConfiguration("qpdf 12.4.1 shared library is unavailable".into())
}
