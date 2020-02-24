use crate::ffi::*;

pub type TriggerKey = xcb_im_ximtriggerkey_fr_t;

bitflags! {
    pub struct InputStyle: u32 {
        const PREEDIT_AREA = _xcb_im_style_t_XCB_IM_PreeditArea;
        const PREEDIT_CALLBACKS = _xcb_im_style_t_XCB_IM_PreeditCallbacks;
        const PREEDIT_POSITION = _xcb_im_style_t_XCB_IM_PreeditPosition;
        const PREEDIT_NOTHING= _xcb_im_style_t_XCB_IM_PreeditNothing;
        const PREEDIT_NONE = _xcb_im_style_t_XCB_IM_PreeditNone;
        const STATUS_AREA = _xcb_im_style_t_XCB_IM_StatusArea;
        const STATUS_CALLBACKS = _xcb_im_style_t_XCB_IM_StatusCallbacks;
        const STATUS_NOTHING = _xcb_im_style_t_XCB_IM_StatusNothing;
        const STATUS_NONE= _xcb_im_style_t_XCB_IM_StatusNone;
    }
}
