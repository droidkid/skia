use egg::*;
use serde_json::Value;
use std::error::Error;
use std::fmt;
use std::fs::File;
use std::io::BufReader;
use strum::VariantNames;

use crate::protos;
use prost::Message;

use crate::skpicture::{SkDrawCommand, SkPicture};

define_language! {
    pub enum SkiLang {
        Num(i32),
        "point" = Point([Id; 2]), // X, Y
        "dimensions" = Dim([Id; 2]), // W, H
        // TODO: Create a RECT type.
        "color" = Color([Id; 4]), // argb, 0-255
        "paint" = Paint([Id; 1]), // color
        "blank" = Blank, // dimensions
        "srcOver" = SrcOver([Id; 2]), // dst, src
        "drawRect" = DrawRect([Id; 3]), // top_point, bot_point, paint
        "drawOval" = DrawOval([Id; 3]), // top_point, bot_point, paint
        "clipRect" = ClipRect([Id; 3]), // top_point, bot_point, surface
    }
}

fn make_rules() -> Vec<Rewrite<SkiLang, ()>> {
    vec![
        rewrite!("remove-blank-dst-savelayers"; "(srcOver ?a blank)" => "?a"),
        rewrite!("remove-blank-src-savelayers"; "(srcOver blank ?a)" => "?a"),
    ]
}

pub fn optimize(
    expr: &RecExpr<SkiLang>,
    run_info: &mut protos::SkiPassRunInfo,
) -> Result<SkiLangExpr, Box<dyn Error>> {
    let mut runner = Runner::default().with_expr(expr).run(&make_rules());
    let root = runner.roots[0];

    // Eventually we'll want our own cost function.
    let extractor = Extractor::new(&runner.egraph, AstSize);
    let (cost, mut optimized) = extractor.find_best(root);

    // Figure out how to walk a RecExpr without the ID.
    // Until then, use this roundabout way to get the optimized recexpr id.
    let mut egraph = EGraph::<SkiLang, ()>::default();
    let id = egraph.add_expr(&optimized);
    run_info.ski_pass_parse_expr = optimized.pretty(50);

    Ok(SkiLangExpr {
        expr: optimized,
        id,
    })
}

pub struct SkiLangExpr {
    pub expr: RecExpr<SkiLang>,
    pub id: Id,
}

#[derive(Debug)]
pub struct UnsupportedDrawCommandsError {}

impl fmt::Display for UnsupportedDrawCommandsError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Unsupported commands. Check the run_info proto")
    }
}

impl Error for UnsupportedDrawCommandsError {
    fn description(&self) -> &str {
        "Error resulting from having unsupported draw commands."
    }
}

pub fn parse_skp_json_file(
    skp_json_path: &str,
    run_info: &mut protos::SkiPassRunInfo,
) -> Result<SkiLangExpr, Box<dyn Error>> {
    let r = BufReader::new(File::open(skp_json_path).unwrap());
    let u: Value = serde_json::from_reader(r)?;

    let mut unsupported_draw_commands = protos::UnsupportedDrawCommands::default();

    let mut drawCommands: Vec<SkDrawCommand> = vec![];
    let commandJsonArray = u["commands"].as_array().unwrap();
    for commandJson in commandJsonArray {
        let commandName = commandJson["command"].as_str().unwrap();
        if SkDrawCommand::VARIANTS.contains(&commandName) {
            let drawCommand: SkDrawCommand = serde_json::from_str(&commandJson.to_string())?;
            drawCommands.push(drawCommand);
        } else {
            unsupported_draw_commands.draw_commands.push(commandName.to_string());
        }
    }

    if !unsupported_draw_commands.draw_commands.is_empty() {
        return Err(Box::new(UnsupportedDrawCommandsError {}));
    }

    let mut expr = RecExpr::default();
    let blankSurface = expr.add(SkiLang::Blank);
    let id = build_expr(&mut drawCommands.iter(), blankSurface, &mut expr);

    run_info.skp_json_parse_expr = expr.pretty(50);

    Ok(SkiLangExpr { expr, id })
}

fn build_expr<'a, I>(drawCommands: &mut I, dst: Id, expr: &mut RecExpr<SkiLang>) -> Id
where
    I: Iterator<Item = &'a SkDrawCommand> + 'a,
{
    match drawCommands.next() {
        Some(drawCommand) => {
            match drawCommand {
                SkDrawCommand::DrawOval {
                    coords,
                    paint,
                    visible,
                } => {
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
                    let drawOval = expr.add(SkiLang::DrawOval([topPoint, botPoint, paint]));
                    let nextDst = expr.add(SkiLang::SrcOver([dst, drawOval]));

                    build_expr(drawCommands, nextDst, expr)
                }
                SkDrawCommand::DrawRect {
                    coords,
                    paint,
                    visible,
                } => {
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
                    let nextDst = expr.add(SkiLang::SrcOver([dst, drawRect]));

                    build_expr(drawCommands, nextDst, expr)
                }
                SkDrawCommand::Save { visible } => {
                    let newLayerDst = expr.add(SkiLang::Blank);
                    let newLayerId = build_expr(drawCommands, newLayerDst, expr);

                    let nextDst = expr.add(SkiLang::SrcOver([dst, newLayerId]));
                    build_expr(drawCommands, nextDst, expr)
                }
                SkDrawCommand::SaveLayer { paint, visible } => {
                    let newLayerDst = expr.add(SkiLang::Blank);
                    let newLayerId = build_expr(drawCommands, newLayerDst, expr);
                    let nextDst = expr.add(SkiLang::SrcOver([dst, newLayerId]));

                    build_expr(drawCommands, nextDst, expr)
                }
                SkDrawCommand::ClipRect { coords, visible } => {
                    let l = expr.add(SkiLang::Num(coords[0]));
                    let t = expr.add(SkiLang::Num(coords[1]));
                    let r = expr.add(SkiLang::Num(coords[2]));
                    let b = expr.add(SkiLang::Num(coords[3]));

                    let topPoint = expr.add(SkiLang::Point([l, t]));
                    let botPoint = expr.add(SkiLang::Point([r, b]));

                    let newLayerDst = expr.add(SkiLang::Blank);
                    let newLayerId = build_expr(drawCommands, newLayerDst, expr);
                    let clipRect = expr.add(SkiLang::ClipRect([topPoint, botPoint, newLayerId]));

                    expr.add(SkiLang::SrcOver([dst, clipRect]))
                    // Don't build further. Let it unroll upto save.
                    // What if there is no save? This means
                    // Either we we hit a restore with no save
                    // Or we reached the end of program
                    // TODO: check if we hit a error here...
                }
                Restore => dst,
            }
        }
        None => dst,
    }
}
