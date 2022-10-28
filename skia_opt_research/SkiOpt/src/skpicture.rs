use serde::Deserialize;
use egg::*;
use skia_safe::{Color, Paint, Surface, Rect, PictureRecorder, canvas::SaveLayerRec};
use std::fs::File;
use std::io::Write;

use crate::ski_lang::{SkiLang};

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
            panic!("This is not a color!")
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
            panic!("This is not a paint!")
        }
    }
}

pub fn write_skp(expr: &RecExpr<SkiLang>, id: Id, file_path: &str) {
    let mut recorder = PictureRecorder::new();
    let canvas = recorder.begin_recording(Rect::new(0.0, 0.0, 512.0, 512.0), None);

    let skp = generate_skpicture(expr, id);
    for drawCommand in skp.drawCommands {
        match drawCommand {
            SkDrawCommand::DrawRect{coords, paint, visible:_} => {
                let r = Rect::new(coords[0] as f32, coords[1] as f32, coords[2] as f32, coords[3] as f32);
                let mut p = Paint::default();
                p.set_argb(paint.color[0], paint.color[1], paint.color[2], paint.color[3]);
                canvas.draw_rect(&r, &p);
            },
            SkDrawCommand::SaveLayer { paint:_, visible :_} => {
                canvas.save_layer(&SaveLayerRec::default());
            }
            SkDrawCommand::Restore { visible :_} => {
                canvas.restore();
            }
        }
    }

    let picture = recorder.finish_recording_as_picture(None).unwrap();
    let d = picture.serialize();
    let mut file = File::create(file_path).unwrap();
    let bytes = d.as_bytes();
    file.write_all(bytes).unwrap();

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