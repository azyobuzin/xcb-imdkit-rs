// Ported from https://github.com/fcitx/xcb-imdkit/blob/bb2f10c4754223bc5afaacab7a6417ee0998e303/test/test_server.c

use std::cell::Cell;
use std::ffi::CString;
use std::rc::Rc;
use xcb_imdkit::imdkit::*;
use xcb_util::keysyms::KeySymbols;

const TEST_STRING: &'static str = "hello world你好世界켐ㅇㄹ貴方元気？☺";

fn main() {
    let (conn, screen_num) = xcb::Connection::connect(None).expect("failed to connect");
    let screen = conn.get_setup().roots().nth(screen_num as usize).unwrap();
    let key_symbols = KeySymbols::new(&conn);

    let w = conn.generate_id();
    xcb::create_window(
        &conn,
        xcb::COPY_FROM_PARENT as u8,
        w,
        screen.root(),
        0,
        0,
        1,
        1,
        1,
        xcb::WINDOW_CLASS_INPUT_OUTPUT as u16,
        screen.root_visual(),
        &[],
    );

    let style_array = [
        InputStyle::PREEDIT_POSITION | InputStyle::STATUS_AREA, // OverTheSpot
        InputStyle::PREEDIT_POSITION | InputStyle::STATUS_NOTHING, // OverTheSpot
        InputStyle::PREEDIT_POSITION | InputStyle::STATUS_NONE, // OverTheSpot
        InputStyle::PREEDIT_NOTHING | InputStyle::STATUS_NOTHING, // Root
        InputStyle::PREEDIT_NOTHING | InputStyle::STATUS_NONE,  // Root
    ];

    let encoding_array = [CString::new("COMPOUND_TEXT").unwrap()];

    let keys = [XimTriggerKey {
        keysym: ' ' as u32,
        modifier: xcb::MOD_MASK_CONTROL,
        modifier_mask: xcb::MOD_MASK_CONTROL,
    }];

    let end = Rc::new(Cell::default());

    let handler = TestServerHandler {
        end: end.clone(),
        key_symbols,
    };

    let mut im = ImServer::create(
        &conn,
        screen_num,
        w,
        &CString::new("test_server").unwrap(),
        all_locales(),
        &style_array,
        &keys,
        &keys,
        &encoding_array,
        0,
        handler,
    );

    im.close_on_drop(true);
    im.open().expect("failed to open IM");

    println!("winid:{}", w);

    while let Some(event) = conn.wait_for_event() {
        im.filter_event(&event);
        if end.get() {
            break;
        }
    }

    if let Err(e) = conn.has_error() {
        eprintln!("{}", e);
    }
}

struct TestServerHandler<'a> {
    #[allow(dead_code)]
    end: Rc<Cell<bool>>,
    key_symbols: KeySymbols<'a>,
}

impl<'a> ImMessageHandler for TestServerHandler<'a> {
    fn handle_disconnect(&mut self, _: &ImServerRef, _: &ImClient) {
        //self.end.set(true)
    }

    fn handle_forward_event(
        &mut self,
        im: &ImServerRef,
        _client: &ImClient,
        ic: &InputContext,
        _frame: &ForwardEventMessage,
        key_event: &xcb::KeyPressEvent,
    ) {
        let sym = self.key_symbols.press_lookup_keysym(key_event, 0);
        if sym == 't' as xcb::Keysym {
            let result = xcb_imdkit::encoding::utf8_to_compound_text(TEST_STRING.as_bytes())
                .expect("failed to convert to CTEXT");
            im.commit_string(ic, &CommittedString::Chars(&result));
        } else {
            im.forward_event(ic, key_event);
        }
    }
}
