use egg::*;
use crate::ski_lang::SkiLang;

use crate::protos::{
    SkPaint,
    Bounds,
    SkColor,
    BlendMode,
    sk_paint::Blender,
    sk_paint::ImageFilter,
    sk_paint::ColorFilter,
    sk_paint::PathEffect,
    sk_paint::MaskFilter,
    sk_paint::Shader, 
};

pub fn bounds_proto_to_rect_expr(expr: &mut RecExpr<SkiLang>, bounds: &Option<Bounds>) -> Id {
    match bounds {
        Some(bounds) => {
            let _boundsExist = expr.add(SkiLang::Exists(true));

            let left = ordered_float::NotNan::new(bounds.left).unwrap();
            let top = ordered_float::NotNan::new(bounds.top).unwrap();
            let right = ordered_float::NotNan::new(bounds.right).unwrap();
            let bottom = ordered_float::NotNan::new(bounds.bottom).unwrap();

            let leftExpr = expr.add(SkiLang::Float(left));
            let topExpr = expr.add(SkiLang::Float(top));
            let rightExpr = expr.add(SkiLang::Float(right));
            let bottomExpr = expr.add(SkiLang::Float(bottom));

            expr.add(SkiLang::Rect([leftExpr, topExpr, rightExpr, bottomExpr]))
        },
        None => {
            panic!("There is no Bounds Proto to unpack!");
        }
    }
}

pub fn bounds_proto_to_expr(expr: &mut RecExpr<SkiLang>, bounds: &Option<Bounds>) -> Id {
    match bounds {
        Some(_value) => {
            let boundsExist = expr.add(SkiLang::Exists(true));
            let rect = bounds_proto_to_rect_expr(expr, bounds);
            expr.add(SkiLang::Bounds([boundsExist, rect]))
        },
        None => {
            let boundsExist = expr.add(SkiLang::Exists(false));
            let noOp = expr.add(SkiLang::NoOp);
            expr.add(SkiLang::Bounds([boundsExist, noOp]))
        }
    }
}

pub fn paint_proto_to_expr(expr: &mut RecExpr<SkiLang>, skPaint: &Option<SkPaint>) -> Id {
    let color = match &skPaint {
       	Some(skPaint) => {
       	    match &skPaint.color {
       	        Some(skColor) => {
       	            color_proto_to_expr(expr, 
       	                skColor.alpha_u8,
       	                skColor.red_u8,
       	                skColor.green_u8,
       	                skColor.blue_u8)
       	            }
       	        None => {
                    // TODO: Assert that this only happens in SaveLayer.
                    color_proto_to_expr(expr, 255, 0, 0, 0)
                }
       	    }
       	},
       	None => {
            // TODO: Assert that this only happens in SaveLayer.
            color_proto_to_expr(expr, 255, 0, 0, 0)
        }
    };


    let blender = match &skPaint {
        Some(skPaint) => {
            match &skPaint.blender {
                Some(blender) => {
                    if blender.blend_mode == BlendMode::SrcOver.into() {
                        let blendMode = expr.add(SkiLang::BlendMode_SrcOver);
                        expr.add(SkiLang::Blender([blendMode]))
                    } 
                    else if blender.blend_mode == BlendMode::Src.into() {
                        let blendMode = expr.add(SkiLang::BlendMode_Src);
                        expr.add(SkiLang::Blender([blendMode]))
                    }
                    else {
                        let blendMode = expr.add(SkiLang::BlendMode_Unknown);
                        expr.add(SkiLang::Blender([blendMode]))
                    }
                },
                None => {
                    let blendMode = expr.add(SkiLang::BlendMode_SrcOver);
                    expr.add(SkiLang::Blender([blendMode]))
                }
            }
        },
        None => {
            let blendMode = expr.add(SkiLang::BlendMode_SrcOver);
            expr.add(SkiLang::Blender([blendMode]))
        }
    };

    let image_filter = match &skPaint {
       	Some(skPaint) => {
            let exists = expr.add(SkiLang::Exists(skPaint.image_filter.is_some()));
            expr.add(SkiLang::ImageFilter([exists]))
       	},
       	None => {
            let exists = expr.add(SkiLang::Exists(false));
            expr.add(SkiLang::ImageFilter([exists]))
        }
    };

    let color_filter = match &skPaint {
       	Some(skPaint) => {
            let exists = expr.add(SkiLang::Exists(skPaint.color_filter.is_some()));
            expr.add(SkiLang::ColorFilter([exists]))
       	},
       	None => {
            let exists = expr.add(SkiLang::Exists(false));
            expr.add(SkiLang::ColorFilter([exists]))
        }
    };

    let path_effect = match &skPaint {
       	Some(skPaint) => {
            let exists = expr.add(SkiLang::Exists(skPaint.path_effect.is_some()));
            expr.add(SkiLang::PathEffect([exists]))
       	},
       	None => {
            let exists = expr.add(SkiLang::Exists(false));
            expr.add(SkiLang::PathEffect([exists]))
        }
    };

    let mask_filter = match &skPaint {
       	Some(skPaint) => {
            let exists = expr.add(SkiLang::Exists(skPaint.mask_filter.is_some()));
            expr.add(SkiLang::MaskFilter([exists]))
       	},
       	None => {
            let exists = expr.add(SkiLang::Exists(false));
            expr.add(SkiLang::MaskFilter([exists]))
        }
    };

    let shader = match &skPaint {
       	Some(skPaint) => {
            let exists = expr.add(SkiLang::Exists(skPaint.shader.is_some()));
            expr.add(SkiLang::Shader([exists]))
       	},
       	None => {
            let exists = expr.add(SkiLang::Exists(false));
            expr.add(SkiLang::Shader([exists]))
        }
    };

    expr.add(SkiLang::Paint([
            color, 
            blender,
            image_filter,
            color_filter,
            path_effect,
            mask_filter,
            shader
        ]))
}

pub fn color_proto_to_expr(expr: &mut RecExpr<SkiLang>, aVal:i32, rVal:i32, gVal:i32, bVal:i32) -> Id {
    let a = expr.add(SkiLang::Num(aVal));
    let r = expr.add(SkiLang::Num(rVal));
    let g = expr.add(SkiLang::Num(gVal));
    let b = expr.add(SkiLang::Num(bVal));
    expr.add(SkiLang::Color([a, r, g, b]))
}

pub fn bounds_expr_to_proto(expr: &RecExpr<SkiLang>, id: Id) -> Option<Bounds> {
    let bounds: Option<Bounds> = match &expr[id] {
        SkiLang::Bounds(ids) => {
            match &expr[ids[0]] {
                SkiLang::Exists(true) => {
                    Some(unpack_rect_to_bounds(&expr, ids[1]))
                },
                SkiLang::Exists(false) => {
                    None
                },
                _ => panic!("First param of bounds not exist flag")
            }
        },
        _ => panic!("Merge params 4th param is not bounds")
    };
    bounds
}

pub fn unpack_rect_to_bounds(expr: &RecExpr<SkiLang>, id: Id) -> Bounds {
    match &expr[id] {
        SkiLang::Rect(ids) => {
            let left = unpack_float(expr, ids[0]);
            let top = unpack_float(expr, ids[1]);
            let right = unpack_float(expr, ids[2]);
            let bottom = unpack_float(expr, ids[3]);
            Bounds {
                left,
                top,
                right,
                bottom
            }
        },
        _ => panic!("This is not a rect!")
    }
}

pub fn unpack_float(expr: &RecExpr<SkiLang>, id: Id) -> f64 {
    match &expr[id] {
        SkiLang::Float(val) => {
            **val
        },
        _ => panic!("This is not a float!")
    }
}

pub fn get_exists_value(expr: &RecExpr<SkiLang>, id: Id) -> bool {
    match expr[id] {
        SkiLang::Exists(value) => {
            value
        },
        _ => panic!("Not a SkiLang::Exists")
    }
}

pub fn get_blend_mode(expr: &RecExpr<SkiLang>, id: Id) -> i32 {
    match expr[id] {
        SkiLang::BlendMode_Src => BlendMode::Src.into() ,
        SkiLang::BlendMode_SrcOver => BlendMode::SrcOver.into(),
        SkiLang::BlendMode_Unknown => BlendMode::Unknown.into(),
        _ => panic!("Not a valid BlendMode")
    }
}

pub fn paint_expr_to_proto(expr: &RecExpr<SkiLang>, id: Id) -> SkPaint {
    let paint_param_ids = match expr[id] {
        SkiLang::Paint(ids) => ids,
        _ => panic!("Attempting to convert a non paint expr to proto")
    };
    let color = Some(color_expr_to_proto(expr, paint_param_ids[0]));

    let blend_mode = match expr[paint_param_ids[1]] {
        SkiLang::Blender(ids) => get_blend_mode(expr, ids[0]),
        _ => panic!("Second parameter of Paint is not Blender!")
    };

    let image_filter_exists = match expr[paint_param_ids[2]] {
        SkiLang::ImageFilter(ids) => get_exists_value(expr, ids[0]),
        _ => panic!("Third parameter of Paint is not ImageFilter!")
    };

    let color_filter_exists = match expr[paint_param_ids[3]] {
        SkiLang::ColorFilter(ids) => get_exists_value(expr, ids[0]),
        _ => panic!("Fourth parameter of Paint is not ColorFilter!")
    };

    let path_effect_exists = match expr[paint_param_ids[4]] {
        SkiLang::PathEffect(ids) => get_exists_value(expr, ids[0]),
        _ => panic!("Fifth parameter of Paint is not PathEffect!")
    };

    let mask_filter_exists = match expr[paint_param_ids[5]] {
        SkiLang::MaskFilter(ids) => get_exists_value(expr, ids[0]),
        _ => panic!("Sixth parameter of Paint is not MaskFilter!")
    };

    let shader_exists = match expr[paint_param_ids[6]] {
        SkiLang::Shader(ids) => get_exists_value(expr, ids[0]),
        _ => panic!("Seventh parameter of Paint is not Shader!")
    };

    SkPaint {
        color,
        // TODO: Fill these fields.
        // It doesn't really matter now, we bail out and copy the command
        // if any of the below fields are set. Only the color.alpha matters
        // at this point.
        blender: Some(Blender{
            blend_mode
        }),
        image_filter: if image_filter_exists {
            Some(ImageFilter {})
        } else {
            None
        },
        color_filter: if color_filter_exists {
            Some(ColorFilter {})
        } else {
            None
        },
        path_effect: if path_effect_exists {
            Some(PathEffect {})
        } else {
            None
        },
        mask_filter: if mask_filter_exists {
            Some(MaskFilter {})
        } else {
            None
        },
        shader: if shader_exists {
            Some(Shader {})
        } else {
            None
        },
    }
}



pub fn color_expr_to_proto(expr: &RecExpr<SkiLang>, id: Id) -> SkColor {
    let node = &expr[id];
    match node {
        SkiLang::Color(ids) => {
            let alpha_u8  = match &expr[ids[0]] {
                SkiLang::Num(value) => *value,
                _ => panic!()
            };
            let red_u8  = match &expr[ids[1]] {
                SkiLang::Num(value) => *value,
                _ => panic!()
            };
            let green_u8  = match &expr[ids[2]] {
                SkiLang::Num(value) => *value,
                _ => panic!()
            };
            let blue_u8  = match &expr[ids[3]] {
                SkiLang::Num(value) => *value,
                _ => panic!()
            };
    
            SkColor {
              alpha_u8,
              red_u8,
              green_u8,
              blue_u8
            }
        },
        _ => {
            panic!("Not a Color!");
        }
    }
}
