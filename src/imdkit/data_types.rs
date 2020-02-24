use crate::ffi::*;

pub type TriggerKey = xcb_im_ximtriggerkey_fr_t;
pub type PreeditAttr = xcb_im_preedit_attr_t;
pub type StatusAttr = xcb_im_status_attr_t;

#[derive(Debug, Clone)]
pub struct PreeditDrawMessage<'a> {
    pub caret: u32,
    pub chg_first: u32,
    pub chg_length: u32,
    pub status: DrawStatus,
    pub preedit_string: &'a [u8],
    pub feedback_array: &'a [Feedback],
}

#[derive(Debug, Clone, Copy)]
pub struct PreeditCaretMessage {
    pub position: u32,
    pub direction: CaretDirection,
    pub style: CaretStyle,
}

bitflags! {
    #[derive(Default)]
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

bitflags! {
    #[derive(Default)]
    pub struct DrawStatus: u32 {
        const NO_STRING = 1;
        const NO_FEEDBACK = 2;
    }
}

bitflags! {
    #[derive(Default)]
    pub struct Feedback: u32 {
        const REVERSE = xcb_im_feedback_t_XCB_XIM_REVERSE;
        const UNDERLINE = xcb_im_feedback_t_XCB_XIM_UNDERLINE;
        const HIGHLIGHT = xcb_im_feedback_t_XCB_XIM_HIGHLIGHT;
        const PRIMARY = xcb_im_feedback_t_XCB_XIM_PRIMARY;
        const SECONDARY = xcb_im_feedback_t_XCB_XIM_SECONDARY;
        const TERTIARY = xcb_im_feedback_t_XCB_XIM_TERTIARY;
        const VISIBLE_TO_FORWARD = xcb_im_feedback_t_XCB_XIM_VISIBLE_TO_FORWARD;
        const VISIBLE_TO_BACKWORD = xcb_im_feedback_t_XCB_XIM_VISIBLE_TO_BACKWORD;
        const VISIBLE_TO_CENTER = xcb_im_feedback_t_XCB_XIM_VISIBLE_TO_CENTER;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CaretDirection {
    ForwardChar = 0,
    BackwardChar = 1,
    ForwardWord = 2,
    BackwardWord = 3,
    CaretUp = 4,
    CaretDown = 5,
    NextLine = 6,
    PreviousLine = 7,
    LineStart = 8,
    LineEnd = 9,
    AbsolutePosition = 10,
    DontChange = 11,
}

impl Default for CaretDirection {
    fn default() -> Self {
        CaretDirection::ForwardChar
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CaretStyle {
    Invisible = 0,
    Primary = 1,
    Secondary = 2,
}

impl Default for CaretStyle {
    fn default() -> Self {
        CaretStyle::Invisible
    }
}
