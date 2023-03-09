use egg::*;

use crate::ski_lang::SkiLang;
use crate::ski_lang_converters::{
    bounds_proto_to_expr,
    bounds_proto_to_rect_expr,
    bounds_expr_to_proto,
    paint_proto_to_expr,
};
use crate::protos::{
    SkRecords,
    ClipOp,
    sk_records::Command, 
};

pub struct SkiLangExpr {
    pub expr: RecExpr<SkiLang>,
    pub id: Id,
}

#[derive(Clone, Debug)]
enum StackOp {
    State,
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
    let _count = 0;
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
               Some(Command::Save(_save)) => {
                    drawStack.push((StackOp::Save, expr.add(SkiLang::NoOp)));
               },
               Some(Command::SaveLayer(save_layer)) => {
                   let index = expr.add(SkiLang::Num(skRecords.index));

                   let paint = paint_proto_to_expr(expr, &save_layer.paint);


                   let backdrop_exists = expr.add(SkiLang::Exists(save_layer.backdrop.is_some()));
                   let backdrop = expr.add(SkiLang::Backdrop([backdrop_exists]));

                   let saveLayerBounds = bounds_proto_to_expr(expr, &save_layer.bounds);

                   // The stack will fill in the right state, for now we put in a identity state inside.
                   let stateAtMerge = expr.add(SkiLang::BlankState);

                   let mergeParams = expr.add(SkiLang::MergeParams([index, paint, backdrop, saveLayerBounds, stateAtMerge]));
		   		   drawStack.push((StackOp::SaveLayer, mergeParams));
               },
               Some(Command::Restore(_restore)) => {
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

fn reduceStateStack(
    expr: &mut RecExpr<SkiLang>,
    stateStack : &mut Vec<(StackOp, Id)>, 
) {
    stateStack.push((StackOp::State, expr.add(SkiLang::BlankState)));
    while stateStack.len() != 1 {
        let (e1_type, e1) = stateStack.pop().unwrap();
        let (e2_type, e2) = stateStack.pop().unwrap();
        match e2_type {
            StackOp::MatrixOp => {
                let nxt = expr.add(SkiLang::MatrixOp([e1, e2]));
                stateStack.push((StackOp::State, nxt));
            },
            StackOp::ClipRect => {
                let nxt = expr.add(SkiLang::ClipRect([e1, e2]));
                stateStack.push((StackOp::State, nxt));
            },
            StackOp::Save => {
                stateStack.push((e1_type, e1));
            },
            _ => {
                panic!("StateStack has non-state ops!");
            },
        };
    }
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
				let src = e1;
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

				reduceStack(expr, drawStack, false);
                let dst = drawStack.pop().unwrap().1;

                let bounds = match expr[merge_params] {
                    SkiLang::MergeParams(ids) => bounds_expr_to_proto(expr, ids[3]),
                    _ => panic!("SaveLayer stack does not have mergeParams")
                };

                let mut mergeStateStack = stateStack.clone();
                reduceStateStack(expr, &mut mergeStateStack);
                let mergeState = mergeStateStack.pop().unwrap().1;

                let corrected_merge_params = match expr[merge_params] {
                    SkiLang::MergeParams(ids) => {
                        expr.add(SkiLang::MergeParams([ids[0], ids[1], ids[2], ids[3], mergeState]))
                    },
                    _ => panic!("SaveLayer stack does not have mergeParams")
                };
		   	    let merged = expr.add(SkiLang::Merge([dst, src, corrected_merge_params]));
		   		drawStack.push((StackOp::Surface, merged));

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
            },
            StackOp::State => {
                panic!("Trying to reduce a stateStack in drawStack method");
            }
        };
    }
}

