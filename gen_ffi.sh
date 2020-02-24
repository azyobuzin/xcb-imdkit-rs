#!/bin/sh

# Prerequires: cargo install bindgen
# Usage: ./gen_ffi.sh -- -Ipath/to/include

WHITELIST='((?i)_?xcb_(xim|im|xic)_.*|xcb_compound_text_.*|xcb_utf8_to_compound_text)'

RAW_LINE='#![allow(non_snake_case, non_camel_case_types, non_upper_case_globals)]
use xcb::ffi::base::*;
use xcb::ffi::xproto::*;'

bindgen --whitelist-function "${WHITELIST}" \
        --blacklist-function '.*_fr_\w+' \
        --whitelist-type "${WHITELIST}" \
        --whitelist-var "${WHITELIST}" \
        --no-recursive-whitelist \
        --size_t-is-usize \
        --impl-debug \
        --raw-line "${RAW_LINE}" \
        -o src/ffi.rs \
        wrapper.h "$@"
