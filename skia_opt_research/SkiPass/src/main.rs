use std::fs::File;
use std::fs;
use std::io::BufReader;
use std::env;
use ski_pass::ski_lang::{SkiLangExpr, parse_skp_json_file, optimize, SkpJsonParseError};
use ski_pass::skpicture::{SkPicture, print_skp, generate_skpicture, write_skp};

use ski_pass::protos;
use prost::Message;

fn write_run_info_to_disk(run_info : &protos::SkiPassRunInfo) -> Result<(), Box<dyn std::error::Error>>  {
    let mut run_info_pb_path = String::from(&run_info.optimized_skp_path);
    run_info_pb_path.push_str(".skipass_run.pb");
    let mut run_info_pb = Vec::new();
    run_info_pb.reserve(run_info.encoded_len());
    run_info.encode(&mut run_info_pb)?;
    fs::write(run_info_pb_path, run_info_pb)?;
    Ok(())
}

fn handle_skipass_run_error(e: &Box<dyn std::error::Error>, run_info_immut: &protos::SkiPassRunInfo) {
    let mut run_info = protos::SkiPassRunInfo::clone(run_info_immut);
    run_info.status = protos::SkiPassRunStatus::Failed as i32;
    let mut rust_error: protos::RustError = protos::RustError::default(); 
    rust_error.error_message = e.to_string();
    run_info.rust_error = Some(rust_error);
    match write_run_info_to_disk(&run_info) {
        Err(e) => panic!("Failed to write SkiPassRunInfo to disk. {}", e),
        Ok(_) => {} // There's nothing TODO here, maybe I'm not writing Idiomatic rust. TODO: Check
    };
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let skp_json_path = &args[1];
    let skp_out_path = &args[2];

    let mut run_info = protos::SkiPassRunInfo::default();
    run_info.input_skp_json_name = String::from(skp_json_path);
    run_info.optimized_skp_path = String::from(skp_out_path);

    // Run optimizer and write back as a SKP. 
    let skilang_expr = parse_skp_json_file(skp_json_path, &mut run_info)
                        .unwrap_or_else(|e| { handle_skipass_run_error(&e, &run_info); panic!("{}", e) });
    let optimized = optimize(&skilang_expr.expr, &mut run_info)
                        .unwrap_or_else(|e| { handle_skipass_run_error(&e, &run_info); panic!("{}", e) });
    write_skp(&optimized.expr, optimized.id, skp_out_path)
                        .unwrap_or_else(|e| { handle_skipass_run_error(&e, &run_info); panic!("{}", e) });

    run_info.status = protos::SkiPassRunStatus::Ok as i32;
    match write_run_info_to_disk(&run_info) {
        Err(e) => panic!("Failed to write SkiPassRunInfo to disk. {}", e),
        Ok(_) => {} // There's nothing TODO here, maybe I'm not writing Idiomatic rust. TODO: Check
    };
}
