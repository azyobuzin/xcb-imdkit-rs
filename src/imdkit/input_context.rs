use crate::ffi;
use super::data_types::*;
use std::ptr::NonNull;
use xcb;

#[derive(Debug, PartialEq, Eq, Hash)]
pub struct InputContext(pub(crate) NonNull<ffi::xcb_im_input_context_t>);

impl InputContext {
    pub fn get_input_style(&self) -> InputStyle {
        InputStyle::from_bits(unsafe {
            ffi::xcb_im_input_context_get_input_style(self.as_ptr())
        }).expect("Unexpected input style")
    }

    pub fn get_client_window(&self) -> xcb::Window {
        unsafe { ffi::xcb_im_input_context_get_client_window(self.as_ptr()) }
    }

    pub fn get_preedit_attr(&self) -> &PreeditAttr {
        unsafe { &*ffi::xcb_im_input_context_get_preedit_attr(self.as_ptr()) }
    }

    pub fn get_status_attr(&self) -> &StatusAttr {
        unsafe { &*ffi::xcb_im_input_context_get_status_attr(self.as_ptr()) }
    }

    pub fn as_ptr(&self) -> *mut ffi::xcb_im_input_context_t {
        self.0.as_ptr()
    }

    pub fn as_ptr_non_null(&self) -> NonNull<ffi::xcb_im_input_context_t> {
        self.0
    }
}
