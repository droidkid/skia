use egg::*;
use serde_json::Value;
use std::error::Error;
use std::fmt;
use std::fs::File;
use std::io::BufReader;
use strum::VariantNames;
use prost::Message;
use ordered_float::OrderedFloat;

use crate::protos;
use crate::skpicture::{SkDrawCommand, SkPicture, SkPaint, SkBBox};


// TODO: Find out why clipRect(Id, SkBBox) does not work.
// The compiler egg::LanguageChildren, FromStr not implemented for Id, SkBBox.
// But why does BBox(SkBBox work then?)
define_language! {
    pub enum SkiLang {
        "clipRect" = ClipRect([Id; 2]), // layer, bbox
        "blank" = Blank,
        "srcOver" = SrcOver([Id; 2]), // dst, src
        // The commands below don't have string parsing implemented.
        BBox(SkBBox),
        DrawCommand(SkDrawCommand),
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

fn build_expr<'a, I>(drawCommands: &mut I, dst: Id, expr: &mut RecExpr<SkiLang>) -> Id
where
    I: Iterator<Item = &'a SkDrawCommand> + 'a,
{
    match drawCommands.next() {
        Some(drawCommand) => {
            match drawCommand {
                SkDrawCommand::DrawOval {..} |
                SkDrawCommand::DrawRect {..} => {
                    let drawOval = expr.add(SkiLang::DrawCommand(drawCommand.clone()));
                    let nextDst = expr.add(SkiLang::SrcOver([dst, drawOval]));

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
                    let l = coords[0];
                    let t = coords[1];
                    let r = coords[2];
                    let b = coords[3];

                    let skBBox = SkBBox {l, t, r, b};
                    let skBBoxId = expr.add(SkiLang::BBox(skBBox));

                    let newLayerDst = expr.add(SkiLang::Blank);
                    let newLayerId = build_expr(drawCommands, newLayerDst, expr);
                    let clipRect = expr.add(SkiLang::ClipRect([newLayerId, skBBoxId]));

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
