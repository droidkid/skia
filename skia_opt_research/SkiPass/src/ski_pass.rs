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
    sk_paint::Effects,
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

    writeln!(&mut skiRunInfo.sk_records_proto, "{:?}", record);

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
        Num(i32),
        Exists(bool),
        "noOp" = NoOp,
        "blank" = Blank,
        "color" = Color([Id; 4]),
        // drawCommand(index, paint)
        "drawCommand" = DrawCommand([Id; 2]),
        // TODO: Split matrix and clip ops. Right now clips are a 'matrixOp'
        // matrixOp(layer, matrixOpParams) -> return layer after applying transform on layer 
        "matrixOp" = MatrixOp([Id; 2]),
        // concat(layer1, layer2) -> return layer resulting from sequential execution of
        // instructions(layer1), instructions(layer2)
        "concat" = Concat([Id; 2]),
        // filter(exists)
        "effects" = Effects([Id; 1]),
		// paint(color, filter)
		"paint" = Paint([Id; 2]),
        // merge(layer1, layer2, mergeParams())
        // This translates directly to saveLayer command in Skia.
        "merge" = Merge([Id; 3]),
        // EGRAPH INTERNAL COMMANDS FOLLOW
        // Below commands have no literal equivalent in Skia, and are only used for EGraph
        // Extraction
        // alpha(layer, value) -> apply transparency of value on layer
        "alpha" = Alpha([Id; 2]), // alphaChannel, layer
        // MergeParams([index, paint]) - eventually add bounds, backdrop.
        "mergeParams" = MergeParams([Id; 2]),
        // MatrixOpParams([index])  - eventually add other matrix stuff.
        "matrixOpParams" = MatrixOpParams([Id; 1]),
    }
}

fn make_rules() -> Vec<Rewrite<SkiLang, ()>> {
    vec![
        rewrite!("remove-noOp-concat-1"; "(concat blank ?a)" => "?a"),
        rewrite!("remove-noOp-concat-2"; "(concat ?a blank)" => "?a"),
        // Kill if only a single drawCommand, and saveLayer is noOp.
        // SaveLayer alpha might have been merged into single drawCommand.
        rewrite!("kill-merge"; 
                 "(merge 
                        ?dst 
                        (drawCommand ?x ?p) 
                        (mergeParams 
                            ?mergeIndex
                            (paint 
                                (color 255 0 0 0) 
                                (effects false)
                            )
                        )
                    )" 
                 => 
                 "(concat 
                        ?dst 
                        (drawCommand ?x ?p)
                  )"),

        rewrite!("apply-alpha-on-src"; 
                 "(merge 
                        ?dst 
                        ?src 
                        (mergeParams
                            ?mergeIndex
                            (paint 
                                (color ?a ?r ?g ?b) 
                                (effects false)
                            )
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
                                (effects false)
                            )
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
                                (effects false)
                            )
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
                                (effects false)
                            )
                        )
                    )"),
        rewrite!("remove-merge-blank"; 
                 "(merge 
                        ?layer 
                        blank 
                        ?mergeParams
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
                                        (effects false)
                                    )
                                )
                            )
                        " => "(drawCommand 
                                    ?x 
                                    (paint
                                        (color ?a ?r ?g ?b)
                                        (effects false)
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

		   	    let merged = expr.add(SkiLang::Merge([dst, src, merge_params]));
		   		drawStack.push((StackOp::Surface, merged));
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
				   let mergeParams = expr.add(SkiLang::MergeParams([index, paint]));
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

            let can_reconstruct = !paint.effects.is_some();

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


// TODO: Change this to color_proto_to_expr
fn color_expr(expr: &mut RecExpr<SkiLang>, aVal:i32, rVal:i32, gVal:i32, bVal:i32) -> Id {
    let a = expr.add(SkiLang::Num(aVal));
    let r = expr.add(SkiLang::Num(rVal));
    let g = expr.add(SkiLang::Num(gVal));
    let b = expr.add(SkiLang::Num(bVal));
    expr.add(SkiLang::Color([a, r, g, b]))
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

    let effectsOp = match &skPaint {
       	Some(skPaint) => {
            if skPaint.effects.is_some() {
                let exists = expr.add(SkiLang::Exists(true));
                expr.add(SkiLang::Effects([exists]))
            } else {
                let exists = expr.add(SkiLang::Exists(false));
                expr.add(SkiLang::Effects([exists]))
            }
       	},
       	None => {
            let exists = expr.add(SkiLang::Exists(false));
            expr.add(SkiLang::Effects([exists]))
        }
    };
    expr.add(SkiLang::Paint([color, effectsOp]))
}

fn paint_expr_to_proto(expr: &RecExpr<SkiLang>, id: Id) -> SkPaint {
    let paint_param_ids = match expr[id] {
        SkiLang::Paint(ids) => ids,
        _ => panic!("Attempting to convert a non paint expr to proto")
    };
    let color = Some(color_expr_to_proto(expr, paint_param_ids[0]));
    let effects = match expr[paint_param_ids[1]] {
        SkiLang::Effects(ids) => {
            match &expr[ids[0]] {
                SkiLang::Exists(value) => {
                    if *value {
                        Some(Effects {})
                    } else {
                        None
                    }
                },
                _ => panic!("Effects first parameter not exists!")
            }
        },
        _ => panic!("Paint expr second parameter not Effects")
    };

    SkPaint {
        color,
        effects
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
