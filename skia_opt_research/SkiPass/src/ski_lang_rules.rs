use egg::*;
use crate::ski_lang::{
    SkiLang,
    SkiLangBlendMode,
    SkiLangApplyAlphaParams
};

pub fn make_rules() -> Vec<Rewrite<SkiLang, ()>> {
    let mut rules = vec![
        rewrite!("kill-blankSurface-clip"; "(clipRect blankSurface ?p)" => "blankSurface"),
        rewrite!("kill-blankSurface-mOp"; "(matrixOp blankSurface ?p)" => "blankSurface"),
        rewrite!("kill-blankSurface-concat44"; "(concat44 blankSurface ?p)" => "blankSurface"),
        rewrite!("kill-blankSurface-concat-1"; "(concat blankSurface ?a)" => "?a"),
        rewrite!("kill-blankSurface-concat-2"; "(concat ?a blankSurface)" => "?a"),
        rewrite!("kill-blank-alpha"; "(apply_alpha ?p blankSurface)" => "blankSurface"),
        rewrite!("kill-noOp-alpha"; "(apply_alpha ([alpha:255]) ?src)" => "?src"),
        rewrite!("kill-merge-blankSurface";  
            "(merge 
                ?layer blankSurface 
                (merge_params_with_state 
                    ?merge_params ?state_ops))" => "?layer"
            if merge_is_simple_src_over("?merge_params")
        ),
        rewrite!("kill-noOp-merge";  
            "(merge ?dst ?src ?merge_params_with_state)" => "(concat ?dst ?src)"
            if merge_is_src_over_and_paint_is_opaque_and_no_state_and_no_bounds("?merge_params_with_state")
        ),
        rewrite!("extract-alpha-virtual-op";
            "(merge 
                ?dst ?src
                (merge_params_with_state 
                    ?merge_params ?state_ops)
            )" => {
                AlphaVirtualOpApplier {
                    merge_params: "?merge_params".parse().unwrap(),
                    alpha_params: "?alpha_params".parse().unwrap(),
                    merge_params_without_alpha: "?merge_params_without_alpha".parse().unwrap(),
                    expr: "(merge 
                            ?dst (apply_alpha ?alpha_params ?src) 
                            (merge_params_with_state 
                                ?merge_params_without_alpha ?state_ops)
                        )".parse().unwrap(),
            }
         } if merge_is_simple_src_over("?merge_params")),
         rewrite!("alpha-virtual-op-revert";
            "(merge
                ?dst (apply_alpha ?alpha_params ?src)
                (merge_params_with_state 
                    ?merge_params ?state_ops)
            )" => {
                AlphaVirtualOpReverter {
                    expr: "(merge 
                            ?dst ?src
                            (merge_params_with_state
                                ?merge_params_with_alpha ?state_ops
                            )
                    )".parse().unwrap(),
                    merge_params: "?merge_params".parse().unwrap(),
                    alpha_params: "?alpha_params".parse().unwrap(),
                    merge_params_with_alpha: "?merge_params_with_alpha".parse().unwrap()
                }
            } if merge_is_simple_src_over("?merge_params")
        ),
        rewrite!("fold-alpha";
            "(apply_alpha ?alpha_params ?src)" => {
                FoldAlpha {
                    alpha_params: "?alpha_params".parse().unwrap(),
                    src: "?src".parse().unwrap(),
                    folded_draw_command: "?draw_command".parse().unwrap(),
                    expr: "?draw_command".parse().unwrap()
                }
            }
        )
    ];
    rules
}

fn merge_is_simple_src_over(var: &'static str) -> impl Fn(&mut EGraph<SkiLang, ()>, Id, &Subst) -> bool {
    let var: Var = var.parse().unwrap();
    move |egraph, _, subst| {
        let merge_params_expr = egraph.id_to_expr(subst[var]);
        let root = merge_params_expr.as_ref().last().unwrap();
        let merge_params = match root {
            SkiLang::MergeParams(merge_params) => merge_params,
            _ => panic!("first id of merge_params_with_state is not merge_params")
        };
        merge_params.paint.blend_mode == SkiLangBlendMode::SrcOver 
        && !merge_params.paint.has_filters
        && !merge_params.has_backdrop
    }
}

fn merge_is_src_over_and_paint_is_opaque_and_no_state_and_no_bounds(var: &'static str) -> impl Fn(&mut EGraph<SkiLang, ()>, Id, &Subst) -> bool {
    let var: Var = var.parse().unwrap();
    move |egraph, _, subst| {
        let merge_params_with_state_expr = egraph.id_to_expr(subst[var]);
        let root = merge_params_with_state_expr.as_ref().last().unwrap();
        let (merge_params_id, state_ops_id) = match root {
            SkiLang::MergeParamsWithState(ids) => (ids[0], ids[1]),
            _ => panic!("operator is not merge_params_with_state")
        };
        let merge_params = match &merge_params_with_state_expr[merge_params_id] {
            SkiLang::MergeParams(merge_params) => merge_params,
            _ => panic!("first id of merge_params_with_state is not merge_params")
        };
        let is_blank_state = match &merge_params_with_state_expr[state_ops_id] {
            SkiLang::BlankState => true,
            _ => false,
        };

        is_blank_state && merge_params.paint.color.a == 255 && 
        !merge_params.has_bounds &&
        merge_params.paint.blend_mode == SkiLangBlendMode::SrcOver && 
        !merge_params.paint.has_filters && 
        !merge_params.has_backdrop
    }
}

struct AlphaVirtualOpApplier {
    merge_params: Var,
    alpha_params: Var,
    merge_params_without_alpha: Var,
    expr: Pattern<SkiLang>
}

impl Applier<SkiLang, ()> for AlphaVirtualOpApplier {
    fn apply_one(
        &self,
        egraph: &mut EGraph<SkiLang, ()>,
        matched_id: Id,
        subst: &Subst,
        searcher_pattern: Option<&PatternAst<SkiLang>>,
        rule_name: Symbol,
    ) -> Vec<Id> {
        let merge_params_expr = egraph.id_to_expr(subst[self.merge_params]);
        let root = merge_params_expr.as_ref().last().unwrap();
        let merge_params = match root {
            SkiLang::MergeParams(merge_params) => merge_params,
            _ => panic!("first id of merge_params_with_state is not merge_params")
        };

        let mut subst = subst.clone();
        subst.insert(
            self.alpha_params, 
            egraph.add(SkiLang::ApplyAlphaParams(
                SkiLangApplyAlphaParams {
                    alpha: merge_params.paint.color.a
                }
            ))
        );

        let mut merge_params_without_alpha = merge_params.clone();
        merge_params_without_alpha.paint.color.a = 255;
        subst.insert(
            self.merge_params_without_alpha,
            egraph.add(SkiLang::MergeParams(merge_params_without_alpha))
        );
        self.expr.apply_one(egraph, matched_id, &subst, searcher_pattern, rule_name)
    }
}

struct AlphaVirtualOpReverter {
    merge_params: Var,
    alpha_params: Var,
    merge_params_with_alpha: Var,
    expr: Pattern<SkiLang>
}

impl Applier<SkiLang, ()> for AlphaVirtualOpReverter {
    fn apply_one(
        &self,
        egraph: &mut EGraph<SkiLang, ()>,
        matched_id: Id,
        subst: &Subst,
        searcher_pattern: Option<&PatternAst<SkiLang>>,
        rule_name: Symbol,
    ) -> Vec<Id> {
        let alpha_params_expr = egraph.id_to_expr(subst[self.alpha_params]);
        let root = alpha_params_expr.as_ref().last().unwrap();
        let alpha_params = match root {
            SkiLang::ApplyAlphaParams(alpha_params) => alpha_params,
            _ => panic!("Not ApplyAlpha")
        };

        let merge_params_expr = egraph.id_to_expr(subst[self.merge_params]);
        let root = merge_params_expr.as_ref().last().unwrap();
        let merge_params = match root {
            SkiLang::MergeParams(merge_params) => merge_params,
            _ => panic!("Not MergeParams")
        };

        let mut subst = subst.clone();
        let mut merge_params_with_alpha = merge_params.clone();
        let layer_alpha = merge_params_with_alpha.paint.color.a;
        let alpha_value = alpha_params.alpha;
        let merged_alpha = (alpha_value * layer_alpha) / 255;
        merge_params_with_alpha.paint.color.a = merged_alpha;
        subst.insert(
            self.merge_params_with_alpha, 
            egraph.add(SkiLang::MergeParams(merge_params_with_alpha))
        );
        self.expr.apply_one(egraph, matched_id, &subst, searcher_pattern, rule_name)
    }
}

struct FoldAlpha {
    alpha_params: Var,
    src: Var,
    folded_draw_command: Var,
    expr: Pattern<SkiLang>
}

impl Applier<SkiLang, ()> for FoldAlpha {
    fn apply_one(
        &self,
        egraph: &mut EGraph<SkiLang, ()>,
        matched_id: Id,
        subst: &Subst,
        searcher_pattern: Option<&PatternAst<SkiLang>>,
        rule_name: Symbol,
    ) -> Vec<Id> {
        let matched_expr  = egraph.id_to_expr(matched_id);
        let alpha_params_expr = egraph.id_to_expr(subst[self.alpha_params]);
        let root = alpha_params_expr.as_ref().last().unwrap();
        let alpha_params = match root {
            SkiLang::ApplyAlphaParams(alpha_params) => alpha_params,
            _ => panic!("Not ApplyAlpha")
        };
        let mut draw_command = None;
        let surface = egraph.id_to_expr(subst[self.src]);
        for e in &egraph[subst[self.src]].nodes {
            match e {
                SkiLang::DrawCommand(draw_command_node) => {
                    draw_command = Some(draw_command_node);
                    break;
                },
                _ => continue,
            };
        }
        if draw_command.is_none() {
            println!("Not a Draw Command!");
            return vec![];
        }
        let draw_command = draw_command.unwrap();
        let mut folded_draw_command = draw_command.clone();
        folded_draw_command.paint.color.a = (draw_command.paint.color.a * alpha_params.alpha) / 255;
        let mut subst = subst.clone();
        subst.insert(
            self.folded_draw_command, 
            egraph.add(SkiLang::DrawCommand(folded_draw_command))
        );
        self.expr.apply_one(egraph, matched_id, &subst, searcher_pattern, rule_name)
    }
}
