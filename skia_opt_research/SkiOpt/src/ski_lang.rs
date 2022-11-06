use std::fs::File;
use std::io::BufReader;
use egg::*;
use std::error::Error;
use serde_json::{Value};
use strum::VariantNames;
use std::fmt;

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

fn make_rules() -> Vec<Rewrite<SkiLang, ()>> {
    vec![
        rewrite!("remove-blank-dst-savelayers"; "(srcOver ?a blank)" => "?a"),
        rewrite!("remove-blank-src-savelayers"; "(srcOver blank ?a)" => "?a"),
    ]
}

pub fn optimize(expr: &RecExpr<SkiLang>) -> SkiLangExpr {
    let mut runner = Runner::default().with_expr(expr).run(&make_rules());
    let root = runner.roots[0];

    // Eventually we'll want our own cost function.
    let extractor = Extractor::new(&runner.egraph, AstSize);
    let (cost, mut optimized) = extractor.find_best(root);

    // Figure out how to walk a RecExpr without the ID.
    // Until then, use this roundabout way to get the optimized recexpr id.
    let mut egraph = EGraph::<SkiLang, ()>::default();
    let id = egraph.add_expr(&optimized);

    SkiLangExpr {
        expr: optimized,
        id,
    }
}

pub struct SkiLangExpr {
    pub expr: RecExpr<SkiLang>,
    pub id: Id,
}

#[derive(Debug)]
pub struct SkpJsonParseError {
    pub unsupported_commands: Vec<String>
}

impl fmt::Display for SkpJsonParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Unsupported commands: {:?}", self.unsupported_commands)
    }
}

impl Error for SkpJsonParseError {
    fn description(&self) -> &str {
        "Error resulting from parsing Skp Json"
    }
}


pub fn parse_skp_json_file (
    skp_json_path: &str
) ->  Result<SkiLangExpr, Box<dyn Error> > {

    let r= BufReader::new(File::open(skp_json_path).unwrap());
    let u: Value = serde_json::from_reader(r)?;

    let mut drawCommands: Vec<SkDrawCommand> = vec![];
    let commandJsonArray = u["commands"].as_array().unwrap();
    let mut unsupported : Vec<String> = vec![];
    for commandJson in commandJsonArray {
        let commandName = commandJson["command"].as_str().unwrap();
        if SkDrawCommand::VARIANTS.contains(&commandName) {
            let drawCommand: SkDrawCommand = serde_json::from_str(&commandJson.to_string())?;
            drawCommands.push(drawCommand);
        } else {
            unsupported.push(commandName.to_string());
        }
    }

    if (!unsupported.is_empty()) {
        return Err(Box::new(SkpJsonParseError{
            unsupported_commands: unsupported
        }));
    }

    let mut expr = RecExpr::default();
    let blankSurface = expr.add(SkiLang::Blank);
    let id = build_expr(&mut u.drawCommands.iter(), blankSurface, &mut expr);

    Ok(SkiLangExpr {
        expr,
        id
    })

}


fn build_expr<'a, I> (
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
                    let newLayerId = build_expr(drawCommands, newLayerDst, expr);
                    expr.add(SkiLang::SrcOver([dst, newLayerId]))
                },
                Restore => {
                    dst
                }
            };
            build_expr(drawCommands, nxtDst, expr)
        },
        None => {
            dst
        }
    }
}