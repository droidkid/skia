use std::fs::File;
use std::io::BufReader;
use std::env;
use ski_opt::ski_lang::{parse_skp};
use ski_opt::skpicture::{SkPicture, print_skp, generate_skpicture};

fn main() {
    let args: Vec<String> = env::args().collect();
    let skp_path = &args[1];

    let r= BufReader::new(File::open(skp_path).unwrap());
    let u: SkPicture = match serde_json::from_reader(r) {
        Ok(skp) => skp,
        Err(e) => panic!("Error {:?}", &e)
    };
    print_skp(&u);

    let parse_result = parse_skp(&mut u.drawCommands.iter());
    println!("{}", parse_result.expr.pretty(50));

    let skp = generate_skpicture(&parse_result.expr, parse_result.id);
    println!("{:#?}", skp);
}
