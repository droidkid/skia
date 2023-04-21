use egg::*;
use parse_display::{Display, FromStr};
use crate::protos::{
    Bounds,
    BlendMode,
    SkPaint,
    SkColor,
    SkM44,
    sk_paint::ImageFilter,
    sk_paint::Blender
};

define_language! {
    pub enum SkiLang {
        "noOp" = NoOp,
        "blankSurface" = BlankSurface,
        "blankState" = BlankState,
        M44(SkiLangM44),
        Rect(SkiLangRect),
        ClipRectParams(SkiLangClipRectParams),
        MatrixOpParams(SkiLangMatrixOpParams),
        ApplyAlphaParams(SkiLangApplyAlphaParams),
        Paint(SkiLangPaint),
        DrawCommand(SkiLangDrawCommand),
        MergeParams(SkiLangMergeParams),
        "merge_params_with_state" = MergeParamsWithState([Id; 2]),
        "apply_filter_with_state" = SomeFilterAndState([Id; 2]),
        "concat" = Concat([Id; 2]),
        "merge" = Merge([Id; 3]),
        "matrixOp" = MatrixOp([Id; 2]),
        "concat44" = Concat44([Id; 2]),
        "clipRect" = ClipRect([Id; 2]),
        "apply_alpha" = ApplyAlpha([Id; 2]), // alphaChannel, layer
        "srcOver" = SrcOver([Id; 2]),
    }
}


#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord, Display, FromStr)]
#[display("[rect:l:{l},t:{t},r:{r},b:{b}]")]
pub struct SkiLangRect {
    pub l: ordered_float::NotNan<f64>,
    pub t: ordered_float::NotNan<f64>,
    pub r: ordered_float::NotNan<f64>,
    pub b: ordered_float::NotNan<f64>,
}

impl SkiLangRect {
    pub fn from_bounds_proto(bounds: &Bounds) -> SkiLangRect {
        SkiLangRect {
            l: ordered_float::NotNan::new(bounds.left).unwrap(),
            t: ordered_float::NotNan::new(bounds.top).unwrap(),
            r: ordered_float::NotNan::new(bounds.right).unwrap(),
            b: ordered_float::NotNan::new(bounds.bottom).unwrap(),
        }
    }
    pub fn to_proto(&self) -> Bounds {
        Bounds {
            left: *self.l,
            right: *self.r,
            top: *self.t,
            bottom: *self.b
        }
    }

    pub fn empty() -> SkiLangRect {
        SkiLangRect {
            l: ordered_float::NotNan::new(0.0).unwrap(),
            t: ordered_float::NotNan::new(0.0).unwrap(),
            r: ordered_float::NotNan::new(0.0).unwrap(),
            b: ordered_float::NotNan::new(0.0).unwrap(),
        }
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord, Display, FromStr)]
#[display("[matrixOp::index:{index}]")]
pub struct SkiLangMatrixOpParams {
    pub index: i32
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
    pub fn from_vec(v: Vec<ordered_float::NotNan<f64>>) -> SkiLangM44 {
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

    pub fn as_vec(&self) -> Vec<f64> {
        vec![
            *self.m00, *self.m01, *self.m02, *self.m03, *self.m04, *self.m05, *self.m06, *self.m07,
            *self.m08, *self.m09, *self.m10, *self.m11, *self.m12, *self.m13, *self.m14, *self.m15,
        ]
    }
    pub fn from_skm44_proto(sk_m44: &SkM44) -> SkiLangM44 {
        let mat = vec![
            ordered_float::NotNan::new(sk_m44.m[0]).unwrap(),
            ordered_float::NotNan::new(sk_m44.m[1]).unwrap(),
            ordered_float::NotNan::new(sk_m44.m[2]).unwrap(),
            ordered_float::NotNan::new(sk_m44.m[3]).unwrap(),
            ordered_float::NotNan::new(sk_m44.m[4]).unwrap(),
            ordered_float::NotNan::new(sk_m44.m[5]).unwrap(),
            ordered_float::NotNan::new(sk_m44.m[6]).unwrap(),
            ordered_float::NotNan::new(sk_m44.m[7]).unwrap(),
            ordered_float::NotNan::new(sk_m44.m[8]).unwrap(),
            ordered_float::NotNan::new(sk_m44.m[9]).unwrap(),
            ordered_float::NotNan::new(sk_m44.m[10]).unwrap(),
            ordered_float::NotNan::new(sk_m44.m[11]).unwrap(),
            ordered_float::NotNan::new(sk_m44.m[12]).unwrap(),
            ordered_float::NotNan::new(sk_m44.m[13]).unwrap(),
            ordered_float::NotNan::new(sk_m44.m[14]).unwrap(),
            ordered_float::NotNan::new(sk_m44.m[15]).unwrap(),
        ];
        SkiLangM44::from_vec(mat)
    }
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord, Display, FromStr)]
pub enum SkiLangClipRectMode {
    Diff,
    Intersect,
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord, Display, FromStr)]
#[display("[ClipRectParams::mode:{clip_rect_mode},bounds:{bounds},antiAlias:{is_anti_aliased}]")]
pub struct SkiLangClipRectParams {
    pub clip_rect_mode: SkiLangClipRectMode,
    pub bounds: SkiLangRect,
    pub is_anti_aliased: bool,
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord, Display, FromStr)]
#[display("[DrawCommand::index:{index},paint:{paint}]")]
pub struct SkiLangDrawCommand {
    pub index: i32,
    pub paint: SkiLangPaint
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord, Display, FromStr)]
#[display("[Color::a:{a},r:{r},g:{g},b:{b}]")]
pub struct SkiLangColor {
    pub a: i32,
    pub r: i32,
    pub g: i32,
    pub b: i32
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord, Display, FromStr)]
pub enum SkiLangBlendMode {
    Src,
    SrcOver,
    Unknown
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord, Display, FromStr)]
#[display("[Paint::color:{color},blend_mode:{blend_mode},has_filters:{has_filters}]")]
pub struct SkiLangPaint {
    pub color : SkiLangColor,
    pub blend_mode: SkiLangBlendMode,
    pub has_filters: bool
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord, Display, FromStr)]
#[display("[MergeParams::index:{index},paint:{paint},has_backdrop:{has_backdrop},has_bounds:{has_bounds},bounds:{bounds}]")]
pub struct SkiLangMergeParams {
    pub index: i32,
    pub paint: SkiLangPaint,
    pub has_backdrop: bool,
    // TODO: wrap this in a Option<bounds> once you have this working.
    pub has_bounds: bool,
    pub bounds: SkiLangRect,
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord, Display, FromStr)]
#[display("[alpha:{alpha}]")]
pub struct SkiLangApplyAlphaParams {
    pub alpha: i32
}

impl SkiLangPaint {
    pub fn from_proto(sk_record_paint: &Option<SkPaint>) -> SkiLangPaint {
        let sk_record_paint = sk_record_paint.as_ref().unwrap();
        let color = match &sk_record_paint.color {
            Some(color) => {
                SkiLangColor {
                    a: color.alpha_u8,
                    r: color.red_u8,
                    g: color.green_u8,
                    b: color.red_u8
                }
            }
            None => {
                SkiLangColor {
                    a: 255,
                    r: 0,
                    g: 0, 
                    b: 0
                }
            }
        };

        let blend_mode = match &sk_record_paint.blender {
            Some(blender) => {
                if blender.blend_mode == BlendMode::SrcOver.into() {
                    SkiLangBlendMode::SrcOver
                } else if blender.blend_mode == BlendMode::Src.into() {
                    SkiLangBlendMode::Src
                } else {
                    SkiLangBlendMode::Unknown
                }
            },
            None => SkiLangBlendMode::SrcOver,
        };

        let mut has_filters = false;
        has_filters = has_filters || sk_record_paint.image_filter.is_some();
        has_filters = has_filters || sk_record_paint.color_filter.is_some();
        has_filters = has_filters || sk_record_paint.path_effect.is_some();
        has_filters = has_filters || sk_record_paint.mask_filter.is_some();
        has_filters = has_filters || sk_record_paint.shader.is_some();

        SkiLangPaint {
            color,
            blend_mode,
            has_filters
        }
    }
    pub fn to_proto(&self) -> SkPaint {
        let blend_mode = match self.blend_mode {
            SkiLangBlendMode::Src => BlendMode::Src.into(),
            SkiLangBlendMode::SrcOver => BlendMode::SrcOver.into(),
            SkiLangBlendMode::Unknown => BlendMode::Unknown.into(),
        };
        // TODO: Just have one field 'has_filters' in the proto too.
        let image_filter = if self.has_filters {
            Some(ImageFilter {} )
        } else {
            None
        };

        SkPaint {
            color: Some(SkColor{
                alpha_u8: self.color.a,
                red_u8: self.color.r,
                green_u8: self.color.g,
                blue_u8: self.color.b
            }),
            blender: Some(Blender {
                blend_mode
            }),
            image_filter,
            color_filter: None,
            path_effect: None,
            mask_filter: None,
            shader: None 
        }
    }
}

    // Trivial Rules, related to blank and identity.
    /*
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
    rules.extend(vec![
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
    rules.extend(
        vec![
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
                    )" <=> 
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
        ]
        .concat(),
    );

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

    rules.extend(
        vec![
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
        ]
        .concat(),
    );

    rules.extend(
        vec![
            rewrite!("rearrange-srcOver"; 
                    "(srcOver ?A (srcOver ?B ?C))" <=> "(srcOver (srcOver ?A ?B) ?C)"),
            rewrite!("concatIsSrcOver";
                     "(concat ?A ?B)" <=> "(srcOver ?A ?B)"),
        ]
        .concat(),
    );

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

        */
    // rules

// This CostFn exists to prevent internal SkiLang functions (such as alpha) to never be extracted.
pub struct SkiLangCostFn;
impl CostFunction<SkiLang> for SkiLangCostFn {
    // Number of virtual ops, number of layers, number of commands
    type Cost = (i32, i32, i32);
    fn cost<C>(&mut self, enode: &SkiLang, mut costs: C) -> Self::Cost
    where
        C: FnMut(Id) -> Self::Cost,
    {
        let op_cost = match enode {
            SkiLang::ApplyAlpha(_ids) => (1, 0, 1),
            SkiLang::SomeFilterAndState(_ids) => (1, 0, 1),
            SkiLang::SrcOver(_ids) => (1, 0, 1),
            // TODO: We want a cost that is (number of layers, cost) and that depends on subtree size.
            SkiLang::Merge(_ids) => (0, 1, 1),
            _ => (0, 0, 1),
        };
        enode.fold(op_cost, |sum, id| {
            (
                sum.0 + costs(id).0,
                sum.1 + costs(id).1,
                sum.2 + costs(id).2,
            )
        })
    }
}

/*
struct FoldAlpha {
    layer_alpha: Var,
    draw_alpha: Var,
    merged_alpha: Var,
    expr: Pattern<SkiLang>,
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
        let matched_expr: RecExpr<SkiLang> = egraph.id_to_expr(matched_id);
        let layer_alpha_id = subst[self.layer_alpha];
        let draw_alpha_id = subst[self.draw_alpha];
        let layer_alpha: i32 = match egraph.id_to_expr(layer_alpha_id)[0.into()] {
            SkiLang::Num(val) => val,
            _ => panic!("Not a valid alpha value"),
        };
        let draw_alpha: i32 = match egraph.id_to_expr(draw_alpha_id)[0.into()] {
            SkiLang::Num(val) => val,
            _ => panic!("Not a valid alpha value"),
        };
        let mut merged_alpha_value = (layer_alpha * draw_alpha) / 255;
        let merged_alpha = SkiLang::Num(merged_alpha_value);

        let mut subst = subst.clone();
        subst.insert(self.merged_alpha, egraph.add(merged_alpha));
        self.expr
            .apply_one(egraph, matched_id, &subst, searcher_pattern, rule_name)
    }
}

struct FoldClipRect {
    innerClipRectParams: Var,
    outerClipRectParams: Var,
    foldedClipRectParams: Var,
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
        let innerClipRectParamExpr = &egraph.id_to_expr(subst[self.innerClipRectParams]);
        let innerClipRectParams = match &innerClipRectParamExpr[0.into()] {
            SkiLang::ClipRectParams(value) => value,
            _ => panic!("This is not a ClipRectParams"),
        };

        let outerClipRectParamExpr = &egraph.id_to_expr(subst[self.outerClipRectParams]);
        let outerClipRectParams = match &outerClipRectParamExpr[0.into()] {
            SkiLang::ClipRectParams(value) => value,
            _ => panic!("This is not a ClipRectParams"),
        };

        if innerClipRectParams.is_anti_aliased != outerClipRectParams.is_anti_aliased {
            return vec![];
        }

        if innerClipRectParams.clip_rect_mode != SkiLangClipRectMode::Intersect
            || outerClipRectParams.clip_rect_mode != SkiLangClipRectMode::Intersect
        {
            return vec![];
        }

        let mergedClipRectParams = SkiLang::ClipRectParams(SkiLangClipRectParams {
            clip_rect_mode: innerClipRectParams.clip_rect_mode,
            is_anti_aliased: innerClipRectParams.is_anti_aliased,
            bounds: bounds_intersection(&innerClipRectParams.bounds, &outerClipRectParams.bounds),
        });

        let mut subst = subst.clone();
        let mut expr = RecExpr::default();
        expr.add(mergedClipRectParams);
        subst.insert(self.foldedClipRectParams, egraph.add_expr(&expr));
        self.expr
            .apply_one(egraph, matched_id, &subst, searcher_pattern, rule_name)
    }
}
*/