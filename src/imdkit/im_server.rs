use super::*;
use crate::ffi;
use std::borrow::Borrow;
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
    close_on_drop: bool,
}

#[derive(PartialEq, Eq, Hash)]
pub struct ImServerRef(NonNull<ImServerData>);

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct DeadInputContextError;

pub type IcResult<T> = Result<T, DeadInputContextError>;

#[derive(Debug, Clone, Copy)]
pub enum CommittedString<'a> {
    KeySym(u32),
    Chars(&'a [u8]),
    Both(u32, &'a [u8]),
}

#[derive(Default)]
struct ImServerData {
    im: Option<NonNull<ffi::xcb_im_t>>,
    callback: Option<Box<dyn FnMut(&ImServerRef, CallbackArgs)>>,
    alive_ics: HashSet<InputContext>,
}

impl<'a> ImServer<'a> {
    pub fn create<E>(
        conn: &'a xcb::Connection,
        screen: i32,
        server_window: xcb::Window,
        server_name: &CStr,
        locale: &CStr,
        input_styles: &[InputStyle],
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

        ImServer {
            conn: Default::default(),
            data_ptr: NonNull::new(data_ptr).unwrap(),
            close_on_drop: false,
        }
    }

    pub fn set_callback<'b, F>(&'b mut self, callback: F)
    where
        F: FnMut(&ImServerRef, CallbackArgs) + 'static,
    {
        self.as_ref().get_data().callback = Some(Box::new(callback));
    }

    pub fn open(&mut self) -> Result<(), ()> {
        match unsafe { ffi::xcb_im_open_im(self.as_ref().get_im_ptr().as_ptr()) } {
            true => Ok(()),
            false => Err(()),
        }
    }

    pub fn close(&mut self) {
        unsafe { ffi::xcb_im_close_im(self.as_ref().get_im_ptr().as_ptr()) }
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
        _ => panic!("Unknown im"),
    }

    let raw_args = super::RawCallbackArgs {
        client: NonNull::new(client).map(ImClient),
        ic: NonNull::new(ic).map(InputContext),
        hdr,
        frame,
        arg,
    };

    let args = super::parse_callback_args(&raw_args);

    // Maintain alive ICs
    let destroyed_ic = match args.parsed {
        ImMessage::CreateIc { ic, .. } => {
            data_cell.alive_ics.insert(ic);
            None
        }
        ImMessage::DestroyIc { ic, .. } => Some(ic),
        _ => None,
    };

    // Call user callback
    if let Some(ref mut callback) = data_cell.callback {
        let im_ref = ImServerRef(data_ptr);
        callback(&im_ref, args);
    }

    if let Some(ic) = destroyed_ic {
        data_cell.alive_ics.remove(&ic);
    }
}

impl<'a> Drop for ImServer<'a> {
    fn drop(&mut self) {
        unsafe {
            let data = Box::from_raw(self.data_ptr.as_ptr());

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

    pub fn preedit_start_callback(&self, ic: &InputContext) -> IcResult<()> {
        self.check_ic(ic)?;
        unsafe { ffi::xcb_im_preedit_start_callback(self.get_im_ptr().as_ptr(), ic.as_ptr()) }
        Ok(())
    }

    pub fn preedit_draw_callback(
        &self,
        ic: &InputContext,
        frame: &PreeditDrawMessage,
    ) -> IcResult<()> {
        self.check_ic(ic)?;

        let mut frame = ffi::xcb_im_preedit_draw_fr_t {
            input_method_ID: 0,  // set by xcb-imdkit
            input_context_ID: 0, // set by xcb-imdkit
            caret: frame.caret,
            chg_first: frame.chg_first,
            chg_length: frame.chg_length,
            status: frame.status.bits(),
            length_of_preedit_string: frame.preedit_string.len() as u16,
            preedit_string: frame.preedit_string.as_ptr() as *mut u8,
            feedback_array: ffi::_xcb_im_preedit_draw_fr_t__bindgen_ty_1 {
                size: (frame.feedback_array.len() * mem::size_of::<u32>()) as u32,
                items: frame.feedback_array.as_ptr() as *mut u32,
            },
        };

        unsafe {
            ffi::xcb_im_preedit_draw_callback(self.get_im_ptr().as_ptr(), ic.as_ptr(), &mut frame)
        }

        Ok(())
    }

    pub fn preedit_caret_callback(
        &self,
        ic: &InputContext,
        frame: &PreeditCaretMessage,
    ) -> IcResult<()> {
        self.check_ic(ic)?;

        let mut frame = ffi::xcb_im_preedit_caret_fr_t {
            input_method_ID: 0,  // set by xcb-imdkit
            input_context_ID: 0, // set by xcb-imdkit
            position: frame.position,
            direction: frame.direction as u32,
            style: frame.style as u32,
        };

        unsafe {
            ffi::xcb_im_preedit_caret_callback(self.get_im_ptr().as_ptr(), ic.as_ptr(), &mut frame)
        }

        Ok(())
    }

    pub fn preedit_done_callback(&self, ic: &InputContext) -> IcResult<()> {
        self.check_ic(ic)?;
        unsafe { ffi::xcb_im_preedit_done_callback(self.get_im_ptr().as_ptr(), ic.as_ptr()) }
        Ok(())
    }

    pub fn preedit_start(&self, ic: &InputContext) -> IcResult<()> {
        self.check_ic(ic)?;
        unsafe { ffi::xcb_im_preedit_start(self.get_im_ptr().as_ptr(), ic.as_ptr()) }
        Ok(())
    }

    pub fn preedit_end(&self, ic: &InputContext) -> IcResult<()> {
        self.check_ic(ic)?;
        unsafe { ffi::xcb_im_preedit_end(self.get_im_ptr().as_ptr(), ic.as_ptr()) }
        Ok(())
    }

    pub fn sync_xlib(&self, ic: &InputContext) -> IcResult<()> {
        self.check_ic(ic)?;
        unsafe { ffi::xcb_im_sync_xlib(self.get_im_ptr().as_ptr(), ic.as_ptr()) }
        Ok(())
    }

    pub fn support_extension(&self, major_code: u16, minor_code: u16) -> bool {
        unsafe { ffi::xcb_im_support_extension(self.get_im_ptr().as_ptr(), major_code, minor_code) }
    }

    pub fn is_alive_ic(&self, ic: &InputContext) -> bool {
        self.get_data().alive_ics.contains(ic)
    }

    pub fn get_im_ptr(&self) -> NonNull<ffi::xcb_im_t> {
        self.get_data().im.unwrap()
    }

    fn get_data(&self) -> &mut ImServerData {
        unsafe { Box::leak(Box::from_raw(self.0.as_ptr())) }
    }

    fn check_ic(&self, ic: &InputContext) -> IcResult<()> {
        match self.is_alive_ic(ic) {
            true => Ok(()),
            false => Err(DeadInputContextError),
        }
    }
}

impl fmt::Debug for ImServerRef {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("ImServerRef")
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
            .field("alive_ics", &self.alive_ics)
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
