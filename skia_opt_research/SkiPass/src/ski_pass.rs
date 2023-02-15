use egg::*;
use std::error::Error;
use std::fmt;
use prost::Message;
use std::fmt::Write;

use crate::protos;
use crate::protos::{
    SkPaint,
    SkColor,
    SkRecord, 
    SkRecords, 
    SkiPassInstruction,
    SkiPassProgram, 
    SkiPassRunInfo,
    SkiPassRunResult,
    ski_pass_instruction::SkiPassCopyRecord,
    ski_pass_instruction::Instruction,
    ski_pass_instruction::SaveLayer,
    ski_pass_instruction::Save,
    ski_pass_instruction::Restore,
    sk_records::Command, 
};

pub fn optimize(record: SkRecord) -> SkiPassRunResult {
    let mut expr = RecExpr::default();
    let blankSurface = expr.add(SkiLang::Blank);
    let id = build_expr(&mut record.records.iter(), blankSurface, createContext(0), &mut expr);

    let mut skiPassRunResult = SkiPassRunResult::default();
    let mut skiRunInfo = SkiPassRunInfo::default();

    match run_eqsat_and_extract(&expr, &mut skiRunInfo) {
        Ok(optExpr) => {
            let mut program = SkiPassProgram::default();
            program.instructions = build_program(&optExpr.expr, optExpr.id).instructions;
            skiPassRunResult.program = Some(program);
            skiPassRunResult.run_info = Some(skiRunInfo);
        }
        Err(e) => {}
    }
    skiPassRunResult
}

define_language! {
    enum SkiLang {
        Num(i32),
        "noOp" = NoOp,
        "blank" = Blank,
        "color" = Color([Id; 4]),
        // TODO: Consider not using Num as a drawCommand index and make a new type for that?
        // drawCommand(num, alpha), where
        //  num: index of drawCommand referenced in source SKP
        //  alpha: Alpha to apply on top of original drawCommand paint
        "drawCommand" = DrawCommand([Id; 2]),
        // matrixOp(layer, num) -> return layer after applying transform on layer 
        "matrixOp" = MatrixOp([Id; 2]),
        // concat(layer1, layer2) -> return layer resulting from sequential execution of
        // instructions(layer1), instructions(layer2)
        "concat" = Concat([Id; 2]),
        // merge(layer1, layer2, color, blendOrFilter)
        // This translates directly to saveLayer command in Skia.
        "merge" = Merge([Id; 4]),
        // EGRAPH INTERNAL COMMANDS FOLLOW
        // Below commands have no literal equivalent in Skia, and are only used for EGraph
        // Extraction
        // alpha(layer, value) -> apply transparency of value on layer
        "alpha" = Alpha([Id; 2]), // alphaChannel, layer
    }
}

fn make_rules() -> Vec<Rewrite<SkiLang, ()>> {
    vec![
        rewrite!("remove-noOp-concat-1"; "(concat blank ?a)" => "?a"),
        rewrite!("remove-noOp-concat-2"; "(concat ?a blank)" => "?a"),
        // Kill if only a single drawCommand, and saveLayer is noOp.
        // SaveLayer alpha might have been merged into single drawCommand.
        rewrite!("kill-merge"; 
                 "(merge ?dst (drawCommand ?x ?a) (color 255 0 0 0) noOp)" 
                 => "(concat ?dst (drawCommand ?x ?a))"),
        rewrite!("apply-alpha-on-src"; 
                 "(merge ?dst ?src (color ?a ?r ?g ?b) noOp)" 
                 => "(merge ?dst (alpha ?a ?src) (color 255 ?r ?g ?b) noOp)"),
        // TODO: For now we assume merge alpha is 255, eventually we probably want to multiply
        // alphas.
        rewrite!("lift-alpha"; 
                 "(merge ?dst (alpha ?a ?src) (color 255 ?r ?g ?b) noOp)" 
                 => "(merge ?dst ?src (color ?a ?r ?g ?b) noOp)"),
        rewrite!("remove-merge-blank"; "(merge ?layer blank ?color ?blend)" => "?layer"),
        rewrite!("remove-noOp-alpha"; "(alpha 255 ?src)" => "?src"),
        // TODO: Again, we assume merge alpha is 255, eventually we probably want to multiply
        // alphas.
        rewrite!("apply-alpha-on-draw"; "(alpha ?a (drawCommand ?x 255))" => "(drawCommand ?x ?a)"),
        rewrite!("remove-blank-matrixOp"; "(matrixOp blank ?a)" => "blank"),
        rewrite!("remove-noOp-matrixOp"; "(matrixOp ?layer noOp)" => "?layer"),
    ]
}

// This CostFn exists to prevent internal SkiLang functions (such as alpha) to never be extracted.
struct SkiLangCostFn;
impl CostFunction<SkiLang> for SkiLangCostFn {
    type Cost=f64;
    fn cost<C>(&mut self, enode: &SkiLang, mut costs: C) -> Self::Cost
        where
            C: FnMut(Id) -> Self::Cost
    {
        let op_cost = match enode {
            SkiLang::Alpha(ids) => 100000000.0,
            SkiLang::Merge(ids) => 1.0,
            _ => 0.0
        };
        enode.fold(op_cost, |sum, id| sum + costs(id))
    }
}


fn run_eqsat_and_extract(
    expr: &RecExpr<SkiLang>,
    run_info: &mut protos::SkiPassRunInfo,
    ) -> Result<SkiLangExpr, Box<dyn Error>> {
    let mut runner = Runner::default().with_expr(expr).run(&make_rules());
    let root = runner.roots[0];

    writeln!(&mut run_info.skilang_expr, "{:#}", expr);

    let extractor = Extractor::new(&runner.egraph, SkiLangCostFn);
    let (cost, mut optimized) = extractor.find_best(root);

    writeln!(&mut run_info.extracted_skilang_expr, "{:#}", optimized);

    // Figure out how to walk a RecExpr without the ID.
    // Until then, use this roundabout way to get the optimized recexpr id.
    let mut egraph = EGraph::<SkiLang, ()>::default();
    let id = egraph.add_expr(&optimized);

    Ok(SkiLangExpr {
        expr: optimized,
        id,
    })
}

struct SkiLangExpr {
    pub expr: RecExpr<SkiLang>,
    pub id: Id,
}

struct SkRecordParseContext {
    matrixOpCount: i32
}

fn createContext(matrixOpCount: i32) -> SkRecordParseContext {
   SkRecordParseContext {
       matrixOpCount
   }
}

fn build_expr<'a, I>(
    skRecordsIter: &mut I, 
    dst: Id, 
    context: SkRecordParseContext,
    expr: &mut RecExpr<SkiLang>
) -> Id
where
I: Iterator<Item = &'a SkRecords> + 'a,
{
    let mut drawStack: Vec<Id> = vec![];
    while true {
        match skRecordsIter.next() {
            Some(skRecords) => {
              let drawCommandIndex = expr.add(SkiLang::Num(skRecords.index));
              let drawCommandAlpha = expr.add(SkiLang::Num(255));
              let src = expr.add(SkiLang::DrawCommand([drawCommandIndex, drawCommandAlpha]));
              drawStack.push(src);
            },
        None => break,
        }
    }

    if drawStack.len() == 0 {
        return expr.add(SkiLang::Blank)
    }

    while drawStack.len() != 1 {
        let id_1 = drawStack.pop().unwrap();
        let id_2 = drawStack.pop().unwrap();
        let nxt = expr.add(SkiLang::Concat([id_2, id_1]));
        drawStack.push(nxt);
    }
    drawStack[0]
}

#[derive(Debug)]
struct SkiPassSurface {
    instructions: Vec<SkiPassInstruction>,
    modified_matrix: bool,
}

fn to_instructions(expr: &RecExpr<SkiLang>, id: Id) -> Vec<SkiPassInstruction> {
    let node = &expr[id];
    match node {
        SkiLang::NoOp => {
            vec![]
        },
        SkiLang::DrawCommand(ids) => {
            let index = match &expr[ids[0]] {
                SkiLang::Num(value) => *value,
                _ => panic!()
            };
            let alpha = match &expr[ids[1]] {
                SkiLang::Num(value) => *value,
                _ => panic!()
            };
            let instruction = SkiPassInstruction {
                // oneof -> Option of a Enum
                instruction: Some(Instruction::CopyRecord(
                    SkiPassCopyRecord {
                        index,
                        alpha
                    }
                ))
            };
            vec![instruction]
        },
        _ => {
            panic!("Not a instruction, this is a Surface!");
        }
    }
}

fn build_program(expr: &RecExpr<SkiLang>, id: Id) -> SkiPassSurface {
    let node = &expr[id];
    match node {
        SkiLang::Blank => {
            SkiPassSurface {
                instructions: vec![],
                modified_matrix: false,
            }
        },
        SkiLang::DrawCommand(ids) => {
            SkiPassSurface {
                instructions: to_instructions(&expr, id),  // NOTE: id, not ids[0] -> we are parsing the command, not its args
                modified_matrix: false,
            }
        },
        SkiLang::MatrixOp(ids) => {
            let mut targetSurface = build_program(&expr, ids[0]);
            let mut matrixOpInstructions = to_instructions(&expr, ids[1]);

            let mut instructions: Vec<SkiPassInstruction> = vec![];
            instructions.append(&mut matrixOpInstructions);
            instructions.append(&mut targetSurface.instructions);

            SkiPassSurface {
                instructions,
                modified_matrix: true,
            }
        },
        SkiLang::Concat(ids) => {
            let mut p1 = build_program(&expr, ids[0]);
            let mut p2 = build_program(&expr, ids[1]);

            let mut instructions: Vec<SkiPassInstruction> = vec![];

            if p1.modified_matrix {
                instructions.push(SkiPassInstruction {
                    instruction: Some(Instruction::Save(Save{}))
                });
                instructions.append(&mut p1.instructions);
                instructions.push(SkiPassInstruction {
                    instruction: Some(Instruction::Restore(Restore{}))
                });
            } else {
                instructions.append(&mut p1.instructions);
            }

            if p2.modified_matrix {
                instructions.push(SkiPassInstruction {
                    instruction: Some(Instruction::Save(Save{}))
                });
                instructions.append(&mut p2.instructions);
                instructions.push(SkiPassInstruction {
                    instruction: Some(Instruction::Restore(Restore{}))
                });
            } else {
                instructions.append(&mut p2.instructions);
            }

            SkiPassSurface {
                instructions,
                modified_matrix: false
            }
        },
        SkiLang::Merge(ids) => {
            let mut dst = build_program(&expr, ids[0]);
            let mut src = build_program(&expr, ids[1]);

            let mut instructions: Vec<SkiPassInstruction> = vec![];
            if dst.modified_matrix {
                instructions.push(SkiPassInstruction {
                    instruction: Some(Instruction::Save(Save{}))
                });
                instructions.append(&mut dst.instructions);
                instructions.push(SkiPassInstruction {
                    instruction: Some(Instruction::Restore(Restore{}))
                });
            } else {
                instructions.append(&mut dst.instructions);
            }

            let mut blendInstructions = to_instructions(&expr, ids[3]);

            if blendInstructions.len() > 0 {
                // Not enough is known about the blend operation, use original saveLayer command.
                // NOTE: This assumption is only valid as long as our rewrite rules don't touch
                // Merges with a blend operation.
                // TODO: Eventually capture the blend parameters and reconstruct the SaveLayer.
                // and stop relying on referencing the original saveLayer.
                instructions.append(&mut blendInstructions);
                instructions.append(&mut src.instructions);
                instructions.push(SkiPassInstruction {
                    instruction: Some(Instruction::Restore(Restore{}))
                });
            } else {
                // Construct a new SaveLayer.
                // We only have a alpha parameter to worry about.
                instructions.push(SkiPassInstruction {
                    instruction: Some(Instruction::SaveLayer(
                                         SaveLayer{
                                             paint: Some(SkPaint{
                                                 color: Some(color_proto_from_expr(expr, ids[2])), 
                                                 filter_info: None,
                                             }),
                                             suggested_bounds: None
                                         }))
                });
                instructions.append(&mut src.instructions);
                instructions.push(SkiPassInstruction {
                    instruction: Some(Instruction::Restore(Restore{}))
                });
            };
            
            SkiPassSurface {
                instructions,
                modified_matrix: false
            }

        },
        SkiLang::Alpha(ids) => {
            panic!("An Alpha survived extraction! THIS SHOULD NOT HAPPEN");
        },
        _ => {
            panic!("Badly constructed Recexpr");
        }
    }
}

fn color_expr(expr: &mut RecExpr<SkiLang>, aVal:i32, rVal:i32, gVal:i32, bVal:i32) -> Id {
    let a = expr.add(SkiLang::Num(aVal));
    let r = expr.add(SkiLang::Num(rVal));
    let g = expr.add(SkiLang::Num(gVal));
    let b = expr.add(SkiLang::Num(bVal));
    expr.add(SkiLang::Color([a, r, g, b]))
}


fn color_proto_from_expr(expr: &RecExpr<SkiLang>, id: Id) -> SkColor {
    let node = &expr[id];
    match node {
        SkiLang::Color(ids) => {
            let alpha_u8  = match &expr[ids[0]] {
                SkiLang::Num(value) => *value,
                _ => panic!()
            };
            let red_u8  = match &expr[ids[1]] {
                SkiLang::Num(value) => *value,
                _ => panic!()
            };
            let green_u8  = match &expr[ids[2]] {
                SkiLang::Num(value) => *value,
                _ => panic!()
            };
            let blue_u8  = match &expr[ids[3]] {
                SkiLang::Num(value) => *value,
                _ => panic!()
            };
    
            SkColor {
              alpha_u8,
              red_u8,
              green_u8,
              blue_u8
            }
        },
        _ => {
            panic!("Not a Color!");
        }
    }
}
