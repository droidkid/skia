use egg::*;
use std::error::Error;


use std::fmt::Write;

use crate::protos;
use crate::protos::{
    SkPaint,
    Bounds,
    SkColor,
    SkRecord, 
    SkiPassInstruction,
    SkiPassProgram, 
    SkiPassRunInfo,
    SkiPassRunResult,
    BlendMode,
    ClipOp,
    sk_paint::Blender,
    sk_paint::ImageFilter,
    sk_paint::ColorFilter,
    sk_paint::PathEffect,
    sk_paint::MaskFilter,
    sk_paint::Shader,
    ski_pass_instruction::SkiPassCopyRecord,
    ski_pass_instruction::Instruction,
    ski_pass_instruction::SaveLayer,
    ski_pass_instruction::Save,
    ski_pass_instruction::Restore,
    ski_pass_instruction::ClipRect, 
};
use crate::ski_lang::SkiLang;
use crate::ski_lang::make_rules;
use crate::ski_lang::SkiLangCostFn;
use crate::build_ski_lang_expr::build_expr;
use crate::build_ski_lang_expr::SkiLangExpr;

pub fn optimize(record: SkRecord) -> SkiPassRunResult {
    let mut expr = RecExpr::default();

    let mut skiPassRunResult = SkiPassRunResult::default();
    let mut skiRunInfo = SkiPassRunInfo::default();

    skiRunInfo.input_record = Some(record.clone());

    let _id = build_expr(&mut record.records.iter(), &mut expr);

    match run_eqsat_and_extract(&expr, &mut skiRunInfo) {
        Ok(optExpr) => {
            let mut program = SkiPassProgram::default();
            program.instructions = build_program(&optExpr.expr, optExpr.id).instructions;
            skiPassRunResult.program = Some(program);
            skiPassRunResult.run_info = Some(skiRunInfo);
        }
        Err(_e) => {}
    }
    skiPassRunResult
}


fn run_eqsat_and_extract(
    expr: &RecExpr<SkiLang>,
    run_info: &mut protos::SkiPassRunInfo,
    ) -> Result<SkiLangExpr, Box<dyn Error>> {
    let runner = Runner::default().with_expr(expr).run(&make_rules());
    let root = runner.roots[0];

    writeln!(&mut run_info.skilang_expr, "{:#}", expr);
    // println!("EXPR: {:#}", expr);

    let extractor = Extractor::new(&runner.egraph, SkiLangCostFn);
    let (_cost, optimized) = extractor.find_best(root);

    writeln!(&mut run_info.extracted_skilang_expr, "{:#}", optimized);
    // println!("OPT: {:#}", optimized);

    // Figure out how to walk a RecExpr without the ID.
    // Until then, use this roundabout way to get the optimized recexpr id.
    let mut egraph = EGraph::<SkiLang, ()>::default();
    let id = egraph.add_expr(&optimized);

    Ok(SkiLangExpr {
        expr: optimized,
        id,
    })
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

fn build_program(expr: &RecExpr<SkiLang>, id: Id) -> SkiPassSurface {
    let node = &expr[id];
    match node {
        SkiLang::Blank => {
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
                SkiLang::Exists(val) => *val,
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
            let index = match &expr[mergeParamIds[0]] {
                SkiLang::Num(index) => *index,
                _ => panic!("Merge Params first parameter not index")
            };
            let paint = paint_expr_to_proto(expr, mergeParamIds[1]);
            let backdrop_exists = match &expr[mergeParamIds[2]] {
                SkiLang::Backdrop(ids) => {
                    match &expr[ids[0]] {
                        SkiLang::Exists(value) => *value,
                        _ => panic!("Backdrop first param not Exists")
                    }
                },
                _ => panic!("Merge params third parameter not backdrop")
            };
            let bounds = bounds_expr_to_proto(expr, mergeParamIds[3]);

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
                instructions.push(SkiPassInstruction {
                    instruction: Some(Instruction::CopyRecord(
                        SkiPassCopyRecord {
                             index,
                             paint: Some(paint),
                             alpha: 255
                         }
                    )),
                });
                instructions.append(&mut src.instructions);
                instructions.push(SkiPassInstruction {
                    instruction: Some(Instruction::Restore(Restore{}))
                });
            } else {
                instructions.push(
                    SkiPassInstruction {
                        instruction: Some(Instruction::SaveLayer(
                                         SaveLayer{
                                             paint: Some(paint),  
                                             bounds,
                                             backdrop: None
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
        SkiLang::Alpha(_ids) => {
            panic!("An Alpha survived extraction! THIS SHOULD NOT HAPPEN");
        },
        _ => {
            panic!("Badly constructed Recexpr {:?} ", node);
        }
    }
}



fn get_exists_value(expr: &RecExpr<SkiLang>, id: Id) -> bool {
    match expr[id] {
        SkiLang::Exists(value) => {
            value
        },
        _ => panic!("Not a SkiLang::Exists")
    }
}

fn get_blend_mode(expr: &RecExpr<SkiLang>, id: Id) -> i32 {
    match expr[id] {
        SkiLang::BlendMode_Src => BlendMode::Src.into() ,
        SkiLang::BlendMode_SrcOver => BlendMode::SrcOver.into(),
        SkiLang::BlendMode_Unknown => BlendMode::Unknown.into(),
        _ => panic!("Not a valid BlendMode")
    }
}

fn bounds_expr_to_proto(expr: &RecExpr<SkiLang>, id: Id) -> Option<Bounds> {
    let bounds: Option<Bounds> = match &expr[id] {
        SkiLang::Bounds(ids) => {
            match &expr[ids[0]] {
                SkiLang::Exists(true) => {
                    Some(unpack_rect_to_bounds(&expr, ids[1]))
                },
                SkiLang::Exists(false) => {
                    None
                },
                _ => panic!("First param of bounds not exist flag")
            }
        },
        _ => panic!("Merge params 4th param is not bounds")
    };
    bounds
}


fn unpack_rect_to_bounds(expr: &RecExpr<SkiLang>, id: Id) -> Bounds {
    match &expr[id] {
        SkiLang::Rect(ids) => {
            let left = unpack_float(expr, ids[0]);
            let top = unpack_float(expr, ids[1]);
            let right = unpack_float(expr, ids[2]);
            let bottom = unpack_float(expr, ids[3]);
            Bounds {
                left,
                top,
                right,
                bottom
            }
        },
        _ => panic!("This is not a rect!")
    }
}

fn paint_expr_to_proto(expr: &RecExpr<SkiLang>, id: Id) -> SkPaint {
    let paint_param_ids = match expr[id] {
        SkiLang::Paint(ids) => ids,
        _ => panic!("Attempting to convert a non paint expr to proto")
    };
    let color = Some(color_expr_to_proto(expr, paint_param_ids[0]));

    let blend_mode = match expr[paint_param_ids[1]] {
        SkiLang::Blender(ids) => get_blend_mode(expr, ids[0]),
        _ => panic!("Second parameter of Paint is not Blender!")
    };

    let image_filter_exists = match expr[paint_param_ids[2]] {
        SkiLang::ImageFilter(ids) => get_exists_value(expr, ids[0]),
        _ => panic!("Third parameter of Paint is not ImageFilter!")
    };

    let color_filter_exists = match expr[paint_param_ids[3]] {
        SkiLang::ColorFilter(ids) => get_exists_value(expr, ids[0]),
        _ => panic!("Fourth parameter of Paint is not ColorFilter!")
    };

    let path_effect_exists = match expr[paint_param_ids[4]] {
        SkiLang::PathEffect(ids) => get_exists_value(expr, ids[0]),
        _ => panic!("Fifth parameter of Paint is not PathEffect!")
    };

    let mask_filter_exists = match expr[paint_param_ids[5]] {
        SkiLang::MaskFilter(ids) => get_exists_value(expr, ids[0]),
        _ => panic!("Sixth parameter of Paint is not MaskFilter!")
    };

    let shader_exists = match expr[paint_param_ids[6]] {
        SkiLang::Shader(ids) => get_exists_value(expr, ids[0]),
        _ => panic!("Seventh parameter of Paint is not Shader!")
    };

    SkPaint {
        color,
        // TODO: Fill these fields.
        // It doesn't really matter now, we bail out and copy the command
        // if any of the below fields are set. Only the color.alpha matters
        // at this point.
        blender: Some(Blender{
            blend_mode
        }),
        image_filter: if image_filter_exists {
            Some(ImageFilter {})
        } else {
            None
        },
        color_filter: if color_filter_exists {
            Some(ColorFilter {})
        } else {
            None
        },
        path_effect: if path_effect_exists {
            Some(PathEffect {})
        } else {
            None
        },
        mask_filter: if mask_filter_exists {
            Some(MaskFilter {})
        } else {
            None
        },
        shader: if shader_exists {
            Some(Shader {})
        } else {
            None
        },
    }
}



fn color_expr_to_proto(expr: &RecExpr<SkiLang>, id: Id) -> SkColor {
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

fn unpack_float(expr: &RecExpr<SkiLang>, id: Id) -> f64 {
    match &expr[id] {
        SkiLang::Float(val) => {
            **val
        },
        _ => panic!("This is not a float!")
    }
}
