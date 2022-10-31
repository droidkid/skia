use std::fs::File;
use std::io::BufReader;
use std::env;
use ski_opt::ski_lang::{parse_skp, optimize};
use ski_opt::skpicture::{SkPicture, print_skp, generate_skpicture, write_skp};

fn main() {
    let args: Vec<String> = env::args().collect();
    let skp_json_path = &args[1];
    let skp_out_path = &args[2];

    // Deserialize JSON
    let r= BufReader::new(File::open(skp_json_path).unwrap());
    let u: SkPicture = match serde_json::from_reader(r) {
        Ok(skp) => skp,
        Err(e) => panic!("Error {:?}", &e)
    };

    // Run optimizer and write back as a SKP. 
    let parse_result = parse_skp(&mut u.drawCommands.iter());
    let optimized = optimize(&parse_result.expr);
    write_skp(&optimized.expr, optimized.id, skp_out_path);
}
