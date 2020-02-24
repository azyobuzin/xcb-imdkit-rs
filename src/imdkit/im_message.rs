use crate::ffi;
use std::os::raw::c_void;

#[derive(Debug)]
pub enum ImMessage<'a> {
    CreateIc(
        &'a ffi::xcb_im_create_ic_fr_t,
        &'a mut ffi::xcb_im_create_ic_reply_fr_t,
    ),
    DestroyIc,
    Other {
        hdr: *const ffi::xcb_im_packet_header_fr_t,
        frame: *mut c_void,
        arg: *mut c_void,
    },
}

pub fn parse_message<'a>(
    hdr: *const ffi::xcb_im_packet_header_fr_t,
    frame: *mut c_void,
    arg: *mut c_void,
) -> ImMessage<'a> {
    use ImMessage::*;
    unsafe {
        match (*hdr).major_opcode as u32 {
            ffi::XCB_XIM_CREATE_IC => CreateIc(
                &*(frame as *const ffi::xcb_im_create_ic_fr_t),
                &mut *(arg as *mut ffi::xcb_im_create_ic_reply_fr_t),
            ),
            ffi::XCB_XIM_DESTROY_IC => DestroyIc,
            _ => Other { hdr, frame, arg },
        }
    }
}
