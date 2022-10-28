use serde::Deserialize;

#[derive(Deserialize, Debug, Clone)]
pub struct SkPaint {
    // ARGB, 0-255
    pub color: Vec<u8>
}

#[derive(Deserialize, Debug)]
#[serde(tag = "command")]
pub enum SkDrawCommand {
    DrawRect {coords: Vec<i32>, paint: SkPaint, visible: bool},
    SaveLayer {paint : Option<SkPaint>, visible: bool},
    Restore {visible: bool}
}

#[derive(Deserialize, Debug)]
pub struct SkPicture {
    #[serde(rename = "commands")]
    pub drawCommands: Vec<SkDrawCommand>,
}

pub fn print_skp(skp: &SkPicture) {
    print_commands(&mut skp.drawCommands.iter());
}

fn print_commands<'a, I>(mut drawCommands: &mut I)
where
    I: Iterator<Item = &'a SkDrawCommand> + 'a {
        match drawCommands.next() {
            Some(d) => {
                println!("{:?}", d);
                print_commands(drawCommands);
            },
            None => {}
        }
}