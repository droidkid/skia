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
        "apply_state" = ApplyState([Id; 2]), // alphaChannel, layer
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
            SkiLang::ApplyState(_ids) => (1, 0, 1),
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
