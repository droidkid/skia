use egg::*;

use crate::protos::{
    ski_pass_instruction::ClipRect, ski_pass_instruction::Concat44,
    ski_pass_instruction::Instruction, ski_pass_instruction::Restore, ski_pass_instruction::Save,
    ski_pass_instruction::SaveLayer, ski_pass_instruction::SkiPassCopyRecord, Bounds,
    ClipOp, SkM44, SkiPassInstruction,
};
use crate::ski_lang::{SkiLang, SkiLangClipRectMode};

#[derive(Debug)]
pub struct SkiPassSurface {
    instructions: Vec<SkiPassInstruction>,
    modified_state: bool,
}

pub fn expr_to_program(expr: &RecExpr<SkiLang>) -> Vec<SkiPassInstruction> {
    let id = (expr.as_ref().len()-1).into();
    build_program(expr, id).instructions
}

fn build_program(expr: &RecExpr<SkiLang>, id: Id) -> SkiPassSurface {
    let node = &expr[id];
    match node {
        SkiLang::BlankSurface => SkiPassSurface {
            instructions: vec![],
            modified_state: false,
        },
        SkiLang::DrawCommand(_ids) => {
            SkiPassSurface {
                instructions: to_instructions(&expr, id), // NOTE: id, not ids[0] -> we are parsing the command, not its args
                modified_state: false,
            }
        }
        SkiLang::OtherStateOp(ids) => {
            let mut target_surface = build_program(&expr, ids[0]);
            let mut matrix_op_instructions = to_instructions(&expr, ids[1]);

            let mut instructions: Vec<SkiPassInstruction> = vec![];
            instructions.append(&mut matrix_op_instructions);
            instructions.append(&mut target_surface.instructions);

            SkiPassSurface {
                instructions,
                modified_state: true,
            }
        }
        SkiLang::Concat44(ids) => {
            let mut target_surface = build_program(&expr, ids[0]);
            let mut matrix_op_instructions = to_instructions(&expr, ids[1]);

            let mut instructions: Vec<SkiPassInstruction> = vec![];
            instructions.append(&mut matrix_op_instructions);
            instructions.append(&mut target_surface.instructions);

            SkiPassSurface {
                instructions,
                modified_state: true,
            }
        }
        SkiLang::ClipRect(ids) => {
            let mut target_surface = build_program(&expr, ids[0]);
            let clip_rect_params = match &expr[ids[1]] {
                SkiLang::ClipRectParams(value) => value,
                _ => panic!("ClipRect first param is not ClipRect"),
            };

            let bounds: Option<Bounds> = Some(Bounds {
                left: *clip_rect_params.bounds.l,
                right: *clip_rect_params.bounds.r,
                top: *clip_rect_params.bounds.t,
                bottom: *clip_rect_params.bounds.b,
            });
            let clip_op: i32 = match clip_rect_params.clip_rect_mode {
                SkiLangClipRectMode::Intersect => ClipOp::Intersect.into(),
                SkiLangClipRectMode::Diff => ClipOp::Difference.into(),
            };
            let do_anti_alias: bool = clip_rect_params.is_anti_aliased;

            let mut instructions: Vec<SkiPassInstruction> = vec![];
            instructions.push(SkiPassInstruction {
                instruction: Some(Instruction::ClipRect({
                    ClipRect {
                        bounds,
                        clip_op,
                        do_anti_alias,
                    }
                })),
            });
            instructions.append(&mut target_surface.instructions);

            SkiPassSurface {
                instructions,
                modified_state: true,
            }
        }
        // Not to be confused with Concat44 (which is a state matrix multiplication)
        SkiLang::Concat(ids) => {
            let mut p1 = build_program(&expr, ids[0]);
            let mut p2 = build_program(&expr, ids[1]);

            let mut instructions: Vec<SkiPassInstruction> = vec![];

            if p1.modified_state {
                instructions.push(SkiPassInstruction {
                    instruction: Some(Instruction::Save(Save {})),
                });
                instructions.append(&mut p1.instructions);
                instructions.push(SkiPassInstruction {
                    instruction: Some(Instruction::Restore(Restore {})),
                });
            } else {
                instructions.append(&mut p1.instructions);
            }

            if p2.modified_state {
                instructions.push(SkiPassInstruction {
                    instruction: Some(Instruction::Save(Save {})),
                });
                instructions.append(&mut p2.instructions);
                instructions.push(SkiPassInstruction {
                    instruction: Some(Instruction::Restore(Restore {})),
                });
            } else {
                instructions.append(&mut p2.instructions);
            }

            SkiPassSurface {
                instructions,
                modified_state: false,
            }
        }
        SkiLang::Merge(ids) => {
            let mut dst = build_program(&expr, ids[0]);
            let mut src = build_program(&expr, ids[1]);

            let mut instructions: Vec<SkiPassInstruction> = vec![];
            if dst.modified_state {
                instructions.push(SkiPassInstruction {
                    instruction: Some(Instruction::Save(Save {})),
                });
                instructions.append(&mut dst.instructions);
                instructions.push(SkiPassInstruction {
                    instruction: Some(Instruction::Restore(Restore {})),
                });
            } else {
                instructions.append(&mut dst.instructions);
            }

            let (merge_params_id, state_expr_id) = match &expr[ids[2]] {
                SkiLang::MergeParamsWithState(ids) => (ids[0], ids[1]),
                _ => panic!("Merge parameter is not MergeParams"),
            };

            let merge_params = match &expr[merge_params_id] {
                SkiLang::MergeParams(merge_params) => merge_params,
                _ => panic!("First argument of MergeParamsWithState not MergeParams")
            };

            let mut state_construction_instructions = build_program(expr, state_expr_id).instructions;
            let mut src_instructions: Vec<SkiPassInstruction> = vec![];
            let can_reconstruct = !merge_params.has_backdrop && !merge_params.paint.has_filters;

            if can_reconstruct {
                src_instructions.push(SkiPassInstruction {
                    instruction: Some(Instruction::SaveLayer(SaveLayer {
                        paint: Some(merge_params.paint.to_proto()),
                        bounds: if merge_params.has_bounds {
                            Some(merge_params.bounds.to_proto())
                         } else {
                            None
                         },
                        backdrop: None,
                    })),
                });
                src_instructions.append(&mut src.instructions);
                src_instructions.push(SkiPassInstruction {
                    instruction: Some(Instruction::Restore(Restore {})),
                });
            } else {
                src_instructions.push(SkiPassInstruction {
                    instruction: Some(Instruction::CopyRecord(SkiPassCopyRecord {
                        index: merge_params.index,
                        paint: Some(merge_params.paint.to_proto()),
                        alpha: 255,
                    })),
                });
                src_instructions.append(&mut src.instructions);
                src_instructions.push(SkiPassInstruction {
                    instruction: Some(Instruction::Restore(Restore {})),
                });
            };

            if state_construction_instructions.len() > 0 {
                instructions.push(SkiPassInstruction {
                    instruction: Some(Instruction::Save(Save {})),
                });
                instructions.append(&mut state_construction_instructions);
                instructions.append(&mut src_instructions);
                instructions.push(SkiPassInstruction {
                    instruction: Some(Instruction::Restore(Restore {})),
                });
            } else {
                instructions.append(&mut src_instructions);
            }

            SkiPassSurface {
                instructions,
                modified_state: false,
            }
        }
        SkiLang::ApplyAlpha(_ids) => {
            panic!("An Alpha survived extraction! THIS SHOULD NOT HAPPEN");
        }
        SkiLang::ApplyState(_ids) => {
            panic!("An Apply State survived extraction! THIS SHOULD NOT HAPPEN");
        }
        SkiLang::BlankState => SkiPassSurface {
            instructions: vec![],
            modified_state: false,
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
        }
        SkiLang::BlankState => {
            vec![]
        }
        SkiLang::M44(m44) => {
            let m: Vec<f64> = m44.as_vec();
            let instruction = SkiPassInstruction {
                instruction: Some(Instruction::Concat44(Concat44 {
                    matrix: Some(SkM44 { m }),
                })),
            };
            vec![instruction]
        }
        SkiLang::OtherStateOpParams(matrix_op_params) => {
            let instruction = SkiPassInstruction {
                instruction: Some(Instruction::CopyRecord(SkiPassCopyRecord {
                    index: matrix_op_params.index,
                    alpha: 255,
                    paint: None,
                })),
            };
            vec![instruction]
        }
        SkiLang::DrawCommand(draw_command) => {
            let instruction = SkiPassInstruction {
                instruction: Some(Instruction::CopyRecord(SkiPassCopyRecord {
                    index: draw_command.index,
                    paint: Some(draw_command.paint.to_proto()),
                    // TODO: Remove alpha
                    alpha: 255,
                })),
            };
            vec![instruction]
        }
        _ => {
            panic!("Not a instruction, this is a Surface!");
        }
    }
}
