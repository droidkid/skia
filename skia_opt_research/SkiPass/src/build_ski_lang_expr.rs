use egg::*;

use crate::ski_lang::SkiLang;
use crate::ski_lang_converters::{
    bounds_proto_to_expr,
    bounds_proto_to_rect_expr,
    bounds_expr_to_proto,
    paint_proto_to_expr,
    color_proto_to_expr,
    unpack_float,
    unpack_rect_to_bounds,
};
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
    ClipOp,
    sk_paint::Blender,
    sk_paint::ImageFilter,
    sk_paint::ColorFilter,
    sk_paint::PathEffect,
    sk_paint::MaskFilter,
    sk_paint::Shader,
    sk_records::Command, 
};

pub struct SkiLangExpr {
    pub expr: RecExpr<SkiLang>,
    pub id: Id,
}

#[derive(Clone, Debug)]
enum StackOp {
    Surface,
    MatrixOp,
    ClipRect,
    Save,
    SaveLayer,
}

pub fn build_expr<'a, I>(
    skRecordsIter: &mut I, 
    expr: &mut RecExpr<SkiLang>,
) -> Id
where
I: Iterator<Item = &'a SkRecords> + 'a,
{
    let mut drawStack: Vec<(StackOp, Id)> = vec![];
    let mut count = 0;
    loop {

    match skRecordsIter.next() {
       Some(skRecords) => {
           match &skRecords.command {
               Some(Command::DrawCommand(draw_command)) => {
                   match draw_command.name.as_str() {
                       "ClipPath" | "ClipRRect" | "Concat44" => {
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
               Some(Command::ClipRect(clip_rect)) => {
                    let clipOpRect = bounds_proto_to_rect_expr(expr, &clip_rect.bounds);
                    let clipOp = if clip_rect.clip_op == ClipOp::Difference.into() {
                        expr.add(SkiLang::ClipOp_Diff)
                    } else if clip_rect.clip_op == ClipOp::Intersect.into() {
                        expr.add(SkiLang::ClipOp_Intersect)
                    } else {
                        panic!("Unknown clipOp mode")
                    };
                    let isAntiAlias = expr.add(SkiLang::Exists(clip_rect.do_anti_alias));
                    let clipRectParams = expr.add(SkiLang::ClipRectParams([clipOpRect, clipOp, isAntiAlias]));
                    drawStack.push((StackOp::ClipRect, clipRectParams));
               },
               Some(Command::Save(save)) => {
                    drawStack.push((StackOp::Save, expr.add(SkiLang::NoOp)));
               },
               Some(Command::SaveLayer(save_layer)) => {
                   let index = expr.add(SkiLang::Num(skRecords.index));

                   let paint = paint_proto_to_expr(expr, &save_layer.paint);


                   let backdrop_exists = expr.add(SkiLang::Exists(save_layer.backdrop.is_some()));
                   let backdrop = expr.add(SkiLang::Backdrop([backdrop_exists]));

                   // We push the saveLayer bounds to a clipRect inside the saveLayer.
                   // TODO: Figure out if this is the right way to handle this.
                   let saveLayerBounds = bounds_proto_to_expr(expr, &None);

                   let mergeParams = expr.add(SkiLang::MergeParams([index, paint, backdrop, saveLayerBounds]));
		   		   drawStack.push((StackOp::SaveLayer, mergeParams));

                   // The clipRect simulating saveLayer bounds.
                   if save_layer.bounds.is_some() {
                        let clipRectBounds = bounds_proto_to_rect_expr(expr, &save_layer.bounds);
                        let clipOp = expr.add(SkiLang::ClipOp_Intersect);
                        let clipDoAntiAlias = expr.add(SkiLang::Exists(true));
                        let clipRectParams = expr.add(SkiLang::ClipRectParams([clipRectBounds, clipOp, clipDoAntiAlias]));
                        drawStack.push((StackOp::ClipRect, clipRectParams));

                        let blank = expr.add(SkiLang::Blank);
                        drawStack.push((StackOp::Surface, blank));
                   }
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
                    	StackOp::ClipRect => stateStack.push(op.clone()),
                        StackOp::Save => stateStack.push(op.clone()),
                        StackOp::SaveLayer => stateStack.clear(),
                        _ => {}
                    }
                }

                let blank = expr.add(SkiLang::Blank);
                stateStack.push((StackOp::Surface, blank));

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
                };
		   		drawStack.append(&mut stateStack);
                if from_restore {
                    break;
                }
			},
            StackOp::Save=> {
                drawStack.push((e1_type, e1));
                if from_restore {
                    break;
                }
            },
            StackOp::MatrixOp => {
                let nxt = expr.add(SkiLang::MatrixOp([e1, e2]));
                drawStack.push((StackOp::Surface, nxt));
            },
            StackOp::ClipRect => {
                let nxt = expr.add(SkiLang::ClipRect([e1, e2]));
                drawStack.push((StackOp::Surface, nxt));
            },
            StackOp::Surface => {
                let nxt = expr.add(SkiLang::Concat([e2, e1]));
                drawStack.push((StackOp::Surface, nxt));
            }
        };
    }
}

