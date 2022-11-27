#[macro_export]
pub mod ski_lang;
pub mod skpicture;
pub mod protos {
    include!(concat!(env!("OUT_DIR"), "/ski_pass.rs"));
}

extern crate libc;
use libc::size_t;
use ffi_utils;


#[repr(C)]
pub struct SkiPassResult {
    ptr: *mut u8,
    len: size_t,
}

#[no_mangle]
pub extern "C" fn ski_pass_optimize(data: *const u8, len: size_t) -> SkiPassResult {
    let mut result_data: Vec<u8> = vec![0, 1, 2, 3]; // Some dummy data.
    let (ptr, len) = ffi_utils::vec_into_raw_parts(result_data);
    SkiPassResult { ptr, len }
}

#[no_mangle]
pub extern "C" fn free_ski_pass_result(result: SkiPassResult) {
    unsafe {
        ffi_utils::vec_from_raw_parts(result.ptr, result.len);
    }
}

