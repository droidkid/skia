use egg::*;



use crate::ski_lang_converters::*;


use crate::protos::{
    Bounds, 
    SkiPassInstruction,
    BlendMode,
    ClipOp,
    SkM44,
    ski_pass_instruction::SkiPassCopyRecord,
    ski_pass_instruction::Concat44,
    ski_pass_instruction::Instruction,
    ski_pass_instruction::SaveLayer,
    ski_pass_instruction::Save,
    ski_pass_instruction::Restore,
    ski_pass_instruction::ClipRect, 
};
use crate::ski_lang::SkiLang;

#[derive(Debug)]
pub struct SkiPassSurface {
    instructions: Vec<SkiPassInstruction>,
    modified_matrix: bool,
}

pub fn expr_to_program(expr: &RecExpr<SkiLang>, id: Id) -> Vec<SkiPassInstruction> {
    build_program(expr, id).instructions
}

fn build_program(expr: &RecExpr<SkiLang>, id: Id) -> SkiPassSurface {
    let node = &expr[id];
    match node {
        SkiLang::BlankSurface => {
            SkiPassSurface {
                instructions: vec![],
                modified_matrix: false,
            }
        },
        SkiLang::DrawCommand(_ids) => {
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
        SkiLang::Concat44(ids) => {
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
        SkiLang::ClipRect(ids) => {
            let mut targetSurface = build_program(&expr, ids[0]);
            let clipRectParams = match &expr[ids[1]] {
                SkiLang::ClipRectParams(ids) => ids,
                _ => panic!("ClipRect first param is not ClipRect")
            };

            let bounds: Option<Bounds> = Some(unpack_rect_to_bounds(&expr, clipRectParams[0]));
            let clip_op : i32 = match &expr[clipRectParams[1]] {
                SkiLang::ClipOp_Intersect => ClipOp::Intersect.into(),
                SkiLang::ClipOp_Diff => ClipOp::Difference.into(),
                _ => panic!("ClipOp is invalid")
            };

            let do_anti_alias : bool = match &expr[clipRectParams[2]] {
                SkiLang::Bool(val) => *val,
                _ => panic!("ClipOp third parameter is not bool (doAntiAlias)"),
            };

            let mut instructions: Vec<SkiPassInstruction> = vec![];
            instructions.push(SkiPassInstruction {
                instruction: Some(Instruction::ClipRect({
                    ClipRect {
                        bounds,
                        clip_op,
                        do_anti_alias
                    }
                }))
            });
            instructions.append(&mut targetSurface.instructions);

            SkiPassSurface {
                instructions,
                modified_matrix: true,
            }
        },
        // Not to be confused with Concat44 (which is a state matrix multiplication)
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

            let mergeParamIds = match &expr[ids[2]] {
                SkiLang::MergeParams(ids) => ids,
                _ => panic!("Merge parameter is not MergeParams")
            };

            let mut reconstructStateInstructions = build_program(expr, mergeParamIds[4]).instructions;
            reconstructStateInstructions.reverse();

            let index = match &expr[mergeParamIds[0]] {
                SkiLang::Num(index) => *index,
                _ => panic!("Merge Params first parameter not index")
            };
            let paint = paint_expr_to_proto(expr, mergeParamIds[1]);
            let backdrop_exists = match &expr[mergeParamIds[2]] {
                SkiLang::Backdrop(ids) => {
                    match &expr[ids[0]] {
                        SkiLang::Bool(value) => *value,
                        _ => panic!("Backdrop first param not Bool")
                    }
                },
                _ => panic!("Merge params third parameter not backdrop")
            };
            let bounds = bounds_expr_to_proto(expr, mergeParamIds[3]);

            let mut src_instructions: Vec<SkiPassInstruction> = vec![];
            let can_reconstruct = !backdrop_exists 
                                && paint.image_filter.is_none()
                                && paint.color_filter.is_none()
                                && paint.path_effect.is_none()
                                && paint.mask_filter.is_none()
                                && paint.shader.is_none()
                                && (paint.blender.is_none() ||
                                    paint.blender.as_ref().unwrap().blend_mode == 
                                    BlendMode::SrcOver.into());

            if !can_reconstruct {
                src_instructions.push(SkiPassInstruction {
                    instruction: Some(Instruction::CopyRecord(
                        SkiPassCopyRecord {
                             index,
                             paint: Some(paint),
                             alpha: 255
                         }
                    )),
                });
                src_instructions.append(&mut src.instructions);
                src_instructions.push(SkiPassInstruction {
                    instruction: Some(Instruction::Restore(Restore{}))
                });
            } else {
                src_instructions.push(
                    SkiPassInstruction {
                        instruction: Some(Instruction::SaveLayer(
                                         SaveLayer{
                                             paint: Some(paint),  
                                             bounds,
                                             backdrop: None
                                         }))
                });
                src_instructions.append(&mut src.instructions);
                src_instructions.push(SkiPassInstruction {
                    instruction: Some(Instruction::Restore(Restore{}))
                });
            };

            if reconstructStateInstructions.len() > 0 {
                instructions.push(SkiPassInstruction {
                    instruction: Some(Instruction::Save(Save{}))
                });
                instructions.append(&mut reconstructStateInstructions);
                instructions.append(&mut src_instructions);
                instructions.push(SkiPassInstruction {
                    instruction: Some(Instruction::Restore(Restore{}))
                });
            } else {
                instructions.append(&mut src_instructions);
            }
            
            SkiPassSurface {
                instructions,
                modified_matrix: false
            }

        },
        SkiLang::Alpha(_ids) => {
            panic!("An Alpha survived extraction! THIS SHOULD NOT HAPPEN");
        },
        SkiLang::BlankState => {
            SkiPassSurface {
                instructions: vec![],
                modified_matrix: false
            }
        },
        _ => {
            panic!("Badly constructed Recexpr {:?} ", node);
        }
    }
}

fn to_instructions(expr: &RecExpr<SkiLang>, id: Id) -> Vec<SkiPassInstruction> {
    let node = &expr[id];
    match node {
        SkiLang::NoOp => {
            vec![]
        },
        SkiLang::BlankState => {
            vec![]
        },
        SkiLang::M44(ids) => {
            let mut m: Vec<f64> = vec![];
            for id in ids {
                m.push(unpack_float(expr, *id));
            }
            let instruction = SkiPassInstruction {
                instruction : Some(Instruction::Concat44(
                    Concat44 {
                        matrix: Some(SkM44{
                            m
                        })
                    }
                ))
            };
            vec![instruction]
        },
        SkiLang::MatrixOpParams(ids) => {
            let instruction = match &expr[ids[0]] {
                SkiLang::Num(index) => SkiPassInstruction {
                        instruction: Some(Instruction::CopyRecord(
                        SkiPassCopyRecord {
                            index: *index,
                            alpha: 255,
                            paint: None
                    }))
                },
                _ => panic!("MatrixParams not constructed correctly")
            };
            vec![instruction]
        },
        SkiLang::Num(index) => {
            let instruction = SkiPassInstruction {
                instruction: Some(Instruction::CopyRecord(
                    SkiPassCopyRecord {
                        index: *index,
                        alpha: 255,
                        paint: None
                }
            ))};
            vec![instruction]
        },
        SkiLang::DrawCommand(ids) => {
            let index = match &expr[ids[0]] {
                SkiLang::Num(value) => *value,
                _ => panic!()
            };
            let paint = paint_expr_to_proto(expr, ids[1]);
            let instruction = SkiPassInstruction {
                // oneof -> Option of a Enum
                instruction: Some(Instruction::CopyRecord(
                    SkiPassCopyRecord {
                        index,
                        alpha: 255,
                        paint: Some(paint),
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

