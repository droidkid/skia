use egg::*;

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
    vec![
        rewrite!("remove-noOp-concat-1"; "(concat blank ?a)" => "?a"),
        rewrite!("remove-noOp-concat-2"; "(concat ?a blank)" => "?a"),
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


        rewrite!("apply-alphaVOp"; 
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
                 => 
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
        rewrite!("pack-alphaVOp"; 
                 "(merge 
                        ?dst 
                        (alpha ?A ?src) 
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
                 "(merge 
                        ?dst 
                        ?src 
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
                    )"),
        rewrite!("remove-merge-blank"; 
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


        rewrite!("remove-noOp-alpha"; "(alpha 255 ?src)" => "?src"),
        rewrite!("apply-alpha-on-draw"; 
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

        rewrite!("apply-srcOver-VOp"; 
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
                   => "(srcOver 
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

        rewrite!("unpack-srcOver-VOp"; 
                   "(srcOver 
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
                 )" =>
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
                ),

        // The only reason we have these rules is we can't unpack them.
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

        // This is to mainly unpack into merge.
        rewrite!("unpack-someFilterAndState";
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
                ))"),

        rewrite!("rearrange-srcOver"; 
                 "(srcOver ?A (srcOver ?B ?C))" => "(srcOver (srcOver ?A ?B) ?C)"),

    ]
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
