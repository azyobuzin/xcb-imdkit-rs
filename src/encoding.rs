use crate::ffi;
use std::error::Error;
use std::fmt;
use std::os::raw::c_char;
use std::slice;
use std::string::FromUtf8Error;
use std::sync::Once;

static INIT: Once = Once::new();

fn init() {
    INIT.call_once(|| unsafe {
        ffi::xcb_compound_text_init();
    });
}

pub fn utf8_to_compound_text(utf8: &[u8]) -> Result<Vec<u8>, ConvertError> {
    init();

    unsafe {
        let mut len_out = 0usize;
        let buf = ffi::xcb_utf8_to_compound_text(
            utf8.as_ptr() as *const c_char,
            utf8.len(),
            &mut len_out,
        );

        if buf.is_null() {
            // When failed, the function returns null
            return Err(ConvertError {
                convert_from: "UTF-8",
                convert_to: "Compound Text",
                inner: None,
            });
        }

        let vec = slice::from_raw_parts(buf as *mut u8, len_out).to_vec();
        libc::free(buf as *mut libc::c_void);
        Ok(vec)
    }
}

pub fn compound_text_to_utf8(compound_text: &[u8]) -> Result<String, ConvertError> {
    fn create_err(e: Option<FromUtf8Error>) -> ConvertError {
        ConvertError {
            convert_from: "Compound Text",
            convert_to: "UTF-8",
            inner: e,
        }
    }

    init();

    unsafe {
        let mut len_out = 0usize;
        let buf = ffi::xcb_compound_text_to_utf8(
            compound_text.as_ptr() as *const c_char,
            compound_text.len(),
            &mut len_out,
        );

        // When failed, the function returns null
        if buf.is_null() {
            return Err(create_err(None));
        }

        let vec = slice::from_raw_parts(buf as *mut u8, len_out).to_vec();
        libc::free(buf as *mut libc::c_void);
        String::from_utf8(vec).map_err(|e| create_err(Some(e)))
    }
}

#[derive(Debug)]
pub struct ConvertError {
    convert_from: &'static str,
    convert_to: &'static str,
    inner: Option<FromUtf8Error>,
}

impl ConvertError {
    pub fn as_bytes(&self) -> Option<&[u8]> {
        self.inner.as_ref().map(|e| e.as_bytes())
    }

    pub fn into_bytes(self) -> Option<Vec<u8>> {
        self.inner.map(|e| e.into_bytes())
    }
}

impl fmt::Display for ConvertError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "failed to convert from {} to {}",
            self.convert_from, self.convert_to
        )?;
        if let Some(e) = &self.inner {
            write!(f, ": {}", e)?;
        }
        Ok(())
    }
}

impl Error for ConvertError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.inner.as_ref().map(|e| e as _)
    }
}

// https://github.com/fcitx/xcb-imdkit/blob/bb2f10c4754223bc5afaacab7a6417ee0998e303/test/test_encoding.c
#[test]
fn shuttle_test() {
    fn test_conversion(s: &str) {
        let result = utf8_to_compound_text(s.as_bytes()).unwrap();
        let utf8_result = compound_text_to_utf8(&result).unwrap();
        assert_eq!(utf8_result, s);
    }

    test_conversion("hello world!你好世界켐ㅇㄹ貴方元気？☺");
    test_conversion(&String::from_utf8(vec![0xe2, 0x80, 0x93]).unwrap());
}
