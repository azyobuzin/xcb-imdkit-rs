use super::data_types::*;
use super::{slice_from_raw, ImClient, ImServerRef, InputContext};
use crate::ffi::*;
use std::mem;
use std::os::raw::c_void;
use std::slice;

#[derive(Debug, Clone)]
pub(crate) struct RawCallbackArgs<'a> {
    pub client: Option<&'a ImClient>,
    pub ic: Option<&'a InputContext>,
    pub major_opcode: u8,
    pub minor_opcode: u8,
    pub frame: *mut c_void,
    pub arg: *mut c_void,
}

pub trait ImMessageHandler {
    fn handle_connect(&mut self, _im: &ImServerRef, _client: &ImClient, _frame: &ConnectMessage) {}

    fn handle_disconnect(&mut self, _im: &ImServerRef, _client: &ImClient) {}

    fn handle_open(&mut self, _im: &ImServerRef, _client: &ImClient, _frame: &OpenMessage) {}

    fn handle_close(&mut self, _im: &ImServerRef, _client: &ImClient, _frame: &CloseMessage) {}

    fn handle_create_ic(
        &mut self,
        _im: &ImServerRef,
        _client: &ImClient,
        _ic: &InputContext,
        _frame: &CreateIcMessage,
        _reply_frame: &CreateIcReplyMessage,
    ) {
    }

    fn handle_set_ic_values(
        &mut self,
        _im: &ImServerRef,
        _client: &ImClient,
        _ic: &InputContext,
        _frame: &SetIcValuesMessage,
    ) {
    }

    fn handle_get_ic_values(
        &mut self,
        _im: &ImServerRef,
        _client: &ImClient,
        _ic: &InputContext,
        _frame: &GetIcValuesMessage,
    ) {
    }

    fn handle_set_ic_focus(
        &mut self,
        _im: &ImServerRef,
        _client: &ImClient,
        _ic: &InputContext,
        _frame: &SetIcFocusMessage,
    ) {
    }

    fn handle_unset_ic_focus(
        &mut self,
        _im: &ImServerRef,
        _client: &ImClient,
        _ic: &InputContext,
        _frame: &UnsetIcFocusMessage,
    ) {
    }

    fn handle_destoy_ic(&mut self, _im: &ImServerRef, _client: &ImClient, _ic: &InputContext) {}

    fn handle_reset_ic(
        &mut self,
        _im: &ImServerRef,
        _client: &ImClient,
        _ic: &InputContext,
        _frame: &ResetIcMessage,
    ) -> ResetIcReplyMessage {
        Default::default()
    }

    fn handle_forward_event(
        &mut self,
        _im: &ImServerRef,
        _client: &ImClient,
        _ic: &InputContext,
        _frame: &ForwardEventMessage,
        _key_event: &xcb::KeyPressEvent,
    ) {
    }

    fn handle_ext_forward_keyevent(
        &mut self,
        _im: &ImServerRef,
        _client: &ImClient,
        _ic: &InputContext,
        _frame: &ExtForwardKeyeventMessage,
        _key_event: &xcb::KeyPressEvent,
    ) {
    }

    fn handle_sync_reply(
        &mut self,
        _im: &ImServerRef,
        _client: &ImClient,
        _ic: &InputContext,
        _frame: &SyncReplyMessage,
    ) {
    }

    fn handle_trigger_notify(
        &mut self,
        _im: &ImServerRef,
        _client: &ImClient,
        _ic: &InputContext,
        _frame: &TriggerNotifyMessage,
    ) {
    }

    fn handle_preedit_start_reply(
        &mut self,
        _im: &ImServerRef,
        _client: &ImClient,
        _ic: &InputContext,
        _frame: &PreeditStartReplyMessage,
    ) {
    }

    fn handle_preedit_caret_reply(
        &mut self,
        _im: &ImServerRef,
        _client: &ImClient,
        _ic: &InputContext,
        _frame: &PreeditCaretReplyMessage,
    ) {
    }
}

pub(crate) fn handle_callback(
    im: &ImServerRef,
    args: &RawCallbackArgs,
    handler: &mut dyn ImMessageHandler,
) {
    unsafe {
        match (args.major_opcode as u32, args.minor_opcode as u32) {
            (XCB_XIM_CONNECT, _) => handler.handle_connect(
                im,
                args.client.unwrap(),
                &(&*(args.frame as *const xcb_im_connect_fr_t)).into(),
            ),
            (XCB_XIM_DISCONNECT, _) => handler.handle_disconnect(im, args.client.unwrap()),
            (XCB_XIM_OPEN, _) => handler.handle_open(
                im,
                args.client.unwrap(),
                &(&*(args.frame as *const xcb_im_open_fr_t)).into(),
            ),
            (XCB_XIM_CLOSE, _) => handler.handle_close(
                im,
                args.client.unwrap(),
                &*(args.frame as *const xcb_im_close_fr_t),
            ),
            (XCB_XIM_CREATE_IC, _) => handler.handle_create_ic(
                im,
                args.client.unwrap(),
                args.ic.unwrap(),
                &(&*(args.frame as *const xcb_im_create_ic_fr_t)).into(),
                &*(args.arg as *mut xcb_im_create_ic_reply_fr_t),
            ),
            (XCB_XIM_SET_IC_VALUES, _) => handler.handle_set_ic_values(
                im,
                args.client.unwrap(),
                args.ic.unwrap(),
                &(&*(args.frame as *const xcb_im_set_ic_values_fr_t)).into(),
            ),
            (XCB_XIM_GET_IC_VALUES, _) => handler.handle_get_ic_values(
                im,
                args.client.unwrap(),
                args.ic.unwrap(),
                &(&*(args.frame as *const xcb_im_get_ic_values_fr_t)).into(),
            ),
            (XCB_XIM_SET_IC_FOCUS, _) => handler.handle_set_ic_focus(
                im,
                args.client.unwrap(),
                args.ic.unwrap(),
                &*(args.frame as *const xcb_im_set_ic_focus_fr_t),
            ),
            (XCB_XIM_UNSET_IC_FOCUS, _) => handler.handle_unset_ic_focus(
                im,
                args.client.unwrap(),
                args.ic.unwrap(),
                &*(args.frame as *const xcb_im_unset_ic_focus_fr_t),
            ),
            (XCB_XIM_DESTROY_IC, _) => {
                handler.handle_destoy_ic(im, args.client.unwrap(), args.ic.unwrap())
            }
            (XCB_XIM_RESET_IC, _) => {
                let reply = handler.handle_reset_ic(
                    im,
                    args.client.unwrap(),
                    args.ic.unwrap(),
                    &*(args.frame as *const xcb_im_reset_ic_fr_t),
                );
                if reply.preedit_string.len() > 0 {
                    let allocated =
                        libc::calloc(reply.preedit_string.len() + 1, mem::size_of::<u8>())
                            as *mut u8;
                    slice::from_raw_parts_mut(allocated, reply.preedit_string.len())
                        .copy_from_slice(&reply.preedit_string);

                    let mut reply_frame = &mut *(args.arg as *mut xcb_im_reset_ic_reply_fr_t);
                    reply_frame.byte_length_of_committed_string = reply.preedit_string.len() as u16;
                    reply_frame.committed_string = allocated; // freed by xcb-imdkit
                }
            }
            (XCB_XIM_FORWARD_EVENT, _) => {
                let key_event = xcb::Event {
                    ptr: args.arg as *mut xcb::ffi::xcb_key_press_event_t,
                };
                handler.handle_forward_event(
                    im,
                    args.client.unwrap(),
                    args.ic.unwrap(),
                    &(&*(args.frame as *const xcb_im_forward_event_fr_t)).into(),
                    &key_event,
                );
                mem::forget(key_event); // do not free
            }
            (XCB_XIM_EXTENSION, XCB_XIM_EXT_FORWARD_KEYEVENT) => {
                let key_event = xcb::Event {
                    ptr: args.arg as *mut xcb::ffi::xcb_key_press_event_t,
                };
                handler.handle_ext_forward_keyevent(
                    im,
                    args.client.unwrap(),
                    args.ic.unwrap(),
                    &(&*(args.frame as *const xcb_im_ext_forward_keyevent_fr_t)).into(),
                    &key_event,
                );
                mem::forget(key_event); // do not free
            }
            (XCB_XIM_SYNC_REPLY, _) => handler.handle_sync_reply(
                im,
                args.client.unwrap(),
                args.ic.unwrap(),
                &*(args.frame as *const xcb_im_sync_reply_fr_t),
            ),
            (XCB_XIM_TRIGGER_NOTIFY, _) => handler.handle_trigger_notify(
                im,
                args.client.unwrap(),
                args.ic.unwrap(),
                &(&*(args.frame as *const xcb_im_trigger_notify_fr_t)).into(),
            ),
            (XCB_XIM_PREEDIT_START_REPLY, _) => handler.handle_preedit_start_reply(
                im,
                args.client.unwrap(),
                args.ic.unwrap(),
                &(&*(args.frame as *const xcb_im_preedit_start_reply_fr_t)).into(),
            ),
            (XCB_XIM_PREEDIT_CARET_REPLY, _) => handler.handle_preedit_caret_reply(
                im,
                args.client.unwrap(),
                args.ic.unwrap(),
                &*(args.frame as *const xcb_im_preedit_caret_reply_fr_t),
            ),
            x => {
                if cfg!(debug_assertions) {
                    panic!("unknown opcode {:?}", x)
                }
            }
        }
    };
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

#[derive(Debug, Clone, Default)]
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
