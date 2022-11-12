use std::fs::File;
use std::fs;
use std::io::BufReader;
use std::env;
use ski_pass::ski_lang::{SkiLangExpr, parse_skp_json_file, optimize};
use ski_pass::skpicture::{SkPicture, print_skp, generate_skpicture, write_skp};
use ski_pass::protos;

use prost::Message;

fn main() {
    let args: Vec<String> = env::args().collect();
    let skp_json_path = &args[1];
    let skp_out_path = &args[2];


    // Run optimizer and write back as a SKP. 
    let skilang_expr = match parse_skp_json_file(skp_json_path) {
        Ok(expr) => expr,
        Err(e) => {
            let error_log = [skp_json_path, ".error_log.txt"].join("");
            fs::write(error_log, e.to_string());
            println!("Error parsing SKP {} ", e);
            return;
        }
    };
    println!("SKP Parse Result");
    println!("{}", skilang_expr.expr.pretty(50));
    let optimized = optimize(&skilang_expr.expr);

    println!("Optimized SKP");
    println!("{} {}", optimized.expr.pretty(50), optimized.id);
    write_skp(&optimized.expr, optimized.id, skp_out_path);

    let mut test_proto = protos::SkiPassRunInfo::default();
    test_proto.input_skp_name = String::from(skp_json_path);

    let mut ski_pass_run_info_path = String::from(skp_out_path);
    ski_pass_run_info_path.push_str(".skipass_run.pb");
    let mut ski_pass_run_info = Vec::new();
    ski_pass_run_info.reserve(test_proto.encoded_len());
    match test_proto.encode(&mut ski_pass_run_info) {
        Ok(_) => fs::write(ski_pass_run_info_path, ski_pass_run_info).unwrap(),
        Err(e) => panic!("Failed writing proto! {}", e)
    };
}
