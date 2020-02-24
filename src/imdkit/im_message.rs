use super::{ImClient, InputContext};
use crate::ffi;
use std::os::raw::c_void;

#[derive(Debug)]
pub struct CallbackArgs<'a> {
    pub major_opcode: u8,
    pub minor_opcode: u8,
    pub parsed: ImMessage<'a>,
    pub raw: &'a RawCallbackArgs<'a>,
}

#[derive(Debug, Clone, Copy)]
pub struct RawCallbackArgs<'a> {
    pub client: Option<&'a ImClient>,
    pub ic: Option<&'a InputContext>,
    pub hdr: *const ffi::xcb_im_packet_header_fr_t,
    pub frame: *mut c_void,
    pub arg: *mut c_void,
}

#[derive(Debug)]
#[non_exhaustive]
pub enum ImMessage<'a> {
    CreateIc {
        client: &'a ImClient,
        ic: &'a InputContext,
        frame: &'a ffi::xcb_im_create_ic_fr_t,
        reply_frame: &'a mut ffi::xcb_im_create_ic_reply_fr_t,
    },
    DestroyIc {
        client: &'a ImClient,
        ic: &'a InputContext,
    },
    #[doc(hidden)]
    __Unsupported,
}

pub(crate) fn parse_callback_args<'a>(raw: &'a RawCallbackArgs) -> CallbackArgs<'a> {
    let (major_opcode, minor_opcode) = unsafe {
        let hdr = &*raw.hdr;
        (hdr.major_opcode, hdr.minor_opcode)
    };

    let parsed = unsafe {
        use ImMessage::*;
        match major_opcode as u32 {
            ffi::XCB_XIM_CREATE_IC => CreateIc {
                client: raw.client.unwrap(),
                ic: raw.ic.unwrap(),
                frame: &*(raw.frame as *const ffi::xcb_im_create_ic_fr_t),
                reply_frame: &mut *(raw.arg as *mut ffi::xcb_im_create_ic_reply_fr_t),
            },
            ffi::XCB_XIM_DESTROY_IC => DestroyIc {
                client: raw.client.unwrap(),
                ic: raw.ic.unwrap(),
            },
            _ => __Unsupported,
        }
    };

    CallbackArgs {
        major_opcode,
        minor_opcode,
        parsed,
        raw,
    }
}
