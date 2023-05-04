use egg::*;
use crate::protos::{sk_records::Command, ClipOp, SkRecords};
use crate::ski_lang::{
    SkiLang, 
    SkiLangPaint,
    SkiLangRect,
    SkiLangClipRectMode, 
    SkiLangClipRectParams,
    SkiLangMatrixOpParams,
    SkiLangMergeParams,
    SkiLangDrawCommand,
    SkiLangM44
};

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

pub fn convert_sk_record_to_ski_lang_expr<'a, I>(sk_records: &mut I) -> RecExpr<SkiLang> 
where
    I: Iterator<Item = &'a SkRecords> + 'a,
{
    let mut expr = RecExpr::default();
    build_expr(sk_records, &mut expr);
    expr
}

fn build_expr<'a, I>(sk_records: &mut I, expr: &mut RecExpr<SkiLang>) -> Id
where
    I: Iterator<Item = &'a SkRecords> + 'a,
{
    let mut command_stack: Vec<(StackOp, Id)> = vec![];
    loop {
        match sk_records.next() {
            Some(sk_record) => {
                match &sk_record.command {
                    Some(Command::Restore(_restore)) => {
                        reduce_stack(expr, &mut command_stack, true);
                        continue;
                    },
                    _ => {}
                };
                command_stack.push(to_command_stack_entry(
                    sk_record.index,
                    &sk_record.command.as_ref().unwrap(), 
                    expr
                ));
            }
            None => break,
        }
    }
    reduce_stack(expr, &mut command_stack, false);
    command_stack[0].1
}

fn to_command_stack_entry(
    index: i32,
    sk_command: &Command, 
    expr: &mut RecExpr<SkiLang>
) -> (StackOp, Id) 
{
    match sk_command {
        Command::DrawCommand(draw_command) => match draw_command.name.as_str() {
            "ClipPath" | "ClipRRect" => {
                (StackOp::MatrixOp, expr.add(SkiLang::MatrixOpParams(
                    SkiLangMatrixOpParams {index}
                )))
            }
            _ => {
                (StackOp::Surface, expr.add(SkiLang::DrawCommand(
                    SkiLangDrawCommand{
                        index,
                        paint: SkiLangPaint::from_proto(&draw_command.paint),
                    }
                )))
            }
        },
        Command::ClipRect(clip_rect) => {
            let bounds = SkiLangRect::from_bounds_proto(&clip_rect.bounds.as_ref().unwrap());
            let clip_rect_mode = 
                if clip_rect.clip_op == ClipOp::Difference.into() {
                    SkiLangClipRectMode::Diff
                } else if clip_rect.clip_op == ClipOp::Intersect.into() {
                    SkiLangClipRectMode::Intersect
                } else {
                    panic!("Unknown clipOp mode")
                };
            let is_anti_aliased = clip_rect.do_anti_alias;
            (StackOp::ClipRect, expr.add(
                SkiLang::ClipRectParams(SkiLangClipRectParams {
                    clip_rect_mode,
                    bounds,
                    is_anti_aliased
                }
            )))
        }
        Command::Concat44(concat44) => {
            (StackOp::Concat44, expr.add(SkiLang::M44(
                SkiLangM44::from_skm44_proto(&concat44.matrix.as_ref().unwrap())
            )))
        }
        Command::Save(_save) => {
            (StackOp::Save, expr.add(SkiLang::NoOp))
        }
        Command::SaveLayer(save_layer) => {
            let merge_params = expr.add(SkiLang::MergeParams(SkiLangMergeParams {
                index,
                paint: SkiLangPaint::from_proto(&save_layer.paint),
                has_backdrop: save_layer.backdrop.is_some(),
                has_bounds: save_layer.bounds.is_some(),
                bounds: match &save_layer.bounds {
                    Some(bounds) => SkiLangRect::from_bounds_proto(&bounds),
                    None => SkiLangRect::empty()
                }
            }));
            // The state will be filled in when the stack is unpacked.
            let state_at_merge = expr.add(SkiLang::BlankState);
            let merge_params_with_state = expr.add(SkiLang::MergeParamsWithState([
                merge_params,
                state_at_merge
            ]));
            (StackOp::SaveLayer, merge_params_with_state)
        },
        _ => {
            panic!("Unhandled Draw Command type!")
        }
    }
}

fn reduce_stack(
    expr: &mut RecExpr<SkiLang>,
    draw_command_stack: &mut Vec<(StackOp, Id)>,
    from_restore: bool,
) {
    draw_command_stack.push((StackOp::Surface, expr.add(SkiLang::BlankSurface)));
    while draw_command_stack.len() != 1 {
        let (e1_type, e1) = draw_command_stack.pop().unwrap();
        let (e2_type, e2) = draw_command_stack.pop().unwrap();
        match e2_type {
            StackOp::SaveLayer => {
                let src = e1;
                let merge_params_with_blank_state = e2;
                if !from_restore {
                    // We're not done with this saveLayer, this saveLayer is a barrier
                    // for some other saveLayer. So push them back to the stack and exit.
                    draw_command_stack.push((e2_type, e2));
                    draw_command_stack.push((e1_type, e1));
                    return;
                }
                let mut state_stack: Vec<(StackOp, Id)> = vec![];
                for op in draw_command_stack.iter() {
                    match op.0 {
                        StackOp::MatrixOp => state_stack.push(op.clone()),
                        StackOp::ClipRect => state_stack.push(op.clone()),
                        StackOp::Concat44 => state_stack.push(op.clone()),
                        StackOp::Save => state_stack.push(op.clone()),
                        StackOp::SaveLayer => state_stack.clear(),
                        _ => {}
                    }
                }
                reduce_stack(expr, draw_command_stack, false);
                let dst = draw_command_stack.pop().unwrap().1;
                let merge_params_id = match expr[merge_params_with_blank_state] {
                    SkiLang::MergeParamsWithState(ids) => ids[0],
                    _ => panic!("SaveLayer stack does not have merge params")
                };
                let mut merge_state_stack = state_stack.clone();
                reduce_stack_to_state_expr(expr, &mut merge_state_stack);
                let merge_state_id = merge_state_stack.pop().unwrap().1;
                let merge_params_with_correct_state = expr.add(
                    SkiLang::MergeParamsWithState([
                        merge_params_id,
                        merge_state_id
                    ])
                );
                let merged = expr.add(SkiLang::Merge([
                    dst, 
                    src, 
                    merge_params_with_correct_state
                ]));
                draw_command_stack.push((StackOp::Surface, merged));
                draw_command_stack.append(&mut state_stack);
                if from_restore {
                    break;
                }
            }
            StackOp::Save => {
                draw_command_stack.push((e1_type, e1));
                if from_restore {
                    break;
                }
            }
            StackOp::MatrixOp => {
                let nxt = expr.add(SkiLang::MatrixOp([e1, e2]));
                draw_command_stack.push((StackOp::Surface, nxt));
            }
            StackOp::Concat44 => {
                let nxt = expr.add(SkiLang::Concat44([e1, e2]));
                draw_command_stack.push((StackOp::Surface, nxt));
            }
            StackOp::ClipRect => {
                let nxt = expr.add(SkiLang::ClipRect([e1, e2]));
                draw_command_stack.push((StackOp::Surface, nxt));
            }
            StackOp::Surface => {
                let nxt = expr.add(SkiLang::Concat([e2, e1]));
                draw_command_stack.push((StackOp::Surface, nxt));
            }
            StackOp::State => {
                panic!("Trying to reduce a state_stack in draw_command_stack method");
            }
        };
    }
}

fn reduce_stack_to_state_expr(expr: &mut RecExpr<SkiLang>, state_stack: &mut Vec<(StackOp, Id)>) {
    state_stack.push((StackOp::State, expr.add(SkiLang::BlankState)));
    while state_stack.len() != 1 {
        let (e1_type, e1) = state_stack.pop().unwrap();
        let (e2_type, e2) = state_stack.pop().unwrap();
        match e2_type {
            StackOp::MatrixOp => {
                let nxt = expr.add(SkiLang::MatrixOp([e1, e2]));
                state_stack.push((StackOp::State, nxt));
            }
            StackOp::ClipRect => {
                let nxt = expr.add(SkiLang::ClipRect([e1, e2]));
                state_stack.push((StackOp::State, nxt));
            }
            StackOp::Concat44 => {
                let nxt = expr.add(SkiLang::Concat44([e1, e2]));
                state_stack.push((StackOp::State, nxt));
            }
            StackOp::Save => {
                state_stack.push((e1_type, e1));
            }
            _ => {
                panic!("StateStack has non-state ops!");
            }
        };
    }
}
