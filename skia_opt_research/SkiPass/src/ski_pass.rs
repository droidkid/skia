use egg::*;
use std::error::Error;
use std::fmt;
use prost::Message;

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
        }
        Err(e) => {}
    }
    skiPassRunResult
}

define_language! {
    enum SkiLang {
        DrawCommand(i32), // skRecords index
        "matrixOp" = MatrixOp([Id; 2]), // layer to apply matrixOp, command referring original clipRect
        "blank" = Blank,
        "srcOver" = SrcOver([Id; 2]), // dst, src
    }
}

fn make_rules() -> Vec<Rewrite<SkiLang, ()>> {
    vec![
        rewrite!("remove-blank-dst-savelayers"; "(srcOver ?a blank)" => "?a"),
        rewrite!("remove-blank-src-savelayers"; "(srcOver blank ?a)" => "?a"),
    ]
}

fn run_eqsat_and_extract(
    expr: &RecExpr<SkiLang>,
    run_info: &mut protos::SkiPassRunInfo,
) -> Result<SkiLangExpr, Box<dyn Error>> {
    let mut runner = Runner::default().with_expr(expr).run(&make_rules());
    let root = runner.roots[0];

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
                            // Reference to clipRect command in input SKP.
                            let matrixOpCommand = expr.add(SkiLang::DrawCommand(skRecords.index));

                            // Construct surface on which matrixOp should be applied.
                            let newLayerDst = expr.add(SkiLang::Blank);
                            let surfaceToApplyMatrixOp = build_expr(skRecordsIter, newLayerDst, matrixOpCount + 1, expr);

                            let src = expr.add(SkiLang::MatrixOp([surfaceToApplyMatrixOp, matrixOpCommand]));
                            let nextDst = expr.add(SkiLang::SrcOver([dst, src]));

                            if matrixOpCount == 0 {
                                build_expr(skRecordsIter, nextDst, matrixOpCount, expr)
                            } else {
                                nextDst // Need to unwind a bit more
                            }
                            
                        }
                        _ => {
                            let src = expr.add(SkiLang::DrawCommand(skRecords.index));
                            let nextDst = expr.add(SkiLang::SrcOver([dst, src]));
                            build_expr(skRecordsIter, nextDst, matrixOpCount, expr)
                        }
                    }
                },
                Some(Command::Save(save)) => {
                    // Ignore, MatrixOp is used to decide where to put Save commands.
                    build_expr(skRecordsIter, dst, 0, expr)
                },
                Some(Command::SaveLayer(save_layer)) => {
                    let blank = expr.add(SkiLang::Blank);
                    let src = build_expr(skRecordsIter, blank, 0, expr);
                    let nextDst = expr.add(SkiLang::SrcOver([dst, src]));
                    build_expr(skRecordsIter, nextDst, matrixOpCount, expr)
                },
                Some(Command::Restore(restore)) => {
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
enum SkiPassSurfaceType {
    Abstract,
    AbstractWithState,
    Allocated,
}

#[derive(Debug)]
struct SkiPassSurface {
    instructions: Vec<SkiPassInstruction>,
    surface_type: SkiPassSurfaceType
}

fn build_program(expr: &RecExpr<SkiLang>, id: Id) -> SkiPassSurface {
    let node = &expr[id];
    match node {
        SkiLang::DrawCommand(index) => {
            let instruction = SkiPassInstruction {
                // oneof -> Option of a Enum
                instruction: Some(Instruction::CopyRecord(
                    SkiPassCopyRecord {
                        index: *index
                    }
                ))
            };
            SkiPassSurface {
                instructions: vec![instruction],
                surface_type: SkiPassSurfaceType::Abstract
            }
        },
        SkiLang::MatrixOp(ids) => {
            let mut targetSurface = build_program(&expr, ids[0]);
            let mut matrixOpCommand = build_program(&expr, ids[1]);

            let mut instructions: Vec<SkiPassInstruction> = vec![];
            instructions.append(&mut matrixOpCommand.instructions);

            match targetSurface.surface_type {
                SkiPassSurfaceType::Abstract => {
                    instructions.append(&mut targetSurface.instructions);
                },
                SkiPassSurfaceType::AbstractWithState => {
                    instructions.push(SkiPassInstruction {
                        instruction: Some(Instruction::Save(Save{}))
                    });
                    instructions.append(&mut targetSurface.instructions);
                    instructions.push(SkiPassInstruction {
                        instruction: Some(Instruction::Restore(Restore{}))
                    });
                },
                SkiPassSurfaceType::Allocated => {
                    instructions.append(&mut targetSurface.instructions);
                },
            };

            SkiPassSurface {
                instructions,
                surface_type: SkiPassSurfaceType::AbstractWithState
            }
        },
        SkiLang::SrcOver(ids) => {
            let mut dst = build_program(&expr, ids[0]);
            let mut src = build_program(&expr, ids[1]);

            let mut instructions: Vec<SkiPassInstruction> = vec![];

            match dst.surface_type {
                SkiPassSurfaceType::Abstract => {
                    instructions.append(&mut dst.instructions);
                },
                SkiPassSurfaceType::AbstractWithState => {
                    instructions.push(SkiPassInstruction {
                        instruction: Some(Instruction::Save(Save{}))
                    });
                    instructions.append(&mut dst.instructions);
                    instructions.push(SkiPassInstruction {
                        instruction: Some(Instruction::Restore(Restore{}))
                    });
                },
                SkiPassSurfaceType::Allocated => {
                    instructions.append(&mut dst.instructions);
                },
            };

            match src.surface_type {
                SkiPassSurfaceType::Abstract => {
                    instructions.append(&mut src.instructions);
                },
                SkiPassSurfaceType::AbstractWithState => {
                    instructions.push(SkiPassInstruction {
                        instruction: Some(Instruction::Save(Save{}))
                    });
                    instructions.append(&mut src.instructions);
                    instructions.push(SkiPassInstruction {
                        instruction: Some(Instruction::Restore(Restore{}))
                    });
                },
                SkiPassSurfaceType::Allocated => {
                    instructions.push(SkiPassInstruction {
                        instruction: Some(Instruction::SaveLayer(SaveLayer{}))
                    });
                    instructions.append(&mut src.instructions);
                    instructions.push(SkiPassInstruction {
                        instruction: Some(Instruction::Restore(Restore{}))
                    });
                },
            };


            SkiPassSurface {
                instructions,
                surface_type: SkiPassSurfaceType::Allocated
            }
        },
        _ => {
            panic!("Badly constructed Recexpr");
        }
    }
}
