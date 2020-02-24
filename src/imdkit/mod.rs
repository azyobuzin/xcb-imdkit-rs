use crate::ffi;
use std::ptr::NonNull;

mod im_message;
mod im_server;

pub use self::im_message::*;
pub use self::im_server::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ImClient(NonNull<ffi::xcb_im_client_t>);

impl ImClient {
    pub fn as_ptr(&self) -> *mut ffi::xcb_im_client_t {
        self.0.as_ptr()
    }
}

unsafe impl Send for ImClient {}
unsafe impl Sync for ImClient {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct InputContext(NonNull<ffi::xcb_im_input_context_t>);

impl InputContext {
    pub fn as_ptr(&self) -> *mut ffi::xcb_im_input_context_t {
        self.0.as_ptr()
    }
}

unsafe impl Send for InputContext {}
unsafe impl Sync for InputContext {}
