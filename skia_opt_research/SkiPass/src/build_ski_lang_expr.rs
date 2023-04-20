use egg::*;

use crate::protos::{sk_records::Command, ClipOp, SkRecords};
use crate::ski_lang::{SkiLang, SkiLangClipRectMode, SkiLangClipRectParams, SkiLangRect};
use crate::ski_lang_converters::{
    bounds_expr_to_proto, bounds_proto_to_expr, bounds_proto_to_rect, bounds_proto_to_rect_expr,
    paint_proto_to_expr, skm44_to_expr,
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
    Concat44,
    ClipRect,
    Save,
    SaveLayer,
}

pub fn build_expr<'a, I>(skRecordsIter: &mut I, expr: &mut RecExpr<SkiLang>) -> Id
where
    I: Iterator<Item = &'a SkRecords> + 'a,
{
    let mut drawStack: Vec<(StackOp, Id)> = vec![];
    let _count = 0;
    loop {
        match skRecordsIter.next() {
            Some(skRecords) => {
                match &skRecords.command {
                    Some(Command::DrawCommand(draw_command)) => match draw_command.name.as_str() {
                        "ClipPath" | "ClipRRect" => {
                            let matrixOpIndex = expr.add(SkiLang::Num(skRecords.index));
                            let matrixOpParams = expr.add(SkiLang::MatrixOpParams([matrixOpIndex]));
                            drawStack.push((StackOp::MatrixOp, matrixOpParams));
                        }
                        _ => {
                            let drawCommandIndex = expr.add(SkiLang::Num(skRecords.index));
                            let drawCommandPaint = paint_proto_to_expr(expr, &draw_command.paint);
                            let drawOpCommand = expr
                                .add(SkiLang::DrawCommand([drawCommandIndex, drawCommandPaint]));
                            drawStack.push((StackOp::Surface, drawOpCommand));
                        }
                    },
                    Some(Command::ClipRect(clip_rect)) => {
                        let bounds = bounds_proto_to_rect(&clip_rect.bounds);
                        let clipRectMode = if clip_rect.clip_op == ClipOp::Difference.into() {
                            SkiLangClipRectMode::Diff
                        } else if clip_rect.clip_op == ClipOp::Intersect.into() {
                            SkiLangClipRectMode::Intersect
                        } else {
                            panic!("Unknown clipOp mode")
                        };
                        let doAntiAlias = clip_rect.do_anti_alias;
                        let clipRectParams =
                            expr.add(SkiLang::ClipRectParams(SkiLangClipRectParams {
                                clipRectMode,
                                bounds,
                                doAntiAlias,
                            }));
                        drawStack.push((StackOp::ClipRect, clipRectParams));
                    }
                    Some(Command::Concat44(concat44)) => {
                        let m44 = skm44_to_expr(expr, &concat44.matrix);
                        drawStack.push((StackOp::Concat44, m44));
                    }
                    Some(Command::Save(_save)) => {
                        drawStack.push((StackOp::Save, expr.add(SkiLang::NoOp)));
                    }
                    Some(Command::SaveLayer(save_layer)) => {
                        let index = expr.add(SkiLang::Num(skRecords.index));

                        let paint = paint_proto_to_expr(expr, &save_layer.paint);

                        let backdrop_exists =
                            expr.add(SkiLang::Bool(save_layer.backdrop.is_some()));
                        let backdrop = expr.add(SkiLang::Backdrop([backdrop_exists]));

                        let saveLayerBounds = bounds_proto_to_expr(expr, &save_layer.bounds);

                        // The stack will fill in the right state, for now we put in a identity state inside.
                        let stateAtMerge = expr.add(SkiLang::BlankState);

                        let mergeParams = expr.add(SkiLang::MergeParams([
                            index,
                            paint,
                            backdrop,
                            saveLayerBounds,
                            stateAtMerge,
                        ]));
                        drawStack.push((StackOp::SaveLayer, mergeParams));
                    }
                    Some(Command::Restore(_restore)) => {
                        reduceStack(expr, &mut drawStack, true);
                    }
                    _ => {
                        panic!("Unsupported SkRecord");
                    }
                    None => {}
                }
            }
            None => break,
        }
    }
    reduceStack(expr, &mut drawStack, false);
    drawStack[0].1
}

fn reduceStateStack(expr: &mut RecExpr<SkiLang>, stateStack: &mut Vec<(StackOp, Id)>) {
    stateStack.reverse();
    stateStack.push((StackOp::State, expr.add(SkiLang::BlankState)));

    while stateStack.len() != 1 {
        let (e1_type, e1) = stateStack.pop().unwrap();
        let (e2_type, e2) = stateStack.pop().unwrap();
        match e2_type {
            StackOp::MatrixOp => {
                let nxt = expr.add(SkiLang::MatrixOp([e1, e2]));
                stateStack.push((StackOp::State, nxt));
            }
            StackOp::ClipRect => {
                let nxt = expr.add(SkiLang::ClipRect([e1, e2]));
                stateStack.push((StackOp::State, nxt));
            }
            StackOp::Concat44 => {
                let nxt = expr.add(SkiLang::Concat44([e1, e2]));
                stateStack.push((StackOp::State, nxt));
            }
            StackOp::Save => {
                stateStack.push((e1_type, e1));
            }
            _ => {
                panic!("StateStack has non-state ops!");
            }
        };
    }
}

fn reduceStack(
    expr: &mut RecExpr<SkiLang>,
    drawStack: &mut Vec<(StackOp, Id)>,
    from_restore: bool,
) {
    drawStack.push((StackOp::Surface, expr.add(SkiLang::BlankSurface)));
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
                        StackOp::Concat44 => stateStack.push(op.clone()),
                        StackOp::Save => stateStack.push(op.clone()),
                        StackOp::SaveLayer => stateStack.clear(),
                        _ => {}
                    }
                }

                let blank = expr.add(SkiLang::BlankSurface);

                reduceStack(expr, drawStack, false);
                let dst = drawStack.pop().unwrap().1;

                let bounds = match expr[merge_params] {
                    SkiLang::MergeParams(ids) => bounds_expr_to_proto(expr, ids[3]),
                    _ => panic!("SaveLayer stack does not have mergeParams"),
                };

                let mut mergeStateStack = stateStack.clone();
                reduceStateStack(expr, &mut mergeStateStack);
                let mergeState = mergeStateStack.pop().unwrap().1;

                let corrected_merge_params = match expr[merge_params] {
                    SkiLang::MergeParams(ids) => expr.add(SkiLang::MergeParams([
                        ids[0], ids[1], ids[2], ids[3], mergeState,
                    ])),
                    _ => panic!("SaveLayer stack does not have mergeParams"),
                };
                let merged = expr.add(SkiLang::Merge([dst, src, corrected_merge_params]));
                drawStack.push((StackOp::Surface, merged));

                drawStack.append(&mut stateStack);
                if from_restore {
                    break;
                }
            }
            StackOp::Save => {
                drawStack.push((e1_type, e1));
                if from_restore {
                    break;
                }
            }
            StackOp::MatrixOp => {
                let nxt = expr.add(SkiLang::MatrixOp([e1, e2]));
                drawStack.push((StackOp::Surface, nxt));
            }
            StackOp::Concat44 => {
                let nxt = expr.add(SkiLang::Concat44([e1, e2]));
                drawStack.push((StackOp::Surface, nxt));
            }
            StackOp::ClipRect => {
                let nxt = expr.add(SkiLang::ClipRect([e1, e2]));
                drawStack.push((StackOp::Surface, nxt));
            }
            StackOp::Surface => {
                let nxt = expr.add(SkiLang::Concat([e2, e1]));
                drawStack.push((StackOp::Surface, nxt));
            }
            StackOp::State => {
                panic!("Trying to reduce a stateStack in drawStack method");
            }
        };
    }
}
