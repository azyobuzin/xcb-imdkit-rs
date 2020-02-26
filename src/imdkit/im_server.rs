use super::*;
use crate::ffi;
use std::borrow::Borrow;
use std::cell::RefCell;
use std::collections::HashMap;
use std::ffi::CStr;
use std::fmt;
use std::marker::PhantomData;
use std::mem;
use std::os::raw::{c_char, c_void};
use std::ptr::NonNull;
use xcb;

pub struct ImServer<'a> {
    conn: PhantomData<&'a xcb::Connection>,
    data_ptr: NonNull<ImServerData<'a>>,
    close_on_drop: bool,
}

#[derive(PartialEq, Eq, Hash)]
pub struct ImServerRef(NonNull<ImServerData<'static>>);

#[derive(Debug, Clone, Copy)]
pub enum CommittedString<'a> {
    KeySym(u32),
    Chars(&'a [u8]),
    Both(u32, &'a [u8]),
}

struct ImServerData<'a> {
    im: Option<NonNull<ffi::xcb_im_t>>,
    handler: RefCell<Box<dyn ImMessageHandler + 'a>>,
    input_contexts: HashMap<NonNull<ffi::xcb_im_input_context_t>, InputContext>,
}

impl<'a> ImServer<'a> {
    pub fn create<E>(
        conn: &'a xcb::Connection,
        screen: i32,
        server_window: xcb::Window,
        server_name: &CStr,
        locale: &CStr,
        input_styles: &[InputStyle],
        on_keys_list: &[XimTriggerKey],
        off_keys_list: &[XimTriggerKey],
        encoding_list: impl IntoIterator<Item = E>,
        event_mask: u32,
        handler: impl ImMessageHandler + 'a,
    ) -> Self
    where
        E: AsRef<CStr>,
    {
        let input_styles = ffi::xcb_im_styles_t {
            nStyles: input_styles.len() as u32,
            styles: input_styles.as_ptr() as *mut u32,
        };
        let on_keys_list = ffi::xcb_im_trigger_keys_t {
            nKeys: on_keys_list.len() as u16,
            keys: on_keys_list.as_ptr() as *mut ffi::xcb_im_ximtriggerkey_fr_t,
        };
        let off_keys_list = ffi::xcb_im_trigger_keys_t {
            nKeys: off_keys_list.len() as u16,
            keys: off_keys_list.as_ptr() as *mut ffi::xcb_im_ximtriggerkey_fr_t,
        };
        let encoding_list_ptrs = encoding_list
            .into_iter()
            .map(|s| s.as_ref().as_ptr() as *mut c_char)
            .collect::<Vec<_>>();
        let encoding_list = ffi::xcb_im_encodings_t {
            nEncodings: encoding_list_ptrs.len() as u16,
            encodings: encoding_list_ptrs.as_ptr() as *mut ffi::xcb_im_encoding_t,
        };

        let data_ptr = Box::into_raw(Box::new(ImServerData {
            im: None,
            handler: RefCell::new(Box::new(handler)),
            input_contexts: Default::default(),
        }));

        let im = NonNull::new(unsafe {
            ffi::xcb_im_create(
                conn.get_raw_conn(),
                screen,
                server_window,
                server_name.as_ptr(),
                locale.as_ptr(),
                &input_styles,
                &on_keys_list,
                &off_keys_list,
                &encoding_list,
                event_mask,
                Some(im_callback),
                data_ptr as *mut c_void,
            )
        })
        .expect("im is null");

        unsafe {
            Box::leak(Box::from_raw(data_ptr)).im = Some(im);
        }

        ImServer {
            conn: Default::default(),
            data_ptr: NonNull::new(data_ptr).unwrap(),
            close_on_drop: false,
        }
    }

    pub fn open(&mut self) -> Result<(), ()> {
        match unsafe { ffi::xcb_im_open_im(self.as_ref().get_im_ptr()) } {
            true => Ok(()),
            false => Err(()),
        }
    }

    pub fn close(&mut self) {
        unsafe { ffi::xcb_im_close_im(self.as_ref().get_im_ptr()) }
    }

    pub fn filter_event(&mut self, event: &xcb::GenericEvent) -> bool {
        unsafe { ffi::xcb_im_filter_event(self.as_ref().get_im_ptr(), event.ptr) }
    }

    pub fn close_on_drop(&mut self, enabled: bool) {
        self.close_on_drop = enabled;
    }
}

extern "C" fn im_callback(
    im: *mut ffi::xcb_im_t,
    client: *mut ffi::xcb_im_client_t,
    ic: *mut ffi::xcb_im_input_context_t,
    hdr: *const ffi::xcb_im_packet_header_fr_t,
    frame: *mut c_void,
    arg: *mut c_void,
    user_data: *mut c_void,
) {
    let data_ptr = NonNull::new(user_data as *mut ImServerData).expect("user_data is null");
    let data_cell = unsafe { Box::leak(Box::from_raw(data_ptr.as_ptr())) };

    match data_cell.im {
        Some(p) if p.as_ptr() == im => (),
        expected => {
            if cfg!(debug_assertions) {
                panic!("unexpected im (actual: {:?}, expected: {:?}", im, expected)
            }
            return;
        }
    }

    let client_opt = NonNull::new(client).map(ImClient);
    let ic_ptr_opt = NonNull::new(ic);
    let ic_opt = ic_ptr_opt.as_ref().map(|x| InputContext(*x));
    let hdr = unsafe { &*hdr };
    let raw_args = super::RawCallbackArgs {
        client: client_opt.as_ref(),
        ic: ic_opt.as_ref(),
        major_opcode: hdr.major_opcode,
        minor_opcode: hdr.minor_opcode,
        frame,
        arg,
    };

    // Maintain alive ICs
    let destroyed_ic = match (raw_args.major_opcode as u32, ic_ptr_opt) {
        (ffi::XCB_XIM_CREATE_IC, Some(ic)) => {
            data_cell.input_contexts.insert(ic, InputContext(ic));
            None
        }
        (ffi::XCB_XIM_DESTROY_IC, ic) => ic,
        _ => None,
    };

    // Call handler
    let im_ref = ImServerRef(data_ptr);
    handle_callback(&im_ref, &raw_args, &mut **data_cell.handler.borrow_mut());

    if let Some(ic) = destroyed_ic {
        data_cell.input_contexts.remove(&ic);
    }
}

impl<'a> Drop for ImServer<'a> {
    fn drop(&mut self) {
        unsafe {
            let data = Box::from_raw(self.data_ptr.as_ptr()); // will be dropped

            if let Some(im) = data.im {
                if self.close_on_drop {
                    ffi::xcb_im_close_im(im.as_ptr())
                }

                ffi::xcb_im_destroy(im.as_ptr())
            }
        }
    }
}

impl<'a> Borrow<ImServerRef> for ImServer<'a> {
    #[inline]
    fn borrow(&self) -> &ImServerRef {
        unsafe { mem::transmute(&self.data_ptr) }
    }
}

impl<'a> AsRef<ImServerRef> for ImServer<'a> {
    #[inline]
    fn as_ref(&self) -> &ImServerRef {
        self.borrow()
    }
}

impl<'a> fmt::Debug for ImServer<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("ImServer")
            .field("data", self.as_ref().get_data())
            .finish()
    }
}

impl ImServerRef {
    pub fn forward_event(&self, ic: &InputContext, event: &xcb::KeyPressEvent) {
        unsafe { ffi::xcb_im_forward_event(self.get_im_ptr(), ic.as_ptr(), event.ptr) }
    }

    pub fn commit_string(&self, ic: &InputContext, committed_str: &CommittedString) {
        use CommittedString::*;
        let flag = match committed_str {
            KeySym(_) => ffi::xcb_xim_lookup_flags_t_XCB_XIM_LOOKUP_KEYSYM,
            Chars(_) => ffi::xcb_xim_lookup_flags_t_XCB_XIM_LOOKUP_CHARS,
            Both(_, _) => ffi::xcb_xim_lookup_flags_t_XCB_XIM_LOOKUP_BOTH,
        };
        let (s, len) = {
            let opt = match committed_str {
                Chars(x) | Both(_, x) => Some(x),
                _ => None,
            };
            match opt {
                Some(x) => (x.as_ptr(), x.len()),
                None => (std::ptr::null(), 0),
            }
        };
        let keysym = match committed_str {
            KeySym(x) | Both(x, _) => *x,
            Chars(_) => 0,
        };

        unsafe {
            ffi::xcb_im_commit_string(
                self.get_im_ptr(),
                ic.as_ptr(),
                flag,
                s as *mut c_char,
                len as u32,
                keysym,
            )
        }
    }

    pub fn geometry_callback(&self, ic: &InputContext) {
        unsafe { ffi::xcb_im_geometry_callback(self.get_im_ptr(), ic.as_ptr()) }
    }

    pub fn preedit_start_callback(&self, ic: &InputContext) {
        unsafe { ffi::xcb_im_preedit_start_callback(self.get_im_ptr(), ic.as_ptr()) }
    }

    pub fn preedit_draw_callback(&self, ic: &InputContext, frame: &PreeditDrawMessage) {
        let mut frame = ffi::xcb_im_preedit_draw_fr_t {
            input_method_ID: 0,  // set by xcb-imdkit
            input_context_ID: 0, // set by xcb-imdkit
            caret: frame.caret as u32,
            chg_first: frame.chg_first as u32,
            chg_length: frame.chg_length as u32,
            status: frame.status.bits(),
            length_of_preedit_string: frame.preedit_string.len() as u16,
            preedit_string: frame.preedit_string.as_ptr() as *mut u8,
            feedback_array: ffi::_xcb_im_preedit_draw_fr_t__bindgen_ty_1 {
                size: (frame.feedback_array.len() * mem::size_of::<u32>()) as u32,
                items: frame.feedback_array.as_ptr() as *mut u32,
            },
        };

        unsafe { ffi::xcb_im_preedit_draw_callback(self.get_im_ptr(), ic.as_ptr(), &mut frame) }
    }

    pub fn preedit_caret_callback(&self, ic: &InputContext, frame: &PreeditCaretMessage) {
        let mut frame = ffi::xcb_im_preedit_caret_fr_t {
            input_method_ID: 0,  // set by xcb-imdkit
            input_context_ID: 0, // set by xcb-imdkit
            position: frame.position as u32,
            direction: frame.direction as u32,
            style: frame.style as u32,
        };

        unsafe { ffi::xcb_im_preedit_caret_callback(self.get_im_ptr(), ic.as_ptr(), &mut frame) }
    }

    pub fn preedit_done_callback(&self, ic: &InputContext) {
        unsafe { ffi::xcb_im_preedit_done_callback(self.get_im_ptr(), ic.as_ptr()) }
    }

    pub fn status_start_callback(&self, ic: &InputContext) {
        unsafe { ffi::xcb_im_status_start_callback(self.get_im_ptr(), ic.as_ptr()) }
    }

    pub fn status_draw_text_callback(&self, ic: &InputContext, frame: &StatusDrawTextMessage) {
        let mut frame = ffi::xcb_im_status_draw_text_fr_t {
            input_method_ID: 0,          // set by xcb-imdkit
            input_context_ID: 0,         // set by xcb-imdkit
            type_: ffi::XCB_IM_TextType, // set by xcb-imdkit
            status: frame.status.bits(),
            length_of_status_string: frame.status_string.len() as u16,
            status_string: frame.status_string.as_ptr() as *mut u8,
            feedback_array: ffi::_xcb_im_status_draw_text_fr_t__bindgen_ty_1 {
                size: (frame.feedback_array.len() * mem::size_of::<u32>()) as u32,
                items: frame.feedback_array.as_ptr() as *mut u32,
            },
        };

        unsafe { ffi::xcb_im_status_draw_text_callback(self.get_im_ptr(), ic.as_ptr(), &mut frame) }
    }

    pub fn status_draw_bitmap_callback(&self, ic: &InputContext, frame: &StatusDrawBitmapMessage) {
        let mut frame = ffi::xcb_im_status_draw_bitmap_fr_t {
            input_method_ID: 0,            // set by xcb-imdkit
            input_context_ID: 0,           // set by xcb-imdkit
            type_: ffi::XCB_IM_BitmapType, // set by xcb-imdkit
            pixmap_data: frame.pixmap_data,
        };

        unsafe {
            ffi::xcb_im_status_draw_bitmap_callback(self.get_im_ptr(), ic.as_ptr(), &mut frame)
        }
    }

    pub fn status_done_callback(&self, ic: &InputContext) {
        unsafe { ffi::xcb_im_status_done_callback(self.get_im_ptr(), ic.as_ptr()) }
    }

    pub fn preedit_start(&self, ic: &InputContext) {
        unsafe { ffi::xcb_im_preedit_start(self.get_im_ptr(), ic.as_ptr()) }
    }

    pub fn preedit_end(&self, ic: &InputContext) {
        unsafe { ffi::xcb_im_preedit_end(self.get_im_ptr(), ic.as_ptr()) }
    }

    pub fn sync_xlib(&self, ic: &InputContext) {
        unsafe { ffi::xcb_im_sync_xlib(self.get_im_ptr(), ic.as_ptr()) }
    }

    pub fn support_extension(&self, major_code: u16, minor_code: u16) -> bool {
        unsafe { ffi::xcb_im_support_extension(self.get_im_ptr(), major_code, minor_code) }
    }

    pub fn get_ic(&self, ic_ptr: *mut ffi::xcb_im_input_context_t) -> Option<&InputContext> {
        NonNull::new(ic_ptr).and_then(|p| self.get_data().input_contexts.get(&p))
    }

    #[inline]
    pub fn get_im_ptr(&self) -> *mut ffi::xcb_im_t {
        self.get_im_ptr_non_null().as_ptr()
    }

    #[inline]
    pub fn get_im_ptr_non_null(&self) -> NonNull<ffi::xcb_im_t> {
        self.get_data().im.unwrap()
    }

    fn get_data(&self) -> &mut ImServerData<'static> {
        // HACK: ignore lifetime parameter because ImServerRef lives shorter than ImServer
        unsafe { Box::leak(Box::from_raw(self.0.as_ptr())) }
    }
}

impl fmt::Debug for ImServerRef {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("ImServerRef")
            .field("data", self.get_data())
            .finish()
    }
}

impl<'a> fmt::Debug for ImServerData<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("ImServerData")
            .field("im", &self.im)
            .field("input_contexts", &self.input_contexts)
            .finish()
    }
}
