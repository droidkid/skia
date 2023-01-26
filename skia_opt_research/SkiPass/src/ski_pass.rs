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
    ski_pass_instruction::Alpha,
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
            println!("{:?}", skiPassRunResult.program);
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
        "drawCommand" = DrawCommand([Id; 1]), // skRecords index
        "matrixOp" = MatrixOp([Id; 2]), // layer to apply matrixOp, command referring original drawCommand
        // dst, src, original saveLayer 
        // original saveLayer filled if saveLayer has bounds to apply or filters to apply
        // TODO: BOUNDS ARE NOT ENFORCED ON SAVELAYER! HANDLE THEM SEPERATELY.
        "srcOver" = SrcOver([Id; 3]),
        "alpha" = Alpha([Id; 2]), // alphaChannel, layer
    }
}

fn make_rules() -> Vec<Rewrite<SkiLang, ()>> {
    vec![
        rewrite!("remove-blank-dst-savelayers"; "(srcOver ?a blank noOp)" => "?a"),
        rewrite!("remove-blank-src-savelayers"; "(srcOver blank ?a noOp)" => "?a"),
        rewrite!("merge-noOp-saveLayer"; "(srcOver ?b (srcOver blank ?a ?c) noOp)" => "(srcOver ?b ?a ?c)"),
        rewrite!("remove-noOp-alpha"; "(alpha 255 ?a)" => "?a"),
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

    println!("Opt: {}", optimized);

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
                            let matrixOpIndex = expr.add(SkiLang::Num(skRecords.index));
                            let matrixOpCommand = expr.add(SkiLang::DrawCommand([matrixOpIndex]));

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
                            let drawCommandIndex = expr.add(SkiLang::Num(skRecords.index));
                            let src = expr.add(SkiLang::DrawCommand([drawCommandIndex]));
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

                    let has_bounds = save_layer.suggested_bounds.is_some();
                    let has_filters = save_layer.filter_info.is_some();
                    let has_backdrop = save_layer.backdrop.is_some();

                    if has_bounds || has_filters || has_backdrop {
                        let mergeOpIndex = expr.add(SkiLang::Num(skRecords.index));
                        let mergeOp = expr.add(SkiLang::DrawCommand([mergeOpIndex]));
                        let nextDst = expr.add(SkiLang::SrcOver([dst, src, mergeOp]));
                        build_expr(skRecordsIter, nextDst, matrixOpCount, expr)
                    } else {
                        let mergeOp = expr.add(SkiLang::NoOp);
                        let alphaChannel = expr.add(SkiLang::Num(save_layer.alpha_u8));
                        let applyAlphaSrc = expr.add(SkiLang::Alpha([alphaChannel, src]));
                        let nextDst = expr.add(SkiLang::SrcOver([dst, applyAlphaSrc, mergeOp]));
                        build_expr(skRecordsIter, nextDst, matrixOpCount, expr)
                    }
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
        SkiLang::DrawCommand(ids) => {
            let index = match &expr[ids[0]] {
                SkiLang::Num(value) => *value,
                _ => panic!()
            };
            let instruction = SkiPassInstruction {
                // oneof -> Option of a Enum
                instruction: Some(Instruction::CopyRecord(
                    SkiPassCopyRecord {
                        index
                    }
                ))
            };
            SkiPassSurface {
                instructions: vec![instruction],
                surface_type: SkiPassSurfaceType::Abstract,
            }
        },
        SkiLang::Alpha(ids) => {
            let mut targetSurface = build_program(&expr, ids[1]);
            let alpha_u8 = match &expr[ids[0]] {
                SkiLang::Num(value) => *value,
                _ => panic!()
            };

            let mut instructions: Vec<SkiPassInstruction> = vec![];
            instructions.push(SkiPassInstruction {
                instruction: Some(Instruction::ApplyAlpha(Alpha{ alpha_u8 }))
            });
            instructions.append(&mut targetSurface.instructions);
            instructions.push(SkiPassInstruction {
                instruction: Some(Instruction::PopAlpha(Alpha{ alpha_u8 }))
            });

            SkiPassSurface {
                instructions,
                surface_type: targetSurface.surface_type
            }
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
            let mut mOp = build_program(&expr, ids[2]); // mOp for mergeOp

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
