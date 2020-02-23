use super::ffi;
use std::collections::HashSet;
use std::ffi::CStr;
use std::fmt;
use std::marker::PhantomData;
use std::mem;
use std::os::raw::{c_char, c_void};
use xcb;

pub struct ImServer<'a> {
    conn: PhantomData<&'a xcb::Connection>,
    im: *mut ffi::xcb_im_t,
    data_ptr: *mut ImServerData,
}

#[derive(Default)]
struct ImServerData {
    im: Option<*mut ffi::xcb_im_t>,
    callback: Option<Box<InternalImCallback>>,
    valid_ics: HashSet<InputContext>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ImClient(usize);

impl ImClient {
    pub fn as_ptr(&self) -> *mut ffi::xcb_im_client_t {
        self.0 as *mut ffi::xcb_im_client_t
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct InputContext(usize);

impl InputContext {
    pub fn as_ptr(&self) -> *mut ffi::xcb_im_input_context_t {
        self.0 as *mut ffi::xcb_im_input_context_t
    }
}

type InternalImCallback = dyn FnMut(&ImServer, ImClient, InputContext, ImMessage);

impl<'a> ImServer<'a> {
    pub fn create<E>(
        conn: &'a xcb::Connection,
        screen: i32,
        server_window: xcb::Window,
        server_name: &CStr,
        locale: &CStr,
        input_styles: &[ffi::xcb_im_style_t],
        on_keys_list: &[ffi::xcb_im_ximtriggerkey_fr_t],
        off_keys_list: &[ffi::xcb_im_ximtriggerkey_fr_t],
        encoding_list: impl IntoIterator<Item = E>,
        event_mask: u32,
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

        let data_ptr = Box::into_raw(Box::<ImServerData>::default());

        let im = unsafe {
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
        };

        unsafe {
            Box::leak(Box::from_raw(data_ptr)).im = Some(im);
        }

        ImServer {
            conn: Default::default(),
            im,
            data_ptr,
        }
    }

    pub fn set_callback<'b, F>(&'b mut self, mut callback: F)
    where
        F: FnMut(&'b ImServer<'a>, ImClient, InputContext, ImMessage) + 'static,
    {
        let internal_callback = move |im: &ImServer, client, ic, msg: ImMessage| {
            // Cast away lifetimes
            unsafe { callback(mem::transmute(im), client, ic, msg) }
        };

        unsafe {
            Box::leak(Box::from_raw(self.data_ptr)).callback = Some(Box::new(internal_callback));
        }
    }

    pub fn open(&mut self) -> Result<(), ()> {
        match unsafe { ffi::xcb_im_open_im(self.im) } {
            true => Ok(()),
            false => Err(()),
        }
    }

    pub fn close(&mut self) {
        unsafe { ffi::xcb_im_close_im(self.im) }
    }

    pub fn filter_event(&self, event: &xcb::GenericEvent) -> bool {
        unsafe { ffi::xcb_im_filter_event(self.im, event.ptr) }
    }

    pub fn forward_event(&self, ic: &InputContext, event: &xcb::KeyPressEvent) {
        unsafe { ffi::xcb_im_forward_event(self.im, ic.as_ptr(), event.ptr) }
    }

    pub fn commit_string(&self, ic: &InputContext, committed_str: &CommittedString) {
        use CommittedString::*;
        let flag = match committed_str {
            Chars(_) => ffi::xcb_xim_lookup_flags_t_XCB_XIM_LOOKUP_CHARS,
            KeySym(_) => ffi::xcb_xim_lookup_flags_t_XCB_XIM_LOOKUP_KEYSYM,
            Both(_, _) => ffi::xcb_xim_lookup_flags_t_XCB_XIM_LOOKUP_BOTH,
        };
        let (s, len) = {
            let opt = match committed_str {
                Chars(x) | Both(_, x) => Some(x),
                _ => None,
            };
            match opt {
                Some(x) => (x.as_ptr(), x.to_bytes().len()),
                None => (std::ptr::null(), 0),
            }
        };
        let keysym = match committed_str {
            KeySym(x) | Both(x, _) => *x,
            Chars(_) => 0,
        };
        unsafe {
            ffi::xcb_im_commit_string(
                self.im,
                ic.as_ptr(),
                flag,
                s as *mut c_char,
                len as u32,
                keysym,
            )
        }
    }
}

impl<'a> fmt::Debug for ImServer<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("ImServer").field("im", &self.im).finish()
    }
}

impl<'a> Drop for ImServer<'a> {
    fn drop(&mut self) {
        unsafe {
            ffi::xcb_im_destroy(self.im);
            Box::from_raw(self.data_ptr);
        }
    }
}

unsafe extern "C" fn im_callback(
    im: *mut ffi::xcb_im_t,
    client: *mut ffi::xcb_im_client_t,
    ic: *mut ffi::xcb_im_input_context_t,
    hdr: *const ffi::xcb_im_packet_header_fr_t,
    frame: *mut c_void,
    arg: *mut c_void,
    user_data: *mut c_void,
) {
    let data_ptr = user_data as *mut ImServerData;
    let data_cell = Box::leak(Box::from_raw(data_ptr));

    match data_cell.im {
        Some(p) if p == im => (),
        _ => panic!("Unknown im"),
    }

    if let Some(ref mut callback) = data_cell.callback {
        // Maintain ICs set
        let ic = InputContext(ic as usize);
        let msg = parse_message(hdr, frame, arg);
        match msg {
            ImMessage::CreateIc(_, _) => {
                data_cell.valid_ics.insert(ic);
            }
            ImMessage::DestroyIc => {
                data_cell.valid_ics.remove(&ic);
            }
            _ => (),
        }

        let im_server = ImServer {
            conn: Default::default(),
            im,
            data_ptr,
        };

        callback(&im_server, ImClient(client as usize), ic, msg);
        mem::forget(im_server);
    }
}

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

fn parse_message<'a>(
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

#[derive(Debug, Clone, Copy)]
pub enum CommittedString<'a> {
    Chars(&'a CStr),
    KeySym(u32),
    Both(u32, &'a CStr),
}
