use super::{ImClient, ImMessage, InputContext};
use crate::ffi;
use std::collections::HashSet;
use std::ffi::CStr;
use std::fmt;
use std::marker::PhantomData;
use std::mem;
use std::os::raw::{c_char, c_void};
use std::ptr::NonNull;
use xcb;

pub struct ImServer<'a> {
    conn: PhantomData<&'a xcb::Connection>,
    data_ptr: NonNull<ImServerData>,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct DeadInputContextError;

pub type IcResult<T> = Result<T, DeadInputContextError>;

pub type TriggerKey = ffi::xcb_im_ximtriggerkey_fr_t;

#[derive(Debug, Clone, Copy)]
pub enum CommittedString<'a> {
    Chars(&'a CStr),
    KeySym(u32),
    Both(u32, &'a CStr),
}

#[derive(Default)]
struct ImServerData {
    im: Option<NonNull<ffi::xcb_im_t>>,
    callback: Option<Box<InternalImCallback>>,
    valid_ics: HashSet<InputContext>,
}

type InternalImCallback = dyn FnMut(&ImServer, ImClient, InputContext, ImMessage);

impl<'a> ImServer<'a> {
    fn new(data_ptr: NonNull<ImServerData>) -> Self {
        ImServer {
            conn: Default::default(),
            data_ptr,
        }
    }

    pub fn create<E>(
        conn: &'a xcb::Connection,
        screen: i32,
        server_window: xcb::Window,
        server_name: &CStr,
        locale: &CStr,
        input_styles: &[ffi::xcb_im_style_t], // TODO: style bitflag
        on_keys_list: &[TriggerKey],
        off_keys_list: &[TriggerKey],
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

        ImServer::new(NonNull::new(data_ptr).expect("data_ptr is null"))
    }

    fn get_data(&self) -> &mut ImServerData {
        unsafe { Box::leak(Box::from_raw(self.data_ptr.as_ptr())) }
    }

    pub fn get_im_ptr(&self) -> NonNull<ffi::xcb_im_t> {
        self.get_data().im.unwrap()
    }

    pub fn set_callback<'b, F>(&'b mut self, mut callback: F)
    where
        F: FnMut(&'b ImServer<'a>, ImClient, InputContext, ImMessage) + 'static,
    {
        let internal_callback = move |im: &ImServer, client, ic, msg: ImMessage| {
            // Cast away lifetimes
            unsafe { callback(mem::transmute(im), client, ic, msg) }
        };

        self.get_data().callback = Some(Box::new(internal_callback));
    }

    pub fn is_alive_ic(&self, ic: &InputContext) -> bool {
        self.get_data().valid_ics.contains(ic)
    }

    fn check_ic(&self, ic: &InputContext) -> IcResult<()> {
        match self.is_alive_ic(ic) {
            true => Ok(()),
            false => Err(DeadInputContextError),
        }
    }

    pub fn open(&mut self) -> Result<(), ()> {
        match unsafe { ffi::xcb_im_open_im(self.get_im_ptr().as_ptr()) } {
            true => Ok(()),
            false => Err(()),
        }
    }

    pub fn close(&mut self) {
        unsafe { ffi::xcb_im_close_im(self.get_im_ptr().as_ptr()) }
    }

    pub fn filter_event(&self, event: &xcb::GenericEvent) -> bool {
        unsafe { ffi::xcb_im_filter_event(self.get_im_ptr().as_ptr(), event.ptr) }
    }

    pub fn forward_event(&self, ic: &InputContext, event: &xcb::KeyPressEvent) -> IcResult<()> {
        self.check_ic(ic)?;
        unsafe { ffi::xcb_im_forward_event(self.get_im_ptr().as_ptr(), ic.as_ptr(), event.ptr) }
        Ok(())
    }

    pub fn commit_string(
        &self,
        ic: &InputContext,
        committed_str: &CommittedString,
    ) -> IcResult<()> {
        self.check_ic(ic)?;

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
                self.get_im_ptr().as_ptr(),
                ic.as_ptr(),
                flag,
                s as *mut c_char,
                len as u32,
                keysym,
            )
        }

        Ok(())
    }

    pub fn geometry_callback(&self, ic: &InputContext) -> IcResult<()> {
        self.check_ic(ic)?;
        unsafe { ffi::xcb_im_geometry_callback(self.get_im_ptr().as_ptr(), ic.as_ptr()) }
        Ok(())
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
    let data_ptr = NonNull::new(user_data as *mut ImServerData).expect("user_data is null");
    let data_cell = Box::leak(Box::from_raw(data_ptr.as_ptr()));

    match data_cell.im {
        Some(p) if p.as_ptr() == im => (),
        _ => panic!("Unknown im"),
    }

    if let Some(ref mut callback) = data_cell.callback {
        // Maintain ICs set
        let ic = InputContext(NonNull::new(ic).expect("ic is null"));
        let msg = super::im_message::parse_message(hdr, frame, arg);
        match msg {
            ImMessage::CreateIc(_, _) => {
                data_cell.valid_ics.insert(ic);
            }
            ImMessage::DestroyIc => {
                data_cell.valid_ics.remove(&ic);
            }
            _ => (),
        }

        let im_server = ImServer::new(data_ptr);
        let client = ImClient(NonNull::new(client).expect("client is null"));

        callback(&im_server, client, ic, msg);
        mem::forget(im_server);
    }
}

impl<'a> Drop for ImServer<'a> {
    fn drop(&mut self) {
        unsafe {
            let data = Box::from_raw(self.data_ptr.as_ptr());
            if let Some(im) = data.im {
                ffi::xcb_im_destroy(im.as_ptr())
            }
        }
    }
}

impl<'a> fmt::Debug for ImServer<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("ImServer")
            .field("data", self.get_data())
            .finish()
    }
}

impl fmt::Debug for ImServerData {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("ImServerData")
            .field("im", &self.im)
            .field(
                "callback",
                &match self.callback {
                    Some(_) => "Some",
                    None => "None",
                },
            )
            .field("valid_ics", &self.valid_ics)
            .finish()
    }
}

impl std::error::Error for DeadInputContextError {
    fn description(&self) -> &str {
        "the specified InputContext is dead"
    }
}

impl fmt::Display for DeadInputContextError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(std::error::Error::description(self))
    }
}
