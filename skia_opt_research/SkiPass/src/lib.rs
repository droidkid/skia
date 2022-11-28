#[macro_export]
pub mod ski_pass;
pub mod protos {
    include!(concat!(env!("OUT_DIR"), "/ski_pass_proto.rs"));
}

extern crate libc;
use libc::size_t;
use ffi_utils;
use std::slice;

use protos::{
    SkRecord, 
    SkiPassProgram, 
    SkiPassRunResult, 
    SkiPassRunInfo,
    ski_pass_run_info::SkiPassRunError,
    ski_pass_run_info::SkiPassRunStatus,
};
use prost::Message;

#[repr(C)]
pub struct SkiPassResultPtr {
    ptr: *mut u8,
    len: size_t,
}

#[no_mangle]
pub extern "C" fn ski_pass_optimize(data_ptr: *const u8, len: size_t) -> SkiPassResultPtr {
    let data_slice : &[u8]= unsafe {
        assert!(!data_ptr.is_null());
        slice::from_raw_parts(data_ptr, len as usize)
    };

    let mut skipass_run = SkiPassRunResult::default();

    match SkRecord::decode(data_slice) {
        Ok(sk_record) => {
            skipass_run = ski_pass::optimize(sk_record);
        }
        Err(e) => {
            let mut run_info = SkiPassRunInfo::default();
            run_info.status = SkiPassRunStatus::Failed as i32;
            run_info.error = Some(SkiPassRunError {
                error_message: "Trouble decoding SkRecords proto bytes".to_string()
            });
            skipass_run.run_info = Some(run_info);
        }
    }

    let mut result_data: Vec<u8> = Vec::new();
    result_data.reserve(skipass_run.encoded_len());
    skipass_run.encode(&mut result_data);

    let (ptr, len) = ffi_utils::vec_into_raw_parts(result_data);
    SkiPassResultPtr { ptr, len }
}

#[no_mangle]
pub extern "C" fn free_ski_pass_result(result: SkiPassResultPtr) {
    unsafe {
        ffi_utils::vec_from_raw_parts(result.ptr, result.len);
    }
}

