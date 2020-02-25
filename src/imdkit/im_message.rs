use super::{ImClient, InputContext};
use crate::ffi;
use std::os::raw::c_void;

//#[derive(Debug)]
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

//#[derive(Debug)]
#[non_exhaustive]
pub enum ImMessage<'a> {
    Connect {
        client: &'a ImClient,
        frame: &'a ffi::xcb_im_connect_fr_t,
    },
    Disconnect {
        client: &'a ImClient,
    },
    Open {
        client: &'a ImClient,
        frame: &'a ffi::xcb_im_open_fr_t,
    },
    Close {
        client: &'a ImClient,
        frame: &'a ffi::xcb_im_close_fr_t,
    },
    CreateIc {
        client: &'a ImClient,
        ic: &'a InputContext,
        frame: &'a ffi::xcb_im_create_ic_fr_t,
        reply_frame: &'a mut ffi::xcb_im_create_ic_reply_fr_t,
    },
    SetIcValues {
        client: &'a ImClient,
        ic: &'a InputContext,
        frame: &'a ffi::xcb_im_set_ic_values_fr_t,
    },
    GetIcValues {
        client: &'a ImClient,
        ic: &'a InputContext,
        frame: &'a ffi::xcb_im_get_ic_values_fr_t,
    },
    SetIcFocus {
        client: &'a ImClient,
        ic: &'a InputContext,
        frame: &'a ffi::xcb_im_set_ic_focus_fr_t,
    },
    UnsetIcFocus {
        client: &'a ImClient,
        ic: &'a InputContext,
        frame: &'a ffi::xcb_im_unset_ic_focus_fr_t,
    },
    DestroyIc {
        client: &'a ImClient,
        ic: &'a InputContext,
    },
    ResetIc {
        client: &'a ImClient,
        ic: &'a InputContext,
        frame: &'a ffi::xcb_im_reset_ic_fr_t,
        // TODO: committed_string は preedit_string の間違いだと思う
        // TODO: 文字列をコールバックから返すには
        reply_frame: &'a mut ffi::xcb_im_reset_ic_reply_fr_t,
    },
    ForwardEvent {
        client: &'a ImClient,
        ic: &'a InputContext,
        frame: &'a ffi::xcb_im_forward_event_fr_t,
        // TODO: KeyPressEvent without dropping
        key_event: &'a xcb::ffi::xcb_key_press_event_t,
    },
    ExtForwardKeyevent {
        client: &'a ImClient,
        ic: &'a InputContext,
        frame: &'a ffi::xcb_im_ext_forward_keyevent_fr_t,
        key_event: &'a xcb::ffi::xcb_key_press_event_t,
    },
    SyncReply {
        client: &'a ImClient,
        ic: &'a InputContext,
        frame: &'a ffi::xcb_im_sync_reply_fr_t,
    },
    TriggerNotify {
        client: &'a ImClient,
        ic: &'a InputContext,
        frame: &'a ffi::xcb_im_trigger_notify_fr_t,
    },
    PreeditStartReply {
        client: &'a ImClient,
        ic: &'a InputContext,
        frame: &'a ffi::xcb_im_preedit_start_reply_fr_t,
    },
    PreeditCaretReply {
        client: &'a ImClient,
        ic: &'a InputContext,
        frame: &'a ffi::xcb_im_preedit_caret_reply_fr_t,
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
            ffi::XCB_XIM_CONNECT => Connect {
                client: raw.client.unwrap(),
                frame: &*(raw.frame as *const _),
            },
            ffi::XCB_XIM_DISCONNECT => Disconnect {
                client: raw.client.unwrap(),
            },
            ffi::XCB_XIM_OPEN => Open {
                client: raw.client.unwrap(),
                frame: &*(raw.frame as *const _),
            },
            ffi::XCB_XIM_CLOSE => Close {
                client: raw.client.unwrap(),
                frame: &*(raw.frame as *const _),
            },
            ffi::XCB_XIM_CREATE_IC => CreateIc {
                client: raw.client.unwrap(),
                ic: raw.ic.unwrap(),
                frame: &*(raw.frame as *const _),
                reply_frame: &mut *(raw.arg as *mut _),
            },
            ffi::XCB_XIM_SET_IC_VALUES => SetIcValues {
                client: raw.client.unwrap(),
                ic: raw.ic.unwrap(),
                frame: &*(raw.frame as *const _),
            },
            ffi::XCB_XIM_GET_IC_VALUES => GetIcValues {
                client: raw.client.unwrap(),
                ic: raw.ic.unwrap(),
                frame: &*(raw.frame as *const _),
            },
            ffi::XCB_XIM_SET_IC_FOCUS => SetIcFocus {
                client: raw.client.unwrap(),
                ic: raw.ic.unwrap(),
                frame: &*(raw.frame as *const _),
            },
            ffi::XCB_XIM_UNSET_IC_FOCUS => UnsetIcFocus {
                client: raw.client.unwrap(),
                ic: raw.ic.unwrap(),
                frame: &*(raw.frame as *const _),
            },
            ffi::XCB_XIM_DESTROY_IC => DestroyIc {
                client: raw.client.unwrap(),
                ic: raw.ic.unwrap(),
            },
            ffi::XCB_XIM_RESET_IC => ResetIc {
                client: raw.client.unwrap(),
                ic: raw.ic.unwrap(),
                frame: &*(raw.frame as *const _),
                reply_frame: &mut *(raw.frame as *mut _),
            },
            ffi::XCB_XIM_FORWARD_EVENT => ForwardEvent {
                client: raw.client.unwrap(),
                ic: raw.ic.unwrap(),
                frame: &*(raw.frame as *const _),
                key_event: &*(raw.arg as *const _),
            },
            ffi::XCB_XIM_EXTENSION => match minor_opcode as u32 {
                ffi::XCB_XIM_EXT_FORWARD_KEYEVENT => ExtForwardKeyevent {
                    client: raw.client.unwrap(),
                    ic: raw.ic.unwrap(),
                    frame: &*(raw.frame as *const _),
                    key_event: &*(raw.arg as *const _),
                },
                _ => __Unsupported,
            },
            ffi::XCB_XIM_SYNC_REPLY => SyncReply {
                client: raw.client.unwrap(),
                ic: raw.ic.unwrap(),
                frame: &*(raw.frame as *const _),
            },
            ffi::XCB_XIM_TRIGGER_NOTIFY => TriggerNotify {
                client: raw.client.unwrap(),
                ic: raw.ic.unwrap(),
                frame: &*(raw.frame as *const _),
            },
            ffi::XCB_XIM_PREEDIT_START_REPLY => PreeditStartReply {
                client: raw.client.unwrap(),
                ic: raw.ic.unwrap(),
                frame: &*(raw.frame as *const _),
            },
            ffi::XCB_XIM_PREEDIT_CARET_REPLY => PreeditCaretReply {
                client: raw.client.unwrap(),
                ic: raw.ic.unwrap(),
                frame: &*(raw.frame as *const _),
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
