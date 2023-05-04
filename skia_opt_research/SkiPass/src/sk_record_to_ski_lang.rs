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

struct SkCanvasState {
    surface_id: Id,
    state_id: Id
}

pub fn convert_sk_record_to_ski_lang_expr<'a, I>(sk_records: &mut I) -> RecExpr<SkiLang> 
where
    I: Iterator<Item = &'a SkRecords> + 'a,
{
    let mut egraph = EGraph::default().with_explanations_enabled();
    let ski_lang_surface = build_expr_2(sk_records, &mut egraph);
    egraph.id_to_expr(ski_lang_surface)
}

fn build_expr_2<'a, I>(sk_records: &mut I, egraph: &mut EGraph<SkiLang, ()>) -> Id  
where
    I: Iterator<Item = &'a SkRecords> + 'a,
{
    let mut state_op_expr:RecExpr<SkiLang> = RecExpr::default();
    state_op_expr.add(SkiLang::BlankState);
    let state_op = egraph.add_expr(&state_op_expr);

    let mut surface_expr:RecExpr<SkiLang> = RecExpr::default();
    surface_expr.add(SkiLang::BlankSurface);
    let surface = egraph.add_expr(&surface_expr);

    let mut canvas_state = SkCanvasState {
        state_id: state_op,
        surface_id: surface
    };

    loop {
        println!("{}", egraph.id_to_expr(canvas_state.surface_id).pretty(40));
        match sk_records.next() {
            Some(sk_record) => {
                match &sk_record.command {
                    Some(Command::Restore(_restore)) => {
                        return canvas_state.surface_id;
                    },
                    Some(Command::Save(_)) => {
                        let inner_surface = build_expr_2(sk_records, egraph);
                        let inner_surface_with_curr_state = egraph.add_expr(&apply_state_on_layer(
                            egraph.id_to_expr(inner_surface),
                            egraph.id_to_expr(canvas_state.state_id)
                        ));
                        let next_surface = egraph.add(
                            SkiLang::Concat([
                                canvas_state.surface_id,
                                inner_surface_with_curr_state
                            ])
                        );
                        canvas_state.surface_id = next_surface;
                    },
                    Some(Command::SaveLayer(save_layer)) => {
                        let merge_params = egraph.add(SkiLang::MergeParams(SkiLangMergeParams {
                            index: sk_record.index,
                            paint: SkiLangPaint::from_proto(&save_layer.paint),
                            has_backdrop: save_layer.backdrop.is_some(),
                            has_bounds: save_layer.bounds.is_some(),
                            bounds: match &save_layer.bounds {
                                Some(bounds) => SkiLangRect::from_bounds_proto(&bounds),
                                None => SkiLangRect::empty()
                            }
                        }));
                        let merge_params_with_state = egraph.add(
                            SkiLang::MergeParamsWithState([merge_params, canvas_state.state_id]
                        ));
                        let inner_surface = build_expr_2(sk_records, egraph);
                        canvas_state.surface_id = egraph.add(
                            SkiLang::Merge([
                                canvas_state.surface_id,
                                inner_surface,
                                merge_params_with_state
                                ])
                            );
                    },
                    _ => {
                        canvas_state = handle_draw_command(
                            sk_record.index, 
                            &sk_record.command.as_ref().unwrap(), 
                            egraph,
                            &canvas_state
                        );
                    }
                }

            },
            None => break,
        }
    }
    return canvas_state.surface_id;
}

fn build_expr<'a, I>(
    sk_records: &mut I, 
    expr: &mut RecExpr<SkiLang>
) -> Id
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

fn handle_draw_command(
    index: i32,
    sk_command: &Command, 
    egraph: &mut EGraph<SkiLang, ()>,
    current_canvas_state: &SkCanvasState,
) -> SkCanvasState
{
    let state_op = current_canvas_state.state_id;
    let surface = current_canvas_state.surface_id;

    let mut next_state_op = state_op;
    let mut next_surface = surface;

    match sk_command {
        Command::Concat44(concat44) => {
            let blank_state = egraph.add(SkiLang::BlankState);
            let concat44_params = egraph.add(SkiLang::M44(
                SkiLangM44::from_skm44_proto(&concat44.matrix.as_ref().unwrap())
            ));
            let op = egraph.add(
                SkiLang::Concat44([
                    blank_state,
                    concat44_params
                ])
            );
            next_state_op = egraph.add_expr(&extend_state_op(
                egraph.id_to_expr(state_op),
                egraph.id_to_expr(op)
            ));

        },
        Command::ClipRect(clip_rect) => {
            let blank_state = egraph.add(SkiLang::BlankState);
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
            let clip_rect_params = egraph.add(
                SkiLang::ClipRectParams(SkiLangClipRectParams {
                    clip_rect_mode,
                    bounds,
                    is_anti_aliased
                }
            ));
            let op = egraph.add(
                SkiLang::ClipRect([
                    blank_state,
                    clip_rect_params
                ])
            );
            next_state_op = egraph.add_expr(&extend_state_op(
                egraph.id_to_expr(state_op),
                egraph.id_to_expr(op)
            ));
        },
        Command::DrawCommand(draw_command) => match draw_command.name.as_str() {
            "ClipPath" | "ClipRRect" => {
                let matrix_op_params = egraph.add(SkiLang::MatrixOpParams(
                    SkiLangMatrixOpParams {index}
                ));
                let blank_state = egraph.add(SkiLang::BlankState);
                let op = egraph.add(
                    SkiLang::Concat44([
                        blank_state,
                        matrix_op_params, 
                    ])
                );
                next_state_op = egraph.add_expr(&extend_state_op(
                    egraph.id_to_expr(state_op),
                    egraph.id_to_expr(op)
                ));
            }
            _ => {
                let draw_command = egraph.add(SkiLang::DrawCommand(
                    SkiLangDrawCommand{
                        index,
                        paint: SkiLangPaint::from_proto(&draw_command.paint),
                    }
                ));
                let draw_command_with_state_expr = apply_state_on_layer(
                    egraph.id_to_expr(draw_command),
                    egraph.id_to_expr(state_op)
                );
                let draw_command_with_state = egraph.add_expr(&draw_command_with_state_expr);
                next_surface = egraph.add(SkiLang::Concat([surface, draw_command_with_state]));
            }
        },
        _ => panic!("Unimplemented")
    }
    SkCanvasState {
        state_id: next_state_op,
        surface_id:next_surface 
    }
}

fn apply_state_on_layer(
    layer_expr: RecExpr<SkiLang>,
    state_op_expr: RecExpr<SkiLang>
) -> RecExpr<SkiLang> {
    let state_op_string = state_op_expr.pretty(0);
    let layer_string = layer_expr.pretty(0);
    state_op_string.replace("blankState", &layer_string).parse().unwrap()
}

fn extend_state_op(
    state_op_expr: RecExpr<SkiLang>,
    op_expr: RecExpr<SkiLang>
) -> RecExpr<SkiLang> {
    let state_op_string = state_op_expr.pretty(0);
    let op_expr = op_expr.pretty(0);
    state_op_string.replace("blankState", &op_expr).parse().unwrap()
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
