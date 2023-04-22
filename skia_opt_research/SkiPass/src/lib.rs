pub mod sk_record_to_ski_lang;
pub mod ski_lang;
pub mod ski_lang_rules;
pub mod ski_lang_to_program;
pub mod ski_pass;
pub mod protos {
    include!(concat!(env!("OUT_DIR"), "/ski_pass_proto.rs"));
}

extern crate libc;
use ffi_utils;
use libc::size_t;
use std::slice;

use prost::Message;
use protos::SkRecord;

#[repr(C)]
pub struct SkiPassResultPtr {
    ptr: *mut u8,
    len: size_t,
}

#[no_mangle]
pub extern "C" fn ski_pass_optimize(data_ptr: *const u8, len: size_t) -> SkiPassResultPtr {
    let data_slice: &[u8] = unsafe {
        assert!(!data_ptr.is_null());
        slice::from_raw_parts(data_ptr, len as usize)
    };

    let skipass_run = match SkRecord::decode(data_slice) {
        Ok(sk_record) => ski_pass::optimize(sk_record),
        Err(_e) => panic!("Decoding input proto from Skia failed"),
    };

    let mut result_data: Vec<u8> = Vec::new();
    result_data.reserve(skipass_run.encoded_len());
    match skipass_run.encode(&mut result_data) {
        Ok(()) => {
            let (ptr, len) = ffi_utils::vec_into_raw_parts(result_data);
            SkiPassResultPtr { ptr, len }
        },
        Err(_) => {
            panic!("Failed to encode result data")
        }
    }
}

#[no_mangle]
pub extern "C" fn free_ski_pass_result(result: SkiPassResultPtr) {
    unsafe {
        ffi_utils::vec_from_raw_parts(result.ptr, result.len);
    }
}