use std::fs::File;
use std::fs;
use std::io::BufReader;
use std::env;
use ski_opt::ski_lang::{SkiLangExpr, parse_skp_json_file, optimize};
use ski_opt::skpicture::{SkPicture, print_skp, generate_skpicture, write_skp};

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
}
