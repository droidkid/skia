use egg::*;

use crate::protos::{
    Bounds
};
use crate::ski_lang_converters::{
    bounds_proto_to_expr,
    bounds_proto_to_rect_expr,
    unpack_rect_to_bounds,
    unpack_float
    
};

define_language! {
    pub enum SkiLang {
        // NOTE: The order of Num and Float matters!
        // First all Nums are parsed, and then Floats. So if
        // you want to force a number to be parsed as a float,
        // make sure to add a . (1.0 instead of 1)
        Num(i32),
        Float(ordered_float::NotNan<f64>),
        // TODO: Rename to Bool
        Exists(bool),
        "noOp" = NoOp,
        // TODO: Rename to BlankSurface
        "blank" = Blank,
        // blankState
        "blankState" = BlankState, 
        // ------ BLEND_MODE SYMBOLS BEGIN --------//
        "blendMode_srcOver" = BlendMode_SrcOver,
        "blendMode_src" = BlendMode_Src,
        "blendMode_unknown" = BlendMode_Unknown,
        // -------BLEND MODES SYMBOLS END --------//
        // ------ CLIP_OP SYMBOLS BEGIN --------//
        "clipOp_diff" = ClipOp_Diff,
        "clipOp_intersect" = ClipOp_Intersect,
        // -------CLIP OP SYMBOLS END --------//
        // drawCommand(index, paint)
        "drawCommand" = DrawCommand([Id; 2]),
        // TODO: Split matrix and clip ops. Right now clips are a 'matrixOp'
        // matrixOp(layer, matrixOpParams) -> return layer after applying transform on layer 
        "matrixOp" = MatrixOp([Id; 2]),
        "concat44" = Concat44([Id; 2]),
        // clipRect(layer, clipRectParams) -> return layer after applying clip on layer 
        "clipRect" = ClipRect([Id; 2]),
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
        // MergeParams([index, paint, backdrop, bounds, state])
        "mergeParams" = MergeParams([Id; 5]),
        // ClipParams
        // ClipRectParams([bounds, clipOp, doAntiAlias])
        "clipRectParams" = ClipRectParams([Id; 3]),
        // MatrixOpParams([index])  - eventually add other matrix stuff.
        "matrixOpParams" = MatrixOpParams([Id; 1]),
        // 4x4 matrix
        // TODO: Are we going to store 16 items?
        "m44" = M44([Id; 16]),
        "srcOver" = SrcOver([Id; 2]),
        "someFilterAndState" = SomeFilterAndState([Id; 2]),
    }
}

// TODO: Decide on a convention in writing rules 
// Both of the below are equivalent:
//    ?pathEffect
//    (pathEffect ?pathEffectExists)
// Right now the choice is arbitrary, there's also a choice of using Null.
pub fn make_rules() -> Vec<Rewrite<SkiLang, ()>> {
    // Trivial Rules, related to blank and identity.
    let mut rules = vec![
        rewrite!("kill-blank-clip"; "(clipRect blank ?p)" => "blank"),
        rewrite!("kill-blank-mOp"; "(matrixOp blank ?p)" => "blank"),
        rewrite!("kill-blank-concat44"; "(concat44 blank ?p)" => "blank"),
        rewrite!("kill-blank-concat-1"; "(concat blank ?a)" => "?a"),
        rewrite!("kill-blank-concat-2"; "(concat ?a blank)" => "?a"),
        rewrite!("kill-noOp-alpha"; "(alpha 255 ?src)" => "?src"),
        rewrite!("kill-merge-blank"; 
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
                            ?bounds
                            ?stateVars
                        )
                   )" 
                => "?layer"),
    ];


    // Merge related rules.
    rules.extend( vec![
        // Kill NoOp - SaveLayer(nullptr, nullptr).
        rewrite!("kill-noOp-merge";
                 "(merge 
                        ?dst 
                        ?src
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
                            (bounds false ?b)
                            blankState
                        )
                    )" => "(concat ?dst ?src)"),

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
                                (color 255 ?r ?g ?b)
                                (blender blendMode_srcOver)
                                (imageFilter false)
                                (colorFilter false)
                                (pathEffect false)
                                (maskFilter false)
                                (shader false)
                            )
                            (backdrop false)
                            ?bounds
                            ?stateVars
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
                            ?bounds
                            ?stateVars
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

    ]);

    // Virtual Op Bidirectional Rules.
    rules.extend(vec![
        rewrite!("alphaVirtualOp"; 
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
                            ?bounds
                            ?stateVars
                        )
                    )" 
                 <=> 
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
                            ?bounds
                            ?stateVars
                        )
                    )"),
        rewrite!("srcOverVirtualOp"; 
                 "(merge 
                        ?layerA
                        ?layerB
                        (mergeParams
                            ?mergeIndex
                            (paint 
                                (color ?A 0 0 0) 
                                (blender blendMode_srcOver)
                                ?imageFilter
                                ?colorFilter
                                ?pathEffect
                                ?maskFilter
                                ?shader
                            )
                            ?backdrop
                            ?bounds
                            ?stateVars
                        )
                   )" 
                   <=> "(srcOver 
                        ?layerA 
                        (someFilterAndState 
                            ?layerB
                            (mergeParams
                                ?mergeIndex
                                (paint 
                                    (color ?A 0 0 0) 
                                    (blender blendMode_srcOver)
                                    ?imageFilter
                                    ?colorFilter
                                    ?pathEffect
                                    ?maskFilter
                                    ?shader
                                )
                                ?backdrop
                                ?bounds
                                ?stateVars
                            )
                        ) 
                    )"),

    ].concat());

    // Folding VirtualOps
    rules.extend(vec![
        rewrite!("fold-draw"; 
                        "(alpha ?layer_alpha
                                (drawCommand 
                                    ?x 
                                    (paint
                                        (color ?draw_alpha ?r ?g ?b) 
                                        (blender blendMode_srcOver)
                                        (imageFilter false)
                                        (colorFilter false)
                                        (pathEffect false)
                                        (maskFilter false)
                                        (shader false)
                                    )
                                )
                            )
                        " => {
                            FoldAlpha {
                                layer_alpha: "?layer_alpha".parse().unwrap(),
                                draw_alpha: "?draw_alpha".parse().unwrap(),
                                merged_alpha: "?merged_alpha".parse().unwrap(),
                                expr: "(drawCommand 
                                    ?x 
                                    (paint
                                        (color ?merged_alpha ?r ?g ?b)
                                        (blender blendMode_srcOver)
                                        (imageFilter false)
                                        (colorFilter false)
                                        (pathEffect false)
                                        (maskFilter false)
                                        (shader false)
                                    )
                                )".parse().unwrap(),
                            }
                        }
                    ),
        rewrite!("clipRect-intersect"; 
                "(clipRect 
                    (clipRect ?layer (clipRectParams ?bounds1 clipOp_intersect ?doAntiAlias)) 
                    (clipRectParams ?bounds2 clipOp_intersect ?doAntiAlias)
                )" =>  {
                FoldClipRect {
                    bounds1: "?bounds1".parse().unwrap(),
                    bounds2: "?bounds2".parse().unwrap(),
                    merged_bounds: "?bounds".parse().unwrap(),
                    expr: "(clipRect ?layer (clipRectParams ?bounds clipOp_intersect ?doAntiAlias))".parse().unwrap(),
                }
            }),
        ]);


        // Packing and Unpacking Filter and State.
        // This is not a bidirectional rule as information is lost when going one way.
        rules.extend(vec![
            rewrite!("killSomeFilterAndState";
                "(someFilterAndState
                    ?layer
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
                        (bounds false noOp)
                        blankState
                     )
                )" => "?layer"),

            rewrite!("recreateSomeFilterAndState";
                "?layer" =>
                "(someFilterAndState
                    ?layer
                    (mergeParams
                        -1
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
                        (bounds false noOp)
                        blankState
                ))")
        ]);


        rules.extend(vec![
            rewrite!("rearrange-srcOver"; 
                    "(srcOver ?A (srcOver ?B ?C))" <=> "(srcOver (srcOver ?A ?B) ?C)"),
            rewrite!("concatIsSrcOver";
                     "(concat ?A ?B)" <=> "(srcOver ?A ?B)"),
        ].concat());



        // Bidirectional rules to extract common clipRect, m44 and matrixOp operations over srcOver.
        rules.extend(vec![
            rewrite!("extract-common-clip"; 
                 "(srcOver (clipRect ?A ?params) (clipRect ?B ?params))" <=> "(clipRect (srcOver ?A ?B) ?params)"),

            rewrite!("extract-common-m44"; 
                 "(srcOver (concat44 ?A ?params) (concat44 ?B ?params))" <=> "(concat44 (srcOver ?A ?B) ?params)"),

            rewrite!("extract-common-matrixOp"; 
                 "(srcOver (matrixOp ?A ?params) (matrixOp ?B ?params))" <=> "(matrixOp (srcOver ?A ?B) ?params)")
        ].concat());

        rules
}

// This CostFn exists to prevent internal SkiLang functions (such as alpha) to never be extracted.
pub struct SkiLangCostFn;
impl CostFunction<SkiLang> for SkiLangCostFn {
    type Cost=f64;
    fn cost<C>(&mut self, enode: &SkiLang, mut costs: C) -> Self::Cost
        where
            C: FnMut(Id) -> Self::Cost
    {
        let op_cost = match enode {
            SkiLang::Alpha(_ids) => 100000000.0,
            SkiLang::SomeFilterAndState(_ids) => 100000000.0,
            SkiLang::SrcOver(_ids) => 100000000.0,
            SkiLang::Merge(_ids) => 1.0,
            _ => 0.0
        };
        enode.fold(op_cost, |sum, id| sum + costs(id))
    }
}

struct FoldAlpha {
    layer_alpha: Var,
    draw_alpha: Var,
    merged_alpha: Var,
    expr: Pattern<SkiLang>
}

impl Applier<SkiLang, ()> for FoldAlpha {
    fn apply_one(
        &self, 
        egraph: &mut EGraph<SkiLang, ()>,
        matched_id: Id, 
        subst: &Subst, 
        searcher_pattern: Option<&PatternAst<SkiLang>>, 
        rule_name: Symbol) -> Vec<Id> {
        let matched_expr: RecExpr<SkiLang> = egraph.id_to_expr(matched_id);
        let layer_alpha_id = subst[self.layer_alpha];
        let draw_alpha_id = subst[self.draw_alpha];
        let layer_alpha :i32 = match egraph.id_to_expr(layer_alpha_id)[0.into()] {
            SkiLang::Num(val) => val,
            _ => panic!("Not a valid alpha value")
        };
        let draw_alpha :i32 = match egraph.id_to_expr(draw_alpha_id)[0.into()] {
            SkiLang::Num(val) => val,
            _ => panic!("Not a valid alpha value")
        };
        let merged_alpha = SkiLang::Num((layer_alpha * draw_alpha) / 255);
        let mut subst = subst.clone();
        subst.insert(self.merged_alpha, egraph.add(merged_alpha));
        self.expr.apply_one(egraph, matched_id, &subst, searcher_pattern, rule_name)
    }
}

struct FoldClipRect {
    bounds1: Var,
    bounds2: Var,
    merged_bounds: Var,
    expr: Pattern<SkiLang>
}

fn bounds_intersection(bounds1: Bounds, bounds2: Bounds) -> Bounds {
    Bounds {
        left: bounds1.left.max(bounds2.left),
        top: bounds1.top.max(bounds2.top),
        right: bounds1.right.min(bounds2.right),
        bottom: bounds1.bottom.min(bounds2.bottom) 
    }
}

impl Applier<SkiLang, ()> for FoldClipRect {

    fn apply_one(
        &self, 
        egraph: &mut EGraph<SkiLang, ()>,
        matched_id: Id, 
        subst: &Subst, 
        searcher_pattern: Option<&PatternAst<SkiLang>>, 
        rule_name: Symbol) -> Vec<Id> {

        let mut matched_expr: RecExpr<SkiLang> = egraph.id_to_expr(matched_id);
        let bounds1 = subst[self.bounds1];
        let bounds2 = subst[self.bounds2];
    
        let bounds1_proto = unpack_rect_to_bounds(&egraph.id_to_expr(bounds1), 4.into());
        let bounds2_proto = unpack_rect_to_bounds(&egraph.id_to_expr(bounds2), 4.into());

        let bounds_proto = bounds_intersection(bounds1_proto, bounds2_proto); 
        let mut bounds_expr = RecExpr::default();
        let bounds = bounds_proto_to_rect_expr(&mut bounds_expr, &Some(bounds_proto));
        
        let mut subst = subst.clone();
        subst.insert(self.merged_bounds, egraph.add_expr(&bounds_expr));
        self.expr.apply_one(egraph, matched_id, &subst, searcher_pattern, rule_name)
    }
}
