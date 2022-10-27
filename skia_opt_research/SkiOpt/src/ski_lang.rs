use egg::*;

use crate::skpicture::{SkPicture, SkDrawCommand};

define_language! {
    pub enum SkiLang {
        Num(i32),
        "point" = Point([Id; 2]), // X, Y
        "dimensions" = Dim([Id; 2]), // W, H
        "color" = Color([Id; 4]), // argb, 0-255
        "paint" = Paint([Id; 1]), // color
        "blank" = Blank, // dimensions
        "srcOver" = SrcOver([Id; 2]), // dst, src
        "drawRect" = DrawRect([Id; 3]), // top_point, bot_point, paint
    }
}

pub fn parse_skp<'a, I> (
    drawCommands: &mut I
) -> RecExpr<SkiLang> 
where
    I: Iterator<Item = &'a SkDrawCommand> + 'a {
    let mut expr = RecExpr::default();
    let blankSurface = expr.add(SkiLang::Blank);
    build_exp(drawCommands, blankSurface, &mut expr);
    expr
}

fn build_exp<'a, I> (
    drawCommands: &mut I,
    dst: Id,
    expr: &mut RecExpr<SkiLang>
) -> Id 
where
    I: Iterator<Item = &'a SkDrawCommand> + 'a
{
    match drawCommands.next() {
        Some(drawCommand) => {
            let nxtDst = match drawCommand {
                SkDrawCommand::DrawRect { coords, paint, visible } => {
                    let l = expr.add(SkiLang::Num(coords[0]));
                    let t = expr.add(SkiLang::Num(coords[1]));
                    let r = expr.add(SkiLang::Num(coords[2]));
                    let b = expr.add(SkiLang::Num(coords[3]));

                    let ca = expr.add(SkiLang::Num(paint.color[0] as i32));
                    let cr = expr.add(SkiLang::Num(paint.color[1] as i32));
                    let cg = expr.add(SkiLang::Num(paint.color[2] as i32));
                    let cb = expr.add(SkiLang::Num(paint.color[3] as i32));

                    let topPoint = expr.add(SkiLang::Point([l, t]));
                    let botPoint = expr.add(SkiLang::Point([r, b]));
                    let color = expr.add(SkiLang::Color([ca, cr, cg, cb]));

                    let paint = expr.add(SkiLang::Paint([color]));
                    let drawRect = expr.add(SkiLang::DrawRect([topPoint, botPoint, paint]));

                    expr.add(SkiLang::SrcOver(([dst, drawRect])))
                },
                SkDrawCommand::SaveLayer {paint, visible} => {
                    let newLayerDst = expr.add(SkiLang::Blank);
                    let newLayerId = build_exp(drawCommands, newLayerDst, expr);
                    expr.add(SkiLang::SrcOver([dst, newLayerId]))
                },
                Restore => {
                    dst
                }
            };
            build_exp(drawCommands, nxtDst, expr)
        },
        None => {
            dst
        }
    }
}