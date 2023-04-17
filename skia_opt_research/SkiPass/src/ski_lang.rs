use egg::*;
use parse_display::{Display, FromStr};

use crate::protos::{
    Bounds
};
use crate::ski_lang_converters::{
    bounds_proto_to_expr,
    bounds_proto_to_rect_expr,
    unpack_rect_to_bounds,
    unpack_float
};

#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord, Display, FromStr)]
#[display("[rect:l:{l},t:{t},r:{r},b:{b}]")]
pub struct SkiLangRect {
	pub l: ordered_float::NotNan<f64>,
	pub t: ordered_float::NotNan<f64>,
	pub r: ordered_float::NotNan<f64>,
	pub b: ordered_float::NotNan<f64>,
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord, Display, FromStr)]
#[display("[m44:{m00},{m01},{m02},{m03},{m04},{m05},{m06},{m07},{m08},{m09},{m10},{m11},{m12},{m13},{m14},{m15}]")]
pub struct SkiLangM44 {
	m00: ordered_float::NotNan<f64>,
	m01: ordered_float::NotNan<f64>,
	m02: ordered_float::NotNan<f64>,
	m03: ordered_float::NotNan<f64>,
	m04: ordered_float::NotNan<f64>,
	m05: ordered_float::NotNan<f64>,
	m06: ordered_float::NotNan<f64>,
	m07: ordered_float::NotNan<f64>,
	m08: ordered_float::NotNan<f64>,
	m09: ordered_float::NotNan<f64>,
	m10: ordered_float::NotNan<f64>,
	m11: ordered_float::NotNan<f64>,
	m12: ordered_float::NotNan<f64>,
	m13: ordered_float::NotNan<f64>,
	m14: ordered_float::NotNan<f64>,
	m15: ordered_float::NotNan<f64>,
}

impl SkiLangM44 {
	pub fn fromVec(v: Vec<ordered_float::NotNan<f64>>) -> SkiLangM44 {
		SkiLangM44 {
			m00: v[0],
			m01: v[1],
			m02: v[2],
			m03: v[3],
			m04: v[4],
			m05: v[5],
			m06: v[6],
			m07: v[7],
			m08: v[8],
			m09: v[9],
			m10: v[10],
			m11: v[11],
			m12: v[12],
			m13: v[13],
			m14: v[14],
			m15: v[15],
		}
	}

	pub fn toVec(&self) -> Vec<f64> {
		vec![
			*self.m00,
			*self.m01,
			*self.m02,
			*self.m03,
			*self.m04,
			*self.m05,
			*self.m06,
			*self.m07,
			*self.m08,
			*self.m09,
			*self.m10,
			*self.m11,
			*self.m12,
			*self.m13,
			*self.m14,
			*self.m15,
		]
	}
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord, Display, FromStr)]
pub enum SkiLangClipRectMode {
	Diff,
	Intersect
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord, Display, FromStr)]
#[display("[ClipRectParams::mode:{clipRectMode},bounds:{bounds},antiAlias:{doAntiAlias}]")]
pub struct SkiLangClipRectParams {
	pub clipRectMode: SkiLangClipRectMode,
	pub bounds: SkiLangRect,
	pub doAntiAlias: bool,
}

define_language! {
    pub enum SkiLang {
        // NOTE: The order of Num and Float matters!
        // First all Nums are parsed, and then Floats. So if
        // you want to force a number to be parsed as a float,
        // make sure to add a . (1.0 instead of 1)
		M44(SkiLangM44),
		Rect(SkiLangRect),
		ClipRectParams(SkiLangClipRectParams),
        Num(i32),
        Float(ordered_float::NotNan<f64>),
        Bool(bool),
        "noOp" = NoOp,
        "blankSurface" = BlankSurface,
        "blankState" = BlankState, 

        // ------ Skia translation operations start ------//
        /* 
            1. Concat: Sequentially apply draw commands.
            (concat layer1 layer2) ->
                <layer1 draw commands>
                <layer2 draw commands>

            2. Merge: Directly corresponds to SaveLayer.
            (merge layer1 layer2 mergeParams) -> 
                <layer1 draw commands>
                save (IF mergeParams has state)
                    <state commands>
                    saveLayer(mergeParams)
                        <layer2 drawCommands>
                    restore()
                restore()
         

            3. DrawCommand: Apply the drawCommand at index in reference SKP.
                (drawCommand index paint)
                    If paint has an alpha, modify the alpha 
                    before applying the drawCommand.
        */
        "concat" = Concat([Id; 2]),
        "merge" = Merge([Id; 3]),
        "drawCommand" = DrawCommand([Id; 2]),
        // ------ Skia translation operations end ------//

        // ------ Virtual Ops (with Skia equivalent) --- //
        "matrixOp" = MatrixOp([Id; 2]),
        "concat44" = Concat44([Id; 2]),
        "clipRect" = ClipRect([Id; 2]),

        // ------ Virtual Ops (with NO Skia equivalent) --- //
        // (alpha layer value) -> apply transparency of value on layer
        "alpha" = Alpha([Id; 2]), // alphaChannel, layer
        // (srcOver dst src)
        "srcOver" = SrcOver([Id; 2]),

        "someFilterAndState" = SomeFilterAndState([Id; 2]),

        // (bound exists? rect)
        "bounds" = Bounds([Id; 2]),

        /* paint(
            color filter blender imageFilter colorFilter
            pathEffect maskFilter shader
        )
        */
        "color" = Color([Id; 4]),
        "blender" = Blender([Id; 1]),
        "imageFilter" = ImageFilter([Id; 1]),
        "colorFilter" = ColorFilter([Id; 1]),
        "pathEffect" = PathEffect([Id; 1]),
        "maskFilter" = MaskFilter([Id; 1]),
        "shader" = Shader([Id; 1]),
		"paint" = Paint([Id; 7]),

        // (backdrop ?exists)
        "backdrop" = Backdrop([Id; 1]),
        // (mergeParams index paint backdrop bounds state)
        "mergeParams" = MergeParams([Id; 5]),
        // (matrixOpParams index)
        "matrixOpParams" = MatrixOpParams([Id; 1]),

        "blendMode_srcOver" = BlendMode_SrcOver,
        "blendMode_src" = BlendMode_Src,
        "blendMode_unknown" = BlendMode_Unknown,
    }
}

pub fn make_rules() -> Vec<Rewrite<SkiLang, ()>> {
    // Trivial Rules, related to blank and identity.
    let mut rules = vec![
        rewrite!("kill-blankSurface-clip"; "(clipRect blankSurface ?p)" => "blankSurface"),
        rewrite!("kill-blankSurface-mOp"; "(matrixOp blankSurface ?p)" => "blankSurface"),
        rewrite!("kill-blankSurface-concat44"; "(concat44 blankSurface ?p)" => "blankSurface"),
        rewrite!("kill-blankSurface-concat-1"; "(concat blankSurface ?a)" => "?a"),
        rewrite!("kill-blankSurface-concat-2"; "(concat ?a blankSurface)" => "?a"),
        rewrite!("kill-noOp-alpha"; "(alpha 255 ?src)" => "?src"),
        rewrite!("kill-merge-blankSurface"; 
                 "(merge 
                        ?layer 
                        blankSurface
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
                            (bounds false ?bounds)
                            blankState
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
                            (bounds false ?bounds)
                            blankState
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
                                ?color
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
                                    ?color
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
                "(clipRect (clipRect ?layer ?innerClipRectParams) ?outerClipRectParams)" =>  {
                FoldClipRect {
                    innerClipRectParams: "?innerClipRectParams".parse().unwrap(),
                    outerClipRectParams: "?outerClipRectParams".parse().unwrap(),
                    foldedClipRectParams: "?foldedClipRectParams".parse().unwrap(),
                    expr: "(clipRect ?layer ?foldedClipRectParams)".parse().unwrap(),
                }
            }


			),
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
                ))"),
        ]);

        rules.extend(vec![
            rewrite!("popFilterOntoLayer";
                "(someFilterAndState
                    ?layer
                    (mergeParams
                        ?mergeIndex
                        ?paint
                        ?backdrop
                        ?bounds
                        ?stateParams
                    )
                )" <=> 
                "(someFilterAndState
                    (someFilterAndState
                        ?layer
                        (mergeParams
                            ?mergeIndex
                            ?paint
                            ?backdrop
                            ?bounds
                            blankState
                        )
                    )
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
                        ?stateParams
                    )
                )"),

            rewrite!("popClipRectOntoLayer";
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
                        (clipRect ?inner ?params)
                     )
                )" <=> 
                "(someFilterAndState
                    (clipRect ?layer ?params)
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
                        ?inner
                     )
                )"),

            rewrite!("popConcatM44OntoLayer";
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
                        (concat44 ?inner ?params)
                     )
                )" <=> 
                "(someFilterAndState
                    (concat44 ?layer ?params)
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
                        ?inner
                     )
                )"),
            rewrite!("popMatrixOpOntoLayer";
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
                        (matrixOp ?inner ?params)
                     )
                )" <=> 
                "(someFilterAndState
                    (matrixOp ?layer ?params)
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
                        ?inner
                     )
                )"),
        ].concat());


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

        // Alpha-Matrix Bidirectional rules
        rules.extend(vec![
            rewrite!("alpha-m44"; 
                     "(alpha ?a (concat44 ?layer ?params))" <=> "(concat44 (alpha ?a ?layer) ?params)"),

            rewrite!("alpha-clipRect"; 
                     "(alpha ?a (clipRect ?layer ?params))" <=> "(clipRect (alpha ?a ?layer) ?params)"),

            rewrite!("alpha-matrixOp"; 
                     "(alpha ?a (matrixOp ?layer ?params))" <=> "(matrixOp (alpha ?a ?layer) ?params)"),
        ].concat());

        rules
}

// This CostFn exists to prevent internal SkiLang functions (such as alpha) to never be extracted.
pub struct SkiLangCostFn;
impl CostFunction<SkiLang> for SkiLangCostFn {
    // Number of virtual ops, number of layers, number of commands
    type Cost=(i32, i32, i32);
    fn cost<C>(&mut self, enode: &SkiLang, mut costs: C) -> Self::Cost
        where
            C: FnMut(Id) -> Self::Cost
    {
        let op_cost = match enode {
            SkiLang::Alpha(_ids) => (1, 0, 1),
            SkiLang::SomeFilterAndState(_ids) => (1, 0, 1),
            SkiLang::SrcOver(_ids) => (1, 0, 1),
            // TODO: We want a cost that is (number of layers, cost) and that depends on subtree size.
            SkiLang::Merge(_ids) => (0, 1, 1), 
            _ => (0, 0, 1)
        };
        enode.fold(op_cost, |sum, id| (
                sum.0 + costs(id).0, 
                sum.1 + costs(id).1,
                sum.2 + costs(id).2
        ))
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
        let mut merged_alpha_value = (layer_alpha * draw_alpha) / 255;
        let merged_alpha = SkiLang::Num(merged_alpha_value);

        let mut subst = subst.clone();
        subst.insert(self.merged_alpha, egraph.add(merged_alpha));
        self.expr.apply_one(egraph, matched_id, &subst, searcher_pattern, rule_name)
    }
}

struct FoldClipRect {
    innerClipRectParams: Var,
    outerClipRectParams: Var,
    foldedClipRectParams: Var,
    expr: Pattern<SkiLang>
}

fn bounds_intersection(bounds1: &SkiLangRect, bounds2: &SkiLangRect) -> SkiLangRect {
    SkiLangRect {
        l: bounds1.l.max(bounds2.l),
        t: bounds1.t.max(bounds2.t),
        r: bounds1.r.min(bounds2.r),
        b: bounds1.b.min(bounds2.b) 
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
		let innerClipRectParamExpr = &egraph.id_to_expr(subst[self.innerClipRectParams]);
        let innerClipRectParams = match &innerClipRectParamExpr[0.into()] {
			SkiLang::ClipRectParams(value) => value,
			_ => panic!("This is not a ClipRectParams")
		};

		let outerClipRectParamExpr = &egraph.id_to_expr(subst[self.outerClipRectParams]);
        let outerClipRectParams = match &outerClipRectParamExpr[0.into()] {
			SkiLang::ClipRectParams(value) => value,
			_ => panic!("This is not a ClipRectParams")
		};

		if innerClipRectParams.doAntiAlias != outerClipRectParams.doAntiAlias {
			return vec![];
		}

		if innerClipRectParams.clipRectMode != SkiLangClipRectMode::Intersect ||
			outerClipRectParams.clipRectMode != SkiLangClipRectMode::Intersect {
			return vec![];
		}

		let mergedClipRectParams = SkiLang::ClipRectParams(SkiLangClipRectParams {
			clipRectMode: innerClipRectParams.clipRectMode,
			doAntiAlias: innerClipRectParams.doAntiAlias,
			bounds: bounds_intersection(&innerClipRectParams.bounds, &outerClipRectParams.bounds)
		});

        let mut subst = subst.clone();
		let mut expr = RecExpr::default();
		expr.add(mergedClipRectParams);
        subst.insert(self.foldedClipRectParams, egraph.add_expr(&expr));
        self.expr.apply_one(egraph, matched_id, &subst, searcher_pattern, rule_name)
    }
}
