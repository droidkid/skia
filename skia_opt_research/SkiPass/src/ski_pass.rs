use egg::*;
use std::error::Error;
use std::fmt;
use prost::Message;
use std::fmt::Write;

use crate::protos;
use crate::protos::{
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
    let id = build_expr(&mut record.records.iter(), blankSurface, 0, &mut expr);

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
        // merge(layer1, layer2, alpha, blend) -> using blend, combine layer1 and alpha(layer2).
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
        rewrite!("kill-merge"; "(merge ?dst ?src 255 noOp)" => "(concat ?dst ?src)"),
        rewrite!("apply-alpha-on-src"; "(merge ?dst ?src ?a noOp)" => "(merge ?dst (alpha ?a ?src) 255 noOp)"),
        // TODO: For now we assume merge alpha is 255, eventually we probably want to multiply
        // alphas.
        rewrite!("lift-alpha"; "(merge ?dst (alpha ?a ?src) 255 noOp)" => "(merge ?dst ?src ?a noOp)"),
        rewrite!("remove-merge-blank"; "(merge ?layer blank ?alpha ?blend)" => "?layer"),
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

fn build_expr<'a, I>(skRecordsIter: &mut I, dst: Id, matrixOpCount: i32, expr: &mut RecExpr<SkiLang>) -> Id
where
I: Iterator<Item = &'a SkRecords> + 'a,
{
    match skRecordsIter.next() {
        Some(skRecords) => {
            match &skRecords.command {
                Some(Command::DrawCommand(draw_command)) => {
                    match draw_command.name.as_str() {
                        "ClipPath" | "ClipRRect" | "ClipRect" | "Concat44" => {
                            // Reference to matrixOp command in input SKP.
                            let matrixOpIndex = expr.add(SkiLang::Num(skRecords.index));
                            let matrixOpAlpha = expr.add(SkiLang::Num(255));
                            let matrixOpCommand = expr.add(SkiLang::DrawCommand([matrixOpIndex, matrixOpAlpha]));
                            // Construct surface on which matrixOp should be applied.
                            //
                            let newLayerDst = expr.add(SkiLang::Blank);
                            let surfaceToApplyMatrixOp = build_expr(skRecordsIter, newLayerDst, matrixOpCount + 1, expr);

                            let src = expr.add(SkiLang::MatrixOp([surfaceToApplyMatrixOp, matrixOpCommand]));
                            let nextDst = expr.add(SkiLang::Concat([dst, src]));

                            // We do this to find the matching Restore
                            // You could have a Save ClipRect Concat44 Draw Restore
                            if matrixOpCount == 0 {
                                build_expr(skRecordsIter, nextDst, matrixOpCount, expr)
                            } else {
                                nextDst // Need to unwind a bit more
                            }

                        }
                        _ => {
                            let drawCommandIndex = expr.add(SkiLang::Num(skRecords.index));
                            let drawCommandAlpha = expr.add(SkiLang::Num(255));
                            let src = expr.add(SkiLang::DrawCommand([drawCommandIndex, drawCommandAlpha]));
                            let nextDst = expr.add(SkiLang::Concat([dst, src]));
                            build_expr(skRecordsIter, nextDst, matrixOpCount, expr)
                        }
                    }
                },
                Some(Command::Save(save)) => {
                    // Ignore, MatrixOp is used to decide where to put Save commands.
                    build_expr(skRecordsIter, dst, 0, expr)
                },
                Some(Command::SaveLayer(save_layer)) => {
                    // Build the layer over a blank canvas.
                    let blank = expr.add(SkiLang::Blank);
                    let src = build_expr(skRecordsIter, blank, 0, expr);

                    let has_bounds = save_layer.suggested_bounds.is_some();
                    let has_filters = save_layer.filter_info.is_some();
                    let has_backdrop = save_layer.backdrop.is_some();
                    let alpha = expr.add(SkiLang::Num(save_layer.alpha_u8));

                    let blendOp = if has_bounds || has_filters || has_backdrop {
                        let blendOpIndex = expr.add(SkiLang::Num(skRecords.index));
                        let blendOpAlpha = expr.add(SkiLang::Num(255));
                        expr.add(SkiLang::DrawCommand([blendOpIndex, blendOpAlpha]))
                    } else {
                        expr.add(SkiLang::NoOp)
                    };

                    let nextDst = expr.add(SkiLang::Merge([dst, src, alpha, blendOp]));
                    build_expr(skRecordsIter, nextDst, matrixOpCount, expr)
                },
                Some(Command::Restore(restore)) => {
                    // Either SaveLayer or the first MatrixOp that matches this will
                    // continue building the program.
                    dst
                },
                _ => {
                    panic!("Not implemented yet!")
                },
                None => {
                    panic!("Empty command!")
                }
            }
        }
        None => dst,
    }
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

            let alpha_u8  = match &expr[ids[2]] {
                SkiLang::Num(value) => *value,
                _ => panic!()
            };

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
                    instruction: Some(Instruction::SaveLayer(SaveLayer{alpha_u8}))
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
