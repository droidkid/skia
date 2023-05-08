use egg::*;
use crate::ski_lang::{
    SkiLang,
    SkiLangBlendMode,
    SkiLangApplyAlphaParams,
    SkiLangRect,
    SkiLangClipRectMode,
    SkiLangClipRectParams
};

pub fn make_rules() -> Vec<Rewrite<SkiLang, ()>> {
    let mut rules = vec![];

    // BlankSurface Rules
    rules.extend([
        rewrite!("blankSurface-clip"; "(clipRect blankSurface ?p)" => "blankSurface"),
        rewrite!("blankSurface-mOp"; "(matrixOp blankSurface ?p)" => "blankSurface"),
        rewrite!("blankSurface-concat44"; "(concat44 blankSurface ?p)" => "blankSurface"),
        rewrite!("blankSurface-concat-1"; "(concat blankSurface ?a)" => "?a"),
        rewrite!("blankSurface-concat-2"; "(concat ?a blankSurface)" => "?a"),
        rewrite!("blank-alpha"; "(apply_alpha ?p blankSurface)" => "blankSurface"),
        rewrite!("merge-blankSurface";  
            "(merge 
                ?layer blankSurface 
                (merge_params_with_state 
                    ?merge_params ?state_ops))" => "?layer"
            if merge_params_is_only_src_over("?merge_params")
    )]);

    // Effectively NoOp Rules
    rules.extend([
        rewrite!("kill-noOp-alpha"; "(apply_alpha ([alpha:255]) ?src)" => "?src"),
        rewrite!("kill-noOp-merge";  
            "(merge ?dst ?src ?merge_params_with_state)" => "(concat ?dst ?src)"
            if merge_is_src_over_and_paint_is_opaque_and_no_state_and_no_bounds("?merge_params_with_state")
        ),
        rewrite!("kill-apply-filter-and-state";
            "(apply_filter_with_state 
                ?surface ?merge_params_with_state)" 
                => "(?surface)" 
                if merge_is_src_over_and_paint_is_opaque_and_no_state_and_no_bounds("?merge_params_with_state")
        ),
    ]);

    // Fold Rules
    rules.extend([
        rewrite!("fold-alpha";
            "(apply_alpha ?alpha_params ?src)" => {
                FoldAlpha {
                    alpha_params: "?alpha_params".parse().unwrap(),
                    src: "?src".parse().unwrap(),
                    folded_draw_command: "?draw_command".parse().unwrap(),
                    expr: "?draw_command".parse().unwrap()
                }
            }
        ),
        rewrite!("fold-clipRect";
            "(clipRect (clipRect ?surface ?innerClipRectParams) ?outerClipRectParams)"
             => {
                FoldClipRect {
                    inner_clip_rect_params: "?innerClipRectParams".parse().unwrap(),
                    outer_clip_rect_params: "?outerClipRectParams".parse().unwrap(),
                    folded_clip_rect_params: "?foldedClipRectParams".parse().unwrap(),
                    expr: "(clipRect ?surface ?foldedClipRectParams)".parse().unwrap(),
                }
            }
        ),
    ]);

    // VirtualOp Rules (pack and unpack 'merge' into VirtualOps)
    rules.extend([
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
         } if merge_params_is_only_src_over("?merge_params")),
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
            } if merge_params_is_only_src_over("?merge_params")
        ),
    ]);

    rules.extend(vec![
        rewrite!("apply-clipRect-directly";
            "(apply_filter_with_state
                ?layer
                (merge_params_with_state 
                    ?merge_params 
                    (clipRect ?state ?params)
                )
            )" <=> 
            "(apply_filter_with_state
                (clipRect ?layer ?params)
                (merge_params_with_state
                    ?merge_params
                    ?state
                )
            )"
            if merge_params_is_only_src_over_and_no_bounds("?merge_params")
        ),
        rewrite!("apply-concat44-directly";
            "(apply_filter_with_state
                ?layer
                (merge_params_with_state 
                    ?merge_params 
                    (concat44 ?state ?params)
                )
            )" <=> 
            "(apply_filter_with_state
                (concat44 ?layer ?params)
                (merge_params_with_state
                    ?merge_params
                    ?state
                )
            )"
            if merge_params_is_only_src_over_and_no_bounds("?merge_params")
        ),
        rewrite!("apply-matrixOp-directly";
            "(apply_filter_with_state
                ?layer
                (merge_params_with_state 
                    ?merge_params 
                    (matrixOp ?state ?params)
                )
            )" <=> 
            "(apply_filter_with_state
                (matrixOp ?layer ?params)
                (merge_params_with_state
                    ?merge_params
                    ?state
                )
            )"
            if merge_params_is_only_src_over_and_no_bounds("?merge_params")
        ),
    ].concat());

    rules.extend(vec![
        rewrite!("src-over";
            "(merge ?dst ?src (merge_params_with_state ?merge_params ?state_ops))" <=>
            "(srcOver 
                ?dst 
                (apply_filter_with_state 
                    ?src 
                    (merge_params_with_state ?merge_params ?state_ops)))"
         if merge_params_is_src_over("?merge_params")),
        rewrite!("seperate-filter-and-state";
            "(apply_filter_with_state ?layer 
                (merge_params_with_state ?merge_params ?state_ops))"
            <=>
            "(apply_filter_with_state 
                (apply_filter_with_state 
                    ?layer 
                    (merge_params_with_state ?merge_params blankState))
                (merge_params_with_state
                    [MergeParams::index:-1,paint:[Paint::color:[Color::a:255,r:0,g:0,b:0],blend_mode:SrcOver,has_filters:false],has_backdrop:false,has_bounds:false,bounds:[rect:l:0,t:0,r:0,b:0]]
                    ?state_ops))"),
    ].concat());


    // SrcOver Rules
    rules.extend(vec![
        rewrite!("rearrange-srcOver"; 
            "(srcOver ?A (srcOver ?B ?C))" <=> "(srcOver (srcOver ?A ?B) ?C)"),
        rewrite!("concatIsSrcOver";
            "(concat ?A ?B)" <=> "(srcOver ?A ?B)"),
    ].concat());

    // Combine common param rules.
    rules.extend(vec![
        rewrite!("extract-common-clip"; 
            "(srcOver (clipRect ?A ?params) (clipRect ?B ?params))" <=> "(clipRect (srcOver ?A ?B) ?params)"),
        rewrite!("extract-common-m44"; 
            "(srcOver (concat44 ?A ?params) (concat44 ?B ?params))" <=> "(concat44 (srcOver ?A ?B) ?params)"),
        rewrite!("extract-common-matrixOp"; 
            "(srcOver (matrixOp ?A ?params) (matrixOp ?B ?params))" <=> "(matrixOp (srcOver ?A ?B) ?params)"),
    ].concat());

    // Alpha-StateOp Commutativity Rules 
    rules.extend(vec![
        rewrite!("alpha-m44"; 
            "(apply_alpha ?a (concat44 ?layer ?params))" <=> "(concat44 (apply_alpha ?a ?layer) ?params)"),
        rewrite!("alpha-clipRect"; 
            "(apply_alpha ?a (clipRect ?layer ?params))" <=> "(clipRect (apply_alpha ?a ?layer) ?params)"),
        rewrite!("alpha-matrixOp"; 
            "(apply_alpha ?a (matrixOp ?layer ?params))" <=> "(matrixOp (apply_alpha ?a ?layer) ?params)"),
    ].concat());

    // ApplyState Rules
    rules.extend(vec![
        rewrite!("apply-clipRect";
                 "(apply_state ?surface (clipRect ?state ?params))" <=> "(apply_state (clipRect ?surface ?params) ?state)"),
        rewrite!("apply-concat44";
                 "(apply_state ?surface (concat44 ?state ?params))" <=> "(apply_state (concat44 ?surface ?params) ?state)"),
        rewrite!("apply-matrixOp";
                 "(apply_state ?surface (matrixOp ?state ?params))" <=> "(apply_state (matrixOp ?surface ?params) ?state)"),
        rewrite!("kill-applyState";
                 "(apply_state ?surface blankState)" <=> "?surface"),
    ].concat());
    rules
}

fn merge_params_is_only_src_over(var: &'static str) -> impl Fn(&mut EGraph<SkiLang, ()>, Id, &Subst) -> bool {
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

fn merge_params_is_src_over(var: &'static str) -> impl Fn(&mut EGraph<SkiLang, ()>, Id, &Subst) -> bool {
    let var: Var = var.parse().unwrap();
    move |egraph, _, subst| {
        let merge_params_expr = egraph.id_to_expr(subst[var]);
        let root = merge_params_expr.as_ref().last().unwrap();
        let merge_params = match root {
            SkiLang::MergeParams(merge_params) => merge_params,
            _ => panic!("first id of merge_params_with_state is not merge_params")
        };
        merge_params.paint.blend_mode == SkiLangBlendMode::SrcOver 
    }
}

fn merge_params_is_only_src_over_and_no_bounds(var: &'static str) -> impl Fn(&mut EGraph<SkiLang, ()>, Id, &Subst) -> bool {
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
        && !merge_params.has_bounds
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
        let alpha_params_expr = egraph.id_to_expr(subst[self.alpha_params]);
        let root = alpha_params_expr.as_ref().last().unwrap();
        let alpha_params = match root {
            SkiLang::ApplyAlphaParams(alpha_params) => alpha_params,
            _ => panic!("Not ApplyAlpha")
        };
        let mut draw_command = None;
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

struct FoldClipRect {
    inner_clip_rect_params: Var,
    outer_clip_rect_params: Var,
    folded_clip_rect_params: Var,
    expr: Pattern<SkiLang>,
}

fn bounds_intersection(bounds1: &SkiLangRect, bounds2: &SkiLangRect) -> SkiLangRect {
    SkiLangRect {
        l: bounds1.l.max(bounds2.l),
        t: bounds1.t.max(bounds2.t),
        r: bounds1.r.min(bounds2.r),
        b: bounds1.b.min(bounds2.b),
    }
}

impl Applier<SkiLang, ()> for FoldClipRect {
    fn apply_one(
        &self,
        egraph: &mut EGraph<SkiLang, ()>,
        matched_id: Id,
        subst: &Subst,
        searcher_pattern: Option<&PatternAst<SkiLang>>,
        rule_name: Symbol,
    ) -> Vec<Id> {
        let inner_params_expr = &egraph.id_to_expr(subst[self.inner_clip_rect_params]);
        let root = &inner_params_expr.as_ref().last().unwrap();
        let inner_params = match root {
            SkiLang::ClipRectParams(value) => value,
            _ => panic!("This is not a ClipRectParams"),
        };
        let outer_params_expr = &egraph.id_to_expr(subst[self.outer_clip_rect_params]);
        let root = &outer_params_expr.as_ref().last().unwrap(); 
        let outer_params = match root {
            SkiLang::ClipRectParams(value) => value,
            _ => panic!("This is not a ClipRectParams"),
        };

        if inner_params.is_anti_aliased != outer_params.is_anti_aliased {
            return vec![];
        }

        if inner_params.clip_rect_mode != SkiLangClipRectMode::Intersect
            || outer_params.clip_rect_mode != SkiLangClipRectMode::Intersect {
            return vec![];
        }

        let merged_params = SkiLang::ClipRectParams(SkiLangClipRectParams {
            clip_rect_mode: inner_params.clip_rect_mode,
            is_anti_aliased: inner_params.is_anti_aliased,
            bounds: bounds_intersection(&inner_params.bounds, &outer_params.bounds),
        });

        let mut subst = subst.clone();
        let mut expr = RecExpr::default();
        expr.add(merged_params);
        subst.insert(self.folded_clip_rect_params, egraph.add_expr(&expr));
        self.expr
            .apply_one(egraph, matched_id, &subst, searcher_pattern, rule_name)
    }
}

struct ApplyStateOnLayer {
    layer: Var,
    state_ops: Var,
    layer_with_state: Var,
    expr: Pattern<SkiLang>,
}

impl Applier<SkiLang, ()> for ApplyStateOnLayer {
    fn apply_one(
        &self,
        egraph: &mut EGraph<SkiLang, ()>,
        matched_id: Id,
        subst: &Subst,
        searcher_pattern: Option<&PatternAst<SkiLang>>,
        rule_name: Symbol,
    ) -> Vec<Id> {
        let state_ops_expr = &egraph.id_to_expr(subst[self.state_ops]).pretty(0);
        let layer_expr = &egraph.id_to_expr(subst[self.layer]).pretty(0);
        let layer_with_state: RecExpr<SkiLang> = state_ops_expr.replace("blankState", &layer_expr).parse().unwrap();

        let mut subst = subst.clone();
        subst.insert(self.layer_with_state, egraph.add_expr(&layer_with_state));
        self.expr
            .apply_one(egraph, matched_id, &subst, searcher_pattern, rule_name)
    }
}
