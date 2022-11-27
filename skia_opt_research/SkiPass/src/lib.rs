#[macro_export]
pub mod ski_lang;
pub mod skpicture;
pub mod protos {
    include!(concat!(env!("OUT_DIR"), "/ski_pass_proto.rs"));
}

extern crate libc;
use libc::size_t;
use ffi_utils;
use std::slice;

use protos::{SkRecord, SkiPassProgram};
use prost::Message;

#[repr(C)]
pub struct SkiPassResult {
    ptr: *mut u8,
    len: size_t,
}

#[no_mangle]
pub extern "C" fn ski_pass_optimize(data_ptr: *const u8, len: size_t) -> SkiPassResult {
    let data_slice : &[u8]= unsafe {
        assert!(!data_ptr.is_null());
        slice::from_raw_parts(data_ptr, len as usize)
    };

    let sk_record = SkRecord::decode(data_slice);
    // TODO: pass through optimizer.
    let ski_pass_program = SkiPassProgram::default();

    let mut result_data: Vec<u8> = Vec::new();
    result_data.reserve(ski_pass_program.encoded_len());
    ski_pass_program.encode(&mut result_data);

    let (ptr, len) = ffi_utils::vec_into_raw_parts(result_data);
    SkiPassResult { ptr, len }
}

#[no_mangle]
pub extern "C" fn free_ski_pass_result(result: SkiPassResult) {
    unsafe {
        ffi_utils::vec_from_raw_parts(result.ptr, result.len);
    }
}

