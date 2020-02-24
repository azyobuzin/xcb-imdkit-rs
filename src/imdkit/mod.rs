use crate::ffi;
use std::ffi::CStr;
use std::ptr::NonNull;

mod data_types;
mod im_message;
mod im_server;
mod input_context;

pub use self::data_types::*;
pub use self::im_message::*;
pub use self::im_server::*;
pub use self::input_context::*;

#[derive(Debug, PartialEq, Eq, Hash)]
pub struct ImClient(NonNull<ffi::xcb_im_client_t>);

impl ImClient {
    pub fn as_ptr(&self) -> *mut ffi::xcb_im_client_t {
        self.0.as_ptr()
    }

    pub fn as_ptr_non_null(&self) -> NonNull<ffi::xcb_im_client_t> {
        self.0
    }
}

pub fn all_locales() -> &'static CStr {
    CStr::from_bytes_with_nul(ffi::XCB_IM_ALL_LOCALES).unwrap()
}
