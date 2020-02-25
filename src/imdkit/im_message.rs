use super::data_types::*;
use super::{slice_from_raw, ImClient, InputContext};
use crate::ffi::*;
use std::os::raw::c_void;

//#[derive(Debug)]
pub struct CallbackArgs<'a> {
    pub major_opcode: u8,
    pub minor_opcode: u8,
    pub parsed: ImMessage<'a>,
    pub raw: &'a RawCallbackArgs<'a>,
}

#[derive(Debug, Clone)]
pub struct RawCallbackArgs<'a> {
    pub client: Option<&'a ImClient>,
    pub ic: Option<&'a InputContext>,
    pub hdr: *const xcb_im_packet_header_fr_t,
    pub frame: *mut c_void,
    pub arg: *mut c_void,
}

// TODO: trait ImMessageHandler

//#[derive(Debug)]
#[non_exhaustive]
pub enum ImMessage<'a> {
    Connect {
        client: &'a ImClient,
        frame: &'a xcb_im_connect_fr_t,
    },
    Disconnect {
        client: &'a ImClient,
    },
    Open {
        client: &'a ImClient,
        frame: &'a xcb_im_open_fr_t,
    },
    Close {
        client: &'a ImClient,
        frame: &'a xcb_im_close_fr_t,
    },
    CreateIc {
        client: &'a ImClient,
        ic: &'a InputContext,
        frame: &'a xcb_im_create_ic_fr_t,
        reply_frame: &'a mut xcb_im_create_ic_reply_fr_t,
    },
    SetIcValues {
        client: &'a ImClient,
        ic: &'a InputContext,
        frame: &'a xcb_im_set_ic_values_fr_t,
    },
    GetIcValues {
        client: &'a ImClient,
        ic: &'a InputContext,
        frame: &'a xcb_im_get_ic_values_fr_t,
    },
    SetIcFocus {
        client: &'a ImClient,
        ic: &'a InputContext,
        frame: &'a xcb_im_set_ic_focus_fr_t,
    },
    UnsetIcFocus {
        client: &'a ImClient,
        ic: &'a InputContext,
        frame: &'a xcb_im_unset_ic_focus_fr_t,
    },
    DestroyIc {
        client: &'a ImClient,
        ic: &'a InputContext,
    },
    ResetIc {
        client: &'a ImClient,
        ic: &'a InputContext,
        frame: &'a xcb_im_reset_ic_fr_t,
        // TODO: committed_string は preedit_string の間違いだと思う
        // TODO: 文字列をコールバックから返すには
        reply_frame: &'a mut xcb_im_reset_ic_reply_fr_t,
    },
    ForwardEvent {
        client: &'a ImClient,
        ic: &'a InputContext,
        frame: &'a xcb_im_forward_event_fr_t,
        // TODO: KeyPressEvent without dropping
        key_event: &'a xcb::ffi::xcb_key_press_event_t,
    },
    ExtForwardKeyevent {
        client: &'a ImClient,
        ic: &'a InputContext,
        frame: &'a xcb_im_ext_forward_keyevent_fr_t,
        key_event: &'a xcb::ffi::xcb_key_press_event_t,
    },
    SyncReply {
        client: &'a ImClient,
        ic: &'a InputContext,
        frame: &'a xcb_im_sync_reply_fr_t,
    },
    TriggerNotify {
        client: &'a ImClient,
        ic: &'a InputContext,
        frame: &'a xcb_im_trigger_notify_fr_t,
    },
    PreeditStartReply {
        client: &'a ImClient,
        ic: &'a InputContext,
        frame: &'a xcb_im_preedit_start_reply_fr_t,
    },
    PreeditCaretReply {
        client: &'a ImClient,
        ic: &'a InputContext,
        frame: &'a xcb_im_preedit_caret_reply_fr_t,
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
            XCB_XIM_CONNECT => Connect {
                client: raw.client.unwrap(),
                frame: &*(raw.frame as *const _),
            },
            XCB_XIM_DISCONNECT => Disconnect {
                client: raw.client.unwrap(),
            },
            XCB_XIM_OPEN => Open {
                client: raw.client.unwrap(),
                frame: &*(raw.frame as *const _),
            },
            XCB_XIM_CLOSE => Close {
                client: raw.client.unwrap(),
                frame: &*(raw.frame as *const _),
            },
            XCB_XIM_CREATE_IC => CreateIc {
                client: raw.client.unwrap(),
                ic: raw.ic.unwrap(),
                frame: &*(raw.frame as *const _),
                reply_frame: &mut *(raw.arg as *mut _),
            },
            XCB_XIM_SET_IC_VALUES => SetIcValues {
                client: raw.client.unwrap(),
                ic: raw.ic.unwrap(),
                frame: &*(raw.frame as *const _),
            },
            XCB_XIM_GET_IC_VALUES => GetIcValues {
                client: raw.client.unwrap(),
                ic: raw.ic.unwrap(),
                frame: &*(raw.frame as *const _),
            },
            XCB_XIM_SET_IC_FOCUS => SetIcFocus {
                client: raw.client.unwrap(),
                ic: raw.ic.unwrap(),
                frame: &*(raw.frame as *const _),
            },
            XCB_XIM_UNSET_IC_FOCUS => UnsetIcFocus {
                client: raw.client.unwrap(),
                ic: raw.ic.unwrap(),
                frame: &*(raw.frame as *const _),
            },
            XCB_XIM_DESTROY_IC => DestroyIc {
                client: raw.client.unwrap(),
                ic: raw.ic.unwrap(),
            },
            XCB_XIM_RESET_IC => ResetIc {
                client: raw.client.unwrap(),
                ic: raw.ic.unwrap(),
                frame: &*(raw.frame as *const _),
                reply_frame: &mut *(raw.frame as *mut _),
            },
            XCB_XIM_FORWARD_EVENT => ForwardEvent {
                client: raw.client.unwrap(),
                ic: raw.ic.unwrap(),
                frame: &*(raw.frame as *const _),
                key_event: &*(raw.arg as *const _),
            },
            XCB_XIM_EXTENSION => match minor_opcode as u32 {
                XCB_XIM_EXT_FORWARD_KEYEVENT => ExtForwardKeyevent {
                    client: raw.client.unwrap(),
                    ic: raw.ic.unwrap(),
                    frame: &*(raw.frame as *const _),
                    key_event: &*(raw.arg as *const _),
                },
                _ => __Unsupported,
            },
            XCB_XIM_SYNC_REPLY => SyncReply {
                client: raw.client.unwrap(),
                ic: raw.ic.unwrap(),
                frame: &*(raw.frame as *const _),
            },
            XCB_XIM_TRIGGER_NOTIFY => TriggerNotify {
                client: raw.client.unwrap(),
                ic: raw.ic.unwrap(),
                frame: &*(raw.frame as *const _),
            },
            XCB_XIM_PREEDIT_START_REPLY => PreeditStartReply {
                client: raw.client.unwrap(),
                ic: raw.ic.unwrap(),
                frame: &*(raw.frame as *const _),
            },
            XCB_XIM_PREEDIT_CARET_REPLY => PreeditCaretReply {
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

#[derive(Debug, Clone)]
pub struct ConnectMessage<'a> {
    pub byte_order: u8,
    pub client_major_protocol_version: u16,
    pub client_minor_protocol_version: u16,
    pub client_auth_protocol_names: Vec<&'a [u8]>,
}

impl<'a> From<&'a xcb_im_connect_fr_t> for ConnectMessage<'a> {
    fn from(fr: &'a xcb_im_connect_fr_t) -> Self {
        let client_auth_protocol_names = unsafe {
            slice_from_raw(
                fr.client_auth_protocol_names.items,
                fr.client_auth_protocol_names.size as usize,
            )
            .iter()
            .map(|xpcs| slice_from_raw(xpcs.string, xpcs.length_of_string_in_bytes))
            .collect()
        };

        ConnectMessage {
            byte_order: fr.byte_order,
            client_major_protocol_version: fr.client_major_protocol_version,
            client_minor_protocol_version: fr.client_minor_protocol_version,
            client_auth_protocol_names,
        }
    }
}

#[derive(Debug, Clone)]
pub struct OpenMessage<'a> {
    pub locale_name: &'a [u8],
}

impl<'a> From<&'a xcb_im_open_fr_t> for OpenMessage<'a> {
    fn from(fr: &'a xcb_im_open_fr_t) -> Self {
        OpenMessage {
            locale_name: unsafe { slice_from_raw(fr.field0.string, fr.field0.length_of_string) },
        }
    }
}

pub type CloseMessage = xcb_im_close_fr_t;

#[derive(Debug, Clone)]
pub struct CreateIcMessage<'a> {
    pub input_method_id: u16,
    pub ic_attributes: Vec<XicAttribute<'a>>,
}

impl<'a> From<&'a xcb_im_create_ic_fr_t> for CreateIcMessage<'a> {
    fn from(fr: &'a xcb_im_create_ic_fr_t) -> Self {
        CreateIcMessage {
            input_method_id: fr.input_method_ID,
            ic_attributes: unsafe {
                slice_from_raw(fr.ic_attributes.items, fr.ic_attributes.size as usize)
                    .iter()
                    .map(|x| (&*x).into())
                    .collect()
            },
        }
    }
}

pub type CreateIcReplyMessage = xcb_im_create_ic_reply_fr_t;

#[derive(Debug, Clone)]
pub struct SetIcValuesMessage<'a> {
    pub input_method_id: u16,
    pub input_context_id: u16,
    pub ic_attributes: Vec<XicAttribute<'a>>,
}

impl<'a> From<&'a xcb_im_set_ic_values_fr_t> for SetIcValuesMessage<'a> {
    fn from(fr: &'a xcb_im_set_ic_values_fr_t) -> Self {
        SetIcValuesMessage {
            input_method_id: fr.input_method_ID,
            input_context_id: fr.input_context_ID,
            ic_attributes: unsafe {
                slice_from_raw(fr.ic_attribute.items, fr.ic_attribute.size as usize)
                    .iter()
                    .map(|x| (&*x).into())
                    .collect()
            },
        }
    }
}

#[derive(Debug, Clone)]
pub struct GetIcValuesMessage<'a> {
    pub input_method_id: u16,
    pub input_context_id: u16,
    pub ic_attribute_id: &'a [u16],
}

impl<'a> From<&'a xcb_im_get_ic_values_fr_t> for GetIcValuesMessage<'a> {
    fn from(fr: &'a xcb_im_get_ic_values_fr_t) -> Self {
        GetIcValuesMessage {
            input_method_id: fr.input_method_ID,
            input_context_id: fr.input_context_ID,
            ic_attribute_id: unsafe {
                slice_from_raw(fr.ic_attribute.items, fr.ic_attribute.size as usize)
            },
        }
    }
}

pub type SetIcFocusMessage = xcb_im_set_ic_focus_fr_t;

pub type UnsetIcFocusMessage = xcb_im_unset_ic_focus_fr_t;

pub type ResetIcMessage = xcb_im_reset_ic_fr_t;

#[derive(Debug, Clone)]
pub struct ResetIcReplyMessage {
    pub preedit_string: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct ForwardEventMessage {
    pub input_method_id: u16,
    pub input_context_id: u16,
    pub flag: ForwardEventFlag,
    pub sequence_number: u16,
}

impl<'a> From<&'a xcb_im_forward_event_fr_t> for ForwardEventMessage {
    fn from(fr: &'a xcb_im_forward_event_fr_t) -> Self {
        debug_assert!(ForwardEventFlag::from_bits(fr.flag).is_some());
        ForwardEventMessage {
            input_method_id: fr.input_method_ID,
            input_context_id: fr.input_context_ID,
            flag: ForwardEventFlag::from_bits_truncate(fr.flag),
            sequence_number: fr.sequence_number,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ExtForwardKeyeventMessage {
    pub input_method_id: u16,
    pub input_context_id: u16,
    pub flag: ExtForwardKeyeventFlag,
    pub sequence_number: u16,
    pub event_type: u8,
    pub keycode: xcb::Keycode,
    pub state: u16,
    pub time: xcb::Timestamp,
    pub window: xcb::Window,
}

impl<'a> From<&'a xcb_im_ext_forward_keyevent_fr_t> for ExtForwardKeyeventMessage {
    fn from(fr: &'a xcb_im_ext_forward_keyevent_fr_t) -> Self {
        debug_assert!(ExtForwardKeyeventFlag::from_bits(fr.flag).is_some());
        ExtForwardKeyeventMessage {
            input_method_id: fr.input_method_ID,
            input_context_id: fr.input_context_ID,
            flag: ExtForwardKeyeventFlag::from_bits_truncate(fr.flag),
            sequence_number: fr.sequence_number,
            event_type: fr.xEvent_u_u_type,
            keycode: fr.keycode,
            state: fr.state,
            time: fr.time,
            window: fr.window,
        }
    }
}

pub type SyncReplyMessage = xcb_im_sync_reply_fr_t;

#[derive(Debug, Clone)]
pub struct TriggerNotifyMessage {
    pub input_method_id: u16,
    pub input_context_id: u16,
    pub flag: TriggerNotifyFlag,
    pub index_of_keys_list: u32,
    pub client_select_event_mask: u32,
}

impl<'a> From<&'a xcb_im_trigger_notify_fr_t> for TriggerNotifyMessage {
    fn from(fr: &'a xcb_im_trigger_notify_fr_t) -> Self {
        let flag = fr.flag.into();
        match (cfg!(debug_assertions), flag) {
            (true, TriggerNotifyFlag::Other(x)) => panic!("unexpected flag {:?}", x),
            _ => (),
        }

        TriggerNotifyMessage {
            input_method_id: fr.input_method_ID,
            input_context_id: fr.input_context_ID,
            flag,
            index_of_keys_list: fr.index_of_keys_list,
            client_select_event_mask: fr.client_select_event_mask,
        }
    }
}

#[derive(Debug, Clone)]
pub struct PreeditStartReplyMessage {
    pub input_method_id: u16,
    pub input_context_id: u16,
    /// The maximum number of bytes of preedit string. If -1, it is unlimited.
    pub return_value: i32,
}

impl<'a> From<&'a xcb_im_preedit_start_reply_fr_t> for PreeditStartReplyMessage {
    fn from(fr: &'a xcb_im_preedit_start_reply_fr_t) -> Self {
        PreeditStartReplyMessage {
            input_method_id: fr.input_method_ID,
            input_context_id: fr.input_context_ID,
            return_value: fr.return_value as i32,
        }
    }
}

pub type PreeditCaretReplyMessage = xcb_im_preedit_caret_reply_fr_t;
