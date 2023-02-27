use egg::*;
use std::error::Error;
use std::fmt;
use prost::Message;
use std::fmt::Write;

use crate::protos;
use crate::protos::{
    SkPaint,
    Bounds,
    SkColor,
    SkRecord, 
    SkRecords, 
    SkiPassInstruction,
    SkiPassProgram, 
    SkiPassRunInfo,
    SkiPassRunResult,
    BlendMode,
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
    sk_records::Command, 
};

pub fn optimize(record: SkRecord) -> SkiPassRunResult {
    let mut expr = RecExpr::default();

    let mut skiPassRunResult = SkiPassRunResult::default();
    let mut skiRunInfo = SkiPassRunInfo::default();

    skiRunInfo.input_record = Some(record.clone());

    let id = build_expr(&mut record.records.iter(), &mut expr);

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
        // NOTE: The order of Num and Float matters!
        // First all Nums are parsed, and then Floats. So if
        // you want to force a number to be parsed as a float,
        // make sure to add a . (1.0 instead of 1)
        Num(i32),
        Float(ordered_float::NotNan<f64>),
        Exists(bool),
        "noOp" = NoOp,
        "blank" = Blank,
        // ------ BLEND_MODE SYMBOLS BEGIN --------//
        "blendMode_srcOver" = BlendMode_SrcOver,
        "blendMode_src" = BlendMode_Src,
        "blendMode_unknown" = BlendMode_Unknown,
        // -------BLEND MODES SYMBOLS END --------//
        // drawCommand(index, paint)
        "drawCommand" = DrawCommand([Id; 2]),
        // TODO: Split matrix and clip ops. Right now clips are a 'matrixOp'
        // matrixOp(layer, matrixOpParams) -> return layer after applying transform on layer 
        "matrixOp" = MatrixOp([Id; 2]),
        // concat(layer1, layer2) -> return layer resulting from sequential execution of
        // instructions(layer1), instructions(layer2)
        "concat" = Concat([Id; 2]),
        // filter(exists)
        "backdrop" = Backdrop([Id; 1]),
        // ------ PAINT_PARAMS BEGIN --------//
        "color" = Color([Id; 4]),
        "blender" = Blender([Id; 1]),
        "imageFilter" = ImageFilter([Id; 1]),
        "colorFilter" = ColorFilter([Id; 1]),
        "pathEffect" = PathEffect([Id; 1]),
        "maskFilter" = MaskFilter([Id; 1]),
        "shader" = Shader([Id; 1]),
        // ------ PAINT_PARAMS END --------//
		// paint(color, 
        //      filter, 
        //      blender,
        //      imageFilter,
        //      colorFilter,
        //      pathEffect,
        //      maskFilter,
        //      shader
        //  )
		"paint" = Paint([Id; 7]),
        // rect ( l, t, r, b )
        "rect" = Rect([Id; 4]),
        // bound (exists? rect)
        "bounds" = Bounds([Id; 2]),
        // merge(layer1, layer2, mergeParams())
        // This translates directly to saveLayer command in Skia.
        "merge" = Merge([Id; 3]),
        // EGRAPH INTERNAL COMMANDS FOLLOW
        // Below commands have no literal equivalent in Skia, and are only used for EGraph
        // Extraction
        // alpha(layer, value) -> apply transparency of value on layer
        "alpha" = Alpha([Id; 2]), // alphaChannel, layer
        // MergeParams([index, paint, backdrop, bounds])
        "mergeParams" = MergeParams([Id; 4]),
        // MatrixOpParams([index])  - eventually add other matrix stuff.
        "matrixOpParams" = MatrixOpParams([Id; 1]),
    }
}

// TODO: Decide on a convention in writing rules 
// Both of the below are equivalent:
//    ?pathEffect
//    (pathEffect ?pathEffectExists)
// Right now the choice is arbitrary, there's also a choice of using Null.
fn make_rules() -> Vec<Rewrite<SkiLang, ()>> {
    vec![
        rewrite!("remove-noOp-concat-1"; "(concat blank ?a)" => "?a"),
        rewrite!("remove-noOp-concat-2"; "(concat ?a blank)" => "?a"),
        // Kill if only a single drawCommand, and saveLayer is noOp.
        // SaveLayer alpha might have been merged into single drawCommand.
        // This rule corresponds to the killSaveLayer at line 216 in src/core/SkRecordOpts.cpp
        rewrite!("kill-merge-drawIsSrcOver"; 
                 "(merge 
                        ?dst 
                        (drawCommand 
                            ?x 
                            (paint
                                ?drawPaintColor
                                (blender blendMode_srcOver)
                                (imageFilter false)
                                (colorFilter false)
                                ?pathEffect
                                ?maskFilter
                                ?shader
                            )
                        ) 
                        (mergeParams 
                            ?mergeIndex
                            (paint 
                                (color 255 0 0 0)
                                (blender blendMode_srcOver)
                                (imageFilter false)
                                (colorFilter false)
                                (pathEffect false)
                                (maskFilter false)
                                (shader false)
                            )
                            (backdrop false)
                            (bounds false ?boundRect)
                        )
                    )" 
                 => 
                 "(concat 
                        ?dst 
                        (drawCommand 
                            ?x 
                            (paint
                                ?drawPaintColor
                                (blender blendMode_srcOver)
                                (imageFilter false)
                                (colorFilter false)
                                ?pathEffect
                                ?maskFilter
                                ?shader
                            )
                        ) 
                  )"),

        // We can still merge if blendMode is src and alpha is 255.
        // This handles drawPaint being blendMode_src branch at
        // This rule corresponds to the killSaveLayer at line 203 in src/core/SkRecordOpts.cpp
        // (with the srcOver case is handled in the above rule)
        rewrite!("kill-merge-drawIsSrc"; 
                 "(merge 
                        ?dst 
                        (drawCommand 
                            ?x 
                            (paint
                                (color 255 ?r ?g ?b)
                                (blender blendMode_src)
                                (imageFilter false)
                                (colorFilter false)
                                (pathEffect false)
                                (maskFilter false)
                                (shader false)
                            )
                        ) 
                        (mergeParams 
                            ?mergeIndex
                            (paint 
                                (color 255 0 0 0)
                                (blender blendMode_srcOver)
                                (imageFilter false)
                                (colorFilter false)
                                (pathEffect false)
                                (maskFilter false)
                                (shader false)
                            )
                            (backdrop false)
                            (bounds false ?boundRect)
                        )
                    )" 
                 => 
                 "(concat 
                        ?dst 
                        (drawCommand 
                            ?x 
                            (paint
                                (color 255 ?r ?g ?b)
                                (blender blendMode_src)
                                (imageFilter false)
                                (colorFilter false)
                                (pathEffect false)
                                (maskFilter false)
                                (shader false)
                            )
                        )
                  )"),


        rewrite!("push-merge-alpha-on-src"; 
                 "(merge 
                        ?dst 
                        ?src 
                        (mergeParams
                            ?mergeIndex
                            (paint 
                                (color ?a ?r ?g ?b) 
                                (blender blendMode_srcOver)
                                (imageFilter false)
                                (colorFilter false)
                                (pathEffect false)
                                (maskFilter false)
                                (shader false)
                            )
                            (backdrop false)
                            (bounds false ?boundRect)
                        )
                    )" 
                 => 
                 "(merge 
                        ?dst 
                        (alpha ?a ?src) 
                        (mergeParams
                            ?mergeIndex
                            (paint 
                                (color 255 ?r ?g ?b) 
                                (blender blendMode_srcOver)
                                (imageFilter false)
                                (colorFilter false)
                                (pathEffect false)
                                (maskFilter false)
                                (shader false)
                            )
                            (backdrop false)
                            (bounds false ?boundRect)
                        )
                    )"),
        // TODO: MULTIPLY ALPHAS!!!
        rewrite!("lift-alpha"; 
                 "(merge 
                        ?dst 
                        (alpha ?A ?src) 
                        (mergeParams
                            ?mergeIndex
                            (paint 
                                (color 255 ?r ?g ?b) 
                                (blender blendMode_srcOver)
                                (imageFilter false)
                                (colorFilter false)
                                (pathEffect false)
                                (maskFilter false)
                                (shader false)
                            )
                            (backdrop false)
                            (bounds false ?boundRect)
                        )
                    )" 
                 => 
                 "(merge 
                        ?dst 
                        ?src 
                        (mergeParams
                            ?mergeIndex
                            (paint 
                                (color ?A ?r ?g ?b) 
                                (blender blendMode_srcOver)
                                (imageFilter false)
                                (colorFilter false)
                                (pathEffect false)
                                (maskFilter false)
                                (shader false)
                            )
                            (backdrop false)
                            (bounds false ?boundRect)
                        )
                    )"),
        rewrite!("remove-merge-blank"; 
                 "(merge 
                        ?layer 
                        blank 
                        (mergeParams
                            ?mergeIndex
                            (paint 
                                (color ?A ?r ?g ?b) 
                                (blender blendMode_srcOver)
                                (imageFilter false)
                                (colorFilter false)
                                (pathEffect false)
                                (maskFilter false)
                                (shader false)
                            )
                            (backdrop false)
                            (bounds false ?boundRect)
                        )
                   )" 
                => "?layer"),
        rewrite!("remove-noOp-alpha"; "(alpha 255 ?src)" => "?src"),
        // TODO: MULTIPLY ALPHAS!!!
        rewrite!("apply-alpha-on-draw"; 
                        "(alpha ?a 
                                (drawCommand 
                                    ?x 
                                    (paint
                                        (color 255 ?r ?g ?b) 
                                        (blender blendMode_srcOver)
                                        (imageFilter false)
                                        (colorFilter false)
                                        (pathEffect false)
                                        (maskFilter false)
                                        (shader false)
                                    )
                                )
                            )
                        " => "(drawCommand 
                                    ?x 
                                    (paint
                                        (color ?a ?r ?g ?b)
                                        (blender blendMode_srcOver)
                                        (imageFilter false)
                                        (colorFilter false)
                                        (pathEffect false)
                                        (maskFilter false)
                                        (shader false)
                                    )
                                )"),
        rewrite!("remove-blank-matrixOp"; "(matrixOp blank ?a)" => "blank"),
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
    // println!("EXPR: {:#}", expr);

    let extractor = Extractor::new(&runner.egraph, SkiLangCostFn);
    let (cost, mut optimized) = extractor.find_best(root);

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

struct SkiLangExpr {
    pub expr: RecExpr<SkiLang>,
    pub id: Id,
}

#[derive(Clone)]
enum StackOp {
    Surface,
    MatrixOp,
    Save,
	SaveLayer,
}

fn reduceStack(
    expr: &mut RecExpr<SkiLang>,
    drawStack : &mut Vec<(StackOp, Id)>, 
    from_restore: bool
) {

    drawStack.push((StackOp::Surface, expr.add(SkiLang::Blank)));
    while drawStack.len() != 1 {
        let (e1_type, e1) = drawStack.pop().unwrap();
        let (e2_type, e2) = drawStack.pop().unwrap();
        match e2_type {
			StackOp::SaveLayer => {
				let tmp_src = e1;
				let merge_params = e2;

                if !from_restore {
                    // We're not done with this saveLayer, this saveLayer is a barrier
                    // for some other saveLayer. So push them back to the stack and exit.
                    drawStack.push((e2_type, e2));
                    drawStack.push((e1_type, e1));
                    return;
                }

                // Copy the state that needs to applied to this surface.
                let mut stateStack: Vec<(StackOp, Id)> = vec![];
                for op in drawStack.iter() {
                	match op.0 {
                    	StackOp::MatrixOp => stateStack.push(op.clone()),
                        StackOp::Save => stateStack.push(op.clone()),
                        StackOp::SaveLayer => stateStack.clear(),
                        _ => {}
                    }
                }

				reduceStack(expr, drawStack, false);
                let dst = drawStack.pop().unwrap().1;

                let mut apply_state_stack = stateStack.clone();
                apply_state_stack.push((StackOp::Surface, tmp_src));
                reduceStack(expr, &mut apply_state_stack, false);
                let src = apply_state_stack.pop().unwrap().1;

                let bounds = match expr[merge_params] {
                    SkiLang::MergeParams(ids) => bounds_expr_to_proto(expr, ids[3]),
                    _ => panic!("SaveLayer stack does not have mergeParams")
                };

                match bounds {
                    Some(bounds) => {
                        // TODO: Add a clipRect here.
                        // Remove the bounds from merge_params
                        let corrected_merge_params = match expr[merge_params] {
                            SkiLang::MergeParams(ids) => {
                                let exists = expr.add(SkiLang::Exists(false));
                                let boundRect = expr.add(SkiLang::NoOp);
                                let bounds = expr.add(SkiLang::Bounds([exists, boundRect]));
                                expr.add(SkiLang::MergeParams([ids[0], ids[1], ids[2], bounds]))
                            },
                            _ => panic!("SaveLayer stack does not have mergeParams")

                        };
		   	            let merged = expr.add(SkiLang::Merge([dst, src, corrected_merge_params]));
		   		        drawStack.push((StackOp::Surface, merged));
                    },
                    None => {
		   	            let merged = expr.add(SkiLang::Merge([dst, src, merge_params]));
		   		        drawStack.push((StackOp::Surface, merged));
                    },
                }

		   		drawStack.append(&mut stateStack);

			},
            StackOp::Save=> {
                drawStack.push((e1_type, e1));
                if from_restore {
                    break;
                }
            }
            StackOp::MatrixOp => {
                let nxt = expr.add(SkiLang::MatrixOp([e1, e2]));
                drawStack.push((StackOp::Surface, nxt));
            }
            StackOp::Surface => {
                let nxt = expr.add(SkiLang::Concat([e2, e1]));
                drawStack.push((StackOp::Surface, nxt));
            }
        };
    }
}

fn build_expr<'a, I>(
    skRecordsIter: &mut I, 
    expr: &mut RecExpr<SkiLang>,
) -> Id
where
I: Iterator<Item = &'a SkRecords> + 'a,
{
    let mut drawStack: Vec<(StackOp, Id)> = vec![];
    loop {
    match skRecordsIter.next() {
       Some(skRecords) => {
           match &skRecords.command {
               Some(Command::DrawCommand(draw_command)) => {
                   match draw_command.name.as_str() {
                       "ClipPath" | "ClipRRect" | "ClipRect" | "Concat44" => {
                           let matrixOpIndex = expr.add(SkiLang::Num(skRecords.index));
                           let matrixOpParams = expr.add(SkiLang::MatrixOpParams([matrixOpIndex]));
                            drawStack.push((StackOp::MatrixOp, matrixOpParams));
                       },
                   _ => {
                           let drawCommandIndex = expr.add(SkiLang::Num(skRecords.index));
                           let drawCommandPaint = paint_proto_to_expr(expr, &draw_command.paint);
                           let drawOpCommand = expr.add(SkiLang::DrawCommand([drawCommandIndex, drawCommandPaint]));
                           drawStack.push((StackOp::Surface, drawOpCommand));
                       }
                   }
               },
               Some(Command::Save(save)) => {
                    drawStack.push((StackOp::Save, expr.add(SkiLang::NoOp)));
               },
               Some(Command::SaveLayer(save_layer)) => {
                   let index = expr.add(SkiLang::Num(skRecords.index));

                   let paint = paint_proto_to_expr(expr, &save_layer.paint);
                   let bounds = bounds_proto_to_expr(expr, &save_layer.bounds);

                   let backdrop_exists = expr.add(SkiLang::Exists(save_layer.backdrop.is_some()));
                   let backdrop = expr.add(SkiLang::Backdrop([backdrop_exists]));

                   let mergeParams = expr.add(SkiLang::MergeParams([index, paint, backdrop, bounds]));
		   		   drawStack.push((StackOp::SaveLayer, mergeParams));
               },
               Some(Command::Restore(restore)) => {
                   reduceStack(expr, &mut drawStack, true);
               },
               None => {}
           }
       },
       None => break,
    }
    }
    reduceStack(expr, &mut drawStack, false);
    drawStack[0].1
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
        SkiLang::Alpha(ids) => {
            panic!("An Alpha survived extraction! THIS SHOULD NOT HAPPEN");
        },
        _ => {
            panic!("Badly constructed Recexpr");
        }
    }
}


// TODO: Change this to color_proto_to_expr
fn color_expr(expr: &mut RecExpr<SkiLang>, aVal:i32, rVal:i32, gVal:i32, bVal:i32) -> Id {
    let a = expr.add(SkiLang::Num(aVal));
    let r = expr.add(SkiLang::Num(rVal));
    let g = expr.add(SkiLang::Num(gVal));
    let b = expr.add(SkiLang::Num(bVal));
    expr.add(SkiLang::Color([a, r, g, b]))
}

fn bounds_proto_to_expr(expr: &mut RecExpr<SkiLang>, bounds: &Option<Bounds>) -> Id {
    match bounds {
        Some(bounds) => {
            let boundsExist = expr.add(SkiLang::Exists(true));

            let left = ordered_float::NotNan::new(bounds.left).unwrap();
            let top = ordered_float::NotNan::new(bounds.top).unwrap();
            let right = ordered_float::NotNan::new(bounds.right).unwrap();
            let bottom = ordered_float::NotNan::new(bounds.bottom).unwrap();

            let leftExpr = expr.add(SkiLang::Float(left));
            let topExpr = expr.add(SkiLang::Float(top));
            let rightExpr = expr.add(SkiLang::Float(right));
            let bottomExpr = expr.add(SkiLang::Float(bottom));

            let rect = expr.add(SkiLang::Rect([leftExpr, topExpr, rightExpr, bottomExpr]));
            expr.add(SkiLang::Bounds([boundsExist, rect]))
        },
        None => {
            let boundsExist = expr.add(SkiLang::Exists(false));
            let noOp = expr.add(SkiLang::NoOp);
            expr.add(SkiLang::Bounds([boundsExist, noOp]))
        }
    }
}

fn paint_proto_to_expr(expr: &mut RecExpr<SkiLang>, skPaint: &Option<SkPaint>) -> Id {
    let color = match &skPaint {
       	Some(skPaint) => {
       	    match &skPaint.color {
       	        Some(skColor) => {
       	            color_expr(expr, 
       	                skColor.alpha_u8,
       	                skColor.red_u8,
       	                skColor.green_u8,
       	                skColor.blue_u8)
       	            }
       	        None => {
                    // TODO: Assert that this only happens in SaveLayer.
                    color_expr(expr, 255, 0, 0, 0)
                }
       	    }
       	},
       	None => {
            // TODO: Assert that this only happens in SaveLayer.
            color_expr(expr, 255, 0, 0, 0)
        }
    };


    let blender = match &skPaint {
        Some(skPaint) => {
            match &skPaint.blender {
                Some(blender) => {
                    if blender.blend_mode == BlendMode::SrcOver.into() {
                        let blendMode = expr.add(SkiLang::BlendMode_SrcOver);
                        expr.add(SkiLang::Blender([blendMode]))
                    } 
                    else if blender.blend_mode == BlendMode::Src.into() {
                        let blendMode = expr.add(SkiLang::BlendMode_Src);
                        expr.add(SkiLang::Blender([blendMode]))
                    }
                    else {
                        let blendMode = expr.add(SkiLang::BlendMode_Unknown);
                        expr.add(SkiLang::Blender([blendMode]))
                    }
                },
                None => {
                    let blendMode = expr.add(SkiLang::BlendMode_SrcOver);
                    expr.add(SkiLang::Blender([blendMode]))
                }
            }
        },
        None => {
            let blendMode = expr.add(SkiLang::BlendMode_SrcOver);
            expr.add(SkiLang::Blender([blendMode]))
        }
    };

    let image_filter = match &skPaint {
       	Some(skPaint) => {
            let exists = expr.add(SkiLang::Exists(skPaint.image_filter.is_some()));
            expr.add(SkiLang::ImageFilter([exists]))
       	},
       	None => {
            let exists = expr.add(SkiLang::Exists(false));
            expr.add(SkiLang::ImageFilter([exists]))
        }
    };

    let color_filter = match &skPaint {
       	Some(skPaint) => {
            let exists = expr.add(SkiLang::Exists(skPaint.color_filter.is_some()));
            expr.add(SkiLang::ColorFilter([exists]))
       	},
       	None => {
            let exists = expr.add(SkiLang::Exists(false));
            expr.add(SkiLang::ColorFilter([exists]))
        }
    };

    let path_effect = match &skPaint {
       	Some(skPaint) => {
            let exists = expr.add(SkiLang::Exists(skPaint.path_effect.is_some()));
            expr.add(SkiLang::PathEffect([exists]))
       	},
       	None => {
            let exists = expr.add(SkiLang::Exists(false));
            expr.add(SkiLang::PathEffect([exists]))
        }
    };

    let mask_filter = match &skPaint {
       	Some(skPaint) => {
            let exists = expr.add(SkiLang::Exists(skPaint.mask_filter.is_some()));
            expr.add(SkiLang::MaskFilter([exists]))
       	},
       	None => {
            let exists = expr.add(SkiLang::Exists(false));
            expr.add(SkiLang::MaskFilter([exists]))
        }
    };

    let shader = match &skPaint {
       	Some(skPaint) => {
            let exists = expr.add(SkiLang::Exists(skPaint.shader.is_some()));
            expr.add(SkiLang::Shader([exists]))
       	},
       	None => {
            let exists = expr.add(SkiLang::Exists(false));
            expr.add(SkiLang::Shader([exists]))
        }
    };

    expr.add(SkiLang::Paint([
            color, 
            blender,
            image_filter,
            color_filter,
            path_effect,
            mask_filter,
            shader
        ]))
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

fn unpack_float(expr: &RecExpr<SkiLang>, id: Id) -> f64 {
    match &expr[id] {
        SkiLang::Float(val) => {
            **val
        },
        _ => panic!("This is not a float!")
    }
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

