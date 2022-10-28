use serde::Deserialize;
use crate::ski_lang::{SkiLang};
use egg::*;

#[derive(Deserialize, Debug, Clone)]
pub struct SkPaint {
    // ARGB, 0-255
    pub color: Vec<u8>
}

#[derive(Deserialize, Debug, Clone)]
#[serde(tag = "command")]
pub enum SkDrawCommand {
    DrawRect {coords: Vec<i32>, paint: SkPaint, visible: bool},
    SaveLayer {paint : Option<SkPaint>, visible: bool},
    Restore {visible: bool}
}

#[derive(Debug)]
pub enum SurfaceType {
    Abstract,
    Allocated
}

#[derive(Deserialize, Debug)]
pub struct SkPicture {
    #[serde(rename = "commands")]
    pub drawCommands: Vec<SkDrawCommand>,
    #[serde(skip)] // Only used internally.
    pub surfaceType: Option<SurfaceType>
}

struct Point { x: i32, y: i32 }

fn get_num(skilang_expr: &RecExpr<SkiLang>, id: Id) -> i32 {
    match &skilang_expr[id] {
        SkiLang::Num(val) => {
            *val
        }
        _ => {
            panic!("This is not a num!")
        }
    }
}

fn get_point(expr: &RecExpr<SkiLang>, id: Id) -> Point {
    match &expr[id] {
        SkiLang::Point(ids) => {
            let x_id = ids[0];
            let y_id = ids[1];

            Point {
                x: get_num(expr, x_id) ,
                y: get_num(expr, y_id)
            }
        }
        _ => {
            panic!("This is not a point!")
        }
    }
}

fn get_color(expr: &RecExpr<SkiLang>, id: Id) -> Vec<u8> {
    match &expr[id] {
        SkiLang::Color(ids) => {
            let a = get_num(expr, ids[0]) as u8;
            let r = get_num(expr, ids[1]) as u8;
            let g = get_num(expr, ids[2]) as u8;
            let b = get_num(expr, ids[3]) as u8;
            vec![a, r, g, b]
        }
        _ => {
            panic!("This is not a point!")
        }
    }
}

fn get_paint(expr: &RecExpr<SkiLang>, id: Id) -> SkPaint {
    match &expr[id] {
        SkiLang::Paint(ids) => {
            SkPaint {
                color: get_color(expr, ids[0])
            }
        }
        _ => {
            panic!("This is not a point!")
        }
    }
}

pub fn generate_skpicture(expr: &RecExpr<SkiLang>, id: Id) -> SkPicture {
    let node = &expr[id];
    match node {
        SkiLang::DrawRect(ids) => {
            let top_left = get_point(expr, ids[0]);
            let bot_rght = get_point(expr, ids[1]);
            let paint = get_paint(expr, ids[2]);
            SkPicture {
                drawCommands: vec![
                    SkDrawCommand::DrawRect {
                        coords: vec![top_left.x, top_left.y, bot_rght.x, bot_rght.y],
                        paint,
                        visible: true
                    }
                ],
                surfaceType: Some(SurfaceType::Abstract)
            }
        },
        SkiLang::SrcOver(ids) => {
            let mut dst = generate_skpicture(&expr, ids[0]);
            let mut src = generate_skpicture(&expr, ids[1]);

            let mut drawCommands: Vec<SkDrawCommand> = vec![];
            match src.surfaceType {
                Some(SurfaceType::Allocated) => {
                    drawCommands.append(&mut dst.drawCommands);
                    drawCommands.push(SkDrawCommand::SaveLayer{paint:None, visible: true});
                    drawCommands.append(&mut src.drawCommands);
                    drawCommands.push(SkDrawCommand::Restore{visible: true});
                },
                Some(SurfaceType::Abstract) => {
                    drawCommands.append(&mut dst.drawCommands);
                    drawCommands.append(&mut src.drawCommands);
                },
                None => {}
            }

            SkPicture {
                drawCommands,
                surfaceType: Some(SurfaceType::Allocated)
            }
        },
        SkiLang::Blank => {
            SkPicture {
                drawCommands: vec![],
                surfaceType: Some(SurfaceType::Allocated)
            }
        }
        _ => {
            panic!("The RecExpr looks off!")
        }

    }
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