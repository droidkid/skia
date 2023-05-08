use egg::*;
use crate::protos::{sk_records::Command, ClipOp, SkRecords};
use crate::ski_lang::{
    SkiLang, 
    SkiLangPaint,
    SkiLangRect,
    SkiLangClipRectMode, 
    SkiLangClipRectParams,
    SkiLangOtherStateOpParams,
    SkiLangMergeParams,
    SkiLangDrawCommand,
    SkiLangM44
};

struct SkCanvasState {
    surface_id: Id,
    state_id: Id
}

pub fn convert_sk_record_to_ski_lang_expr<'a, I>(sk_records: &mut I) -> RecExpr<SkiLang> 
where
    I: Iterator<Item = &'a SkRecords> + 'a,
{
    let mut egraph = EGraph::default().with_explanations_enabled();
    let ski_lang_surface = build_expr(sk_records, &mut egraph);
    egraph.id_to_expr(ski_lang_surface)
}

fn build_expr<'a, I>(sk_records: &mut I, egraph: &mut EGraph<SkiLang, ()>) -> Id  
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
        match sk_records.next() {
            Some(sk_record) => {
                match &sk_record.command {
                    Some(Command::Restore(_restore)) => {
                        return canvas_state.surface_id;
                    },
                    Some(Command::Save(_)) => {
                        let inner_surface = build_expr(sk_records, egraph);
                        let inner_surface_with_curr_state = egraph.add(
                            SkiLang::ApplyState([inner_surface, canvas_state.state_id])
                        );
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
                        let inner_surface = build_expr(sk_records, egraph);
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
            let concat44_params = egraph.add(SkiLang::M44(
                SkiLangM44::from_skm44_proto(&concat44.matrix.as_ref().unwrap())
            ));
            next_state_op = egraph.add(
                SkiLang::Concat44([
                    state_op,
                    concat44_params
                ])
            );

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
            let clip_rect_params = egraph.add(
                SkiLang::ClipRectParams(SkiLangClipRectParams {
                    clip_rect_mode,
                    bounds,
                    is_anti_aliased
                }
            ));
            next_state_op = egraph.add(
                SkiLang::ClipRect([
                    state_op,
                    clip_rect_params
                ])
            );
        },
        Command::DrawCommand(draw_command) => match draw_command.name.as_str() {
            "ClipPath" | "ClipRRect" => {
                let matrix_op_params = egraph.add(SkiLang::OtherStateOpParams(
                    SkiLangOtherStateOpParams {index}
                ));
                next_state_op = egraph.add(
                    SkiLang::Concat44([
                        state_op,
                        matrix_op_params, 
                    ])
                );
            }
            _ => {
                let draw_command = egraph.add(SkiLang::DrawCommand(
                    SkiLangDrawCommand{
                        index,
                        paint: SkiLangPaint::from_proto(&draw_command.paint),
                    }
                ));
                let draw_command_with_state = egraph.add(
                    SkiLang::ApplyState([draw_command, state_op])
                );
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
