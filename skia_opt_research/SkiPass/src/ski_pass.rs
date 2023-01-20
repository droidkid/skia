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
        AlphaChannel(u8),
        "noOp" = NoOp,
        DrawCommand(i32), // skRecords index
        "matrixOp" = MatrixOp([Id; 2]), // layer to apply matrixOp, command referring original drawCommand
        "blank" = Blank,
        "srcOver" = SrcOver([Id; 4]), // dst, src, bounds (if any or NoOp), filters (if any or NoOp)
        "applyAlpha" = ApplyAlpha([Id; 2]), // alphaChannel, layer
    }
}

fn make_rules() -> Vec<Rewrite<SkiLang, ()>> {
    vec![
        rewrite!("remove-blank-dst-savelayers"; "(srcOver ?a blank noOp)" => "?a"),
        rewrite!("remove-blank-src-savelayers"; "(srcOver blank ?a noOp)" => "?a"),
        rewrite!("merge-noOp-saveLayer"; "(srcOver ?b (srcOver blank ?a ?c) noOp)" => "(srcOver ?b ?a ?c)"),
    ]
}

fn run_eqsat_and_extract(
    expr: &RecExpr<SkiLang>,
    run_info: &mut protos::SkiPassRunInfo,
    ) -> Result<SkiLangExpr, Box<dyn Error>> {
    let mut runner = Runner::default().with_expr(expr).run(&make_rules());
    let root = runner.roots[0];

    // println!("Exp: {}", expr);

    let extractor = Extractor::new(&runner.egraph, AstSize);
    let (cost, mut optimized) = extractor.find_best(root);

    // println!("Opt: {}", optimized);

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
                            let noOp = expr.add(SkiLang::NoOp);
                            let nextDst = expr.add(SkiLang::SrcOver([dst, src, noOp]));

                            if matrixOpCount == 0 {
                                build_expr(skRecordsIter, nextDst, matrixOpCount, expr)
                            } else {
                                nextDst // Need to unwind a bit more
                            }

                        }
                        _ => {
                            let src = expr.add(SkiLang::DrawCommand(skRecords.index));
                            let noOp = expr.add(SkiLang::NoOp);
                            let nextDst = expr.add(SkiLang::SrcOver([dst, src, noOp]));
                            build_expr(skRecordsIter, nextDst, matrixOpCount, expr)
                        }
                    }
                },
                Some(Command::Save(save)) => {
                    // Ignore, MatrixOp is used to decide where to put Save commands.
                    build_expr(skRecordsIter, dst, 0, expr)
                },
                Some(Command::SaveLayer(save_layer)) => {
                    println!("{:?}", save_layer);
                    let blank = expr.add(SkiLang::Blank);
                    let src = build_expr(skRecordsIter, blank, 0, expr);

                    let bounds = match (save_layer.suggested_bounds) {
                        Some(suggested_bounds) => expr.add(SkiLang::DrawCommand(skRecords.index)),
                        None => expr.add(SkiLang::NoOp),
                    }

                    let filters = match (save_layer.suggested_bounds) {
                        Some(suggested_bounds) => expr.add(SkiLang::DrawCommand(skRecords.index)),
                        None => expr.add(SkiLang::NoOp),
                    }

                    let alphaChannel = expr.add(SkiLang::AlphaChannel(save_layer.alpha_u8));
                    let applyAlphaSrc = expr.add(SkiLang::ApplyAlpha(alphaChannel, src));

                    let nextDst = expr.add(SkiLang::SrcOver([dst, applyAlphaSrc, bounds, filters]));
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
    surface_type: SkiPassSurfaceType,
}

fn build_program(expr: &RecExpr<SkiLang>, id: Id) -> SkiPassSurface {
    let node = &expr[id];
    match node {
        SkiLang::Blank => {
            SkiPassSurface {
                instructions: vec![],
                surface_type: SkiPassSurfaceType::Abstract,

            }
        },
        SkiLang::NoOp => {
            SkiPassSurface {
                instructions: vec![],
                surface_type: SkiPassSurfaceType::Abstract,
            }
        },
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
                surface_type: SkiPassSurfaceType::Abstract,
            }
        },
        SkiLang::ApplyAlpha(ids) => {
            // TODO: Add a ApplyAlpha command
        },
        SkiLang::MatrixOp(ids) => {
            let mut targetSurface = build_program(&expr, ids[0]);
            let mut matrixOpCommand = build_program(&expr, ids[1]);

            let mut instructions: Vec<SkiPassInstruction> = vec![];
            let mut surface_type: SkiPassSurfaceType = SkiPassSurfaceType::AbstractWithState;

            instructions.append(&mut matrixOpCommand.instructions);
            instructions.append(&mut targetSurface.instructions);

            SkiPassSurface {
                instructions,
                surface_type,
            }
        },
        SkiLang::SrcOver(ids) => {
            let mut dst = build_program(&expr, ids[0]);
            let mut src = build_program(&expr, ids[1]);

            // TODO: Check if there are bounds
            match expr[ids[2]] {
                SkiLang::NoOp => {}
                SkiLang::DrawCommand(index) => {}
            }

            // TODO: Check if there are filters
            match expr[ids[3]] {
                SkiLang::NoOp => {}
                SkiLang::DrawCommand(index) => {}
            }

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

            let len = mOp.instructions.len();
            // This is a saveLayer with a paint parameter. So wrap the src in a saveLayer.
            if (len > 0) {
                instructions.append(&mut mOp.instructions);
                instructions.append(&mut src.instructions);
                for _ in 0..len {
                    instructions.push(SkiPassInstruction {
                        instruction: Some(Instruction::Restore(Restore{}))
                    });
                }
            } 
            else {
                match src.surface_type {
                    // It's just a sequence of draw calls, so let's just append.
                    SkiPassSurfaceType::Abstract | SkiPassSurfaceType::Allocated => {
                        instructions.append(&mut src.instructions);
                    },
                    // Else if there's a matrix state, wrap it in a save, restore pair
                    _ => {
                        instructions.push(SkiPassInstruction {
                            instruction: Some(Instruction::Save(Save{}))
                        });
                        instructions.append(&mut src.instructions);
                        instructions.push(SkiPassInstruction {
                            instruction: Some(Instruction::Restore(Restore{}))
                        });
                    }
                };
            } 

            SkiPassSurface {
                instructions,
                surface_type: SkiPassSurfaceType::Allocated,
            }
        },
        _ => {
            panic!("Badly constructed Recexpr");
        }
    }
}
