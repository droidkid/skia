use egg::*;
use std::fmt;
use std::str::FromStr;
use std::error::Error;
use std::fs::File;
use std::io::Write;
use serde::Deserialize;
use strum_macros::{EnumString, EnumVariantNames};
use ordered_float::OrderedFloat;
use parse_display::{Display, FromStr};

use crate::ski_lang::SkiLang;


#[derive(Deserialize, Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SkPaint {
    // ARGB, 0-255
    #[serde(default = "default_color")]
    pub color: Vec<u8>,
}

impl fmt::Display for SkPaint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "(a:{:?}, r:{:?}, g:{:?}, b:{:?})", 
            self.color[0], 
            self.color[1], 
            self.color[2], 
            self.color[3], 
        )
    }
}

impl FromStr for SkPaint {
    type Err = Box<dyn Error>;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        todo!("Parsing from string not implemented yet!");
    }
}

#[derive(Deserialize, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SkBBox { 
    pub l: OrderedFloat<f32>, 
    pub t: OrderedFloat<f32>, 
    pub r: OrderedFloat<f32>,
    pub b: OrderedFloat<f32> 
}

impl fmt::Display for SkBBox {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({:?}, {:?}, {:?}, {:?})", self.l, self.t, self.r, self.b)
    }
}

impl FromStr for SkBBox {
    type Err = Box<dyn Error>;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        todo!("Parsing from string not implemented yet!");
    }
}

fn default_color() -> Vec<u8> {
    vec![255, 0, 0, 0]
}

fn get_skbbox(skilang_expr: &RecExpr<SkiLang>, id: Id) -> SkBBox {
    match &skilang_expr[id] {
        SkiLang::BBox(val) => (*val).try_into().unwrap(),
        _ => {
            panic!("This is not a Bounding Box!")
        }
    }
}

#[derive(Deserialize, Debug, Clone, EnumVariantNames, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(tag = "command")]
pub enum SkDrawCommand {
    DrawRect {
        coords: Vec<OrderedFloat<f32>>,
        paint: SkPaint,
        visible: bool,
    },
    DrawOval {
        coords: Vec<OrderedFloat<f32>>,
        paint: SkPaint,
        visible: bool,
    },
    ClipRect {
        coords: Vec<OrderedFloat<f32>>,
        visible: bool,
    }, // TODO: Support op, antiAlias
    Save {
        visible: bool,
    },
    SaveLayer {
        paint: Option<SkPaint>,
        visible: bool,
    },
    Restore {
        visible: bool,
    },
}

impl fmt::Display for SkDrawCommand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SkDrawCommand")
    }
}

impl FromStr for SkDrawCommand {
    type Err = Box<dyn Error>;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        todo!("Parsing from string not implemented yet!");
    }
}

#[derive(Debug)]
pub enum SurfaceType {
    Abstract,
    AbstractWithState,
    Allocated,
}

#[derive(Deserialize, Debug)]
pub struct SkPicture {
    #[serde(rename = "commands")]
    pub drawCommands: Vec<SkDrawCommand>,
    #[serde(skip)] // Only used internally.
    pub surfaceType: Option<SurfaceType>,
}


pub fn generate_skpicture(expr: &RecExpr<SkiLang>, id: Id) -> SkPicture {
    let node = &expr[id];
    match node {
        SkiLang::DrawCommand(skDrawCommand) => {
            SkPicture {
                drawCommands: vec![skDrawCommand.clone()],
                surfaceType: Some(SurfaceType::Abstract),
            }
        }
        SkiLang::ClipRect(ids) => {
            let mut src = generate_skpicture(&expr, ids[0]);
            let skBBox = get_skbbox(&expr, ids[1]);
            let mut drawCommands: Vec<SkDrawCommand> = vec![];
            drawCommands.push(SkDrawCommand::ClipRect {
                coords: vec![skBBox.l, skBBox.t, skBBox.r, skBBox.b],
                visible: true,
            });
            drawCommands.append(&mut src.drawCommands);

            SkPicture {
                drawCommands,
                surfaceType: Some(SurfaceType::AbstractWithState),
            }
        }
        SkiLang::SrcOver(ids) => {
            let mut dst = generate_skpicture(&expr, ids[0]);
            let mut src = generate_skpicture(&expr, ids[1]);

            let mut drawCommands: Vec<SkDrawCommand> = vec![];

            match dst.surfaceType {
                Some(SurfaceType::Abstract) => {
                    drawCommands.append(&mut dst.drawCommands);
                }
                Some(SurfaceType::AbstractWithState) => {
                    drawCommands.push(SkDrawCommand::Save { visible: true });
                    drawCommands.append(&mut dst.drawCommands);
                    drawCommands.push(SkDrawCommand::Restore { visible: true });
                }
                Some(SurfaceType::Allocated) => {
                    drawCommands.append(&mut dst.drawCommands);
                }
                None => {}
            };

            match src.surfaceType {
                Some(SurfaceType::Allocated) => {
                    drawCommands.push(SkDrawCommand::SaveLayer {
                        paint: None,
                        visible: true,
                    });
                    drawCommands.append(&mut src.drawCommands);
                    drawCommands.push(SkDrawCommand::Restore { visible: true });
                }
                Some(SurfaceType::AbstractWithState) => {
                    drawCommands.push(SkDrawCommand::Save { visible: true });
                    drawCommands.append(&mut src.drawCommands);
                    drawCommands.push(SkDrawCommand::Restore { visible: true });
                }
                Some(SurfaceType::Abstract) => {
                    drawCommands.append(&mut src.drawCommands);
                }
                None => {}
            };

            SkPicture {
                drawCommands,
                surfaceType: Some(SurfaceType::Allocated),
            }
        }
        SkiLang::Blank => SkPicture {
            drawCommands: vec![],
            surfaceType: Some(SurfaceType::Allocated),
        },
        _ => {
            panic!("The RecExpr looks off!")
        }
    }
}

pub fn print_skp(skp: &SkPicture) {
    print_commands(&mut skp.drawCommands.iter());
}

fn print_commands<'a, I>(mut drawCommands: &mut I)
where
    I: Iterator<Item = &'a SkDrawCommand> + 'a,
{
    match drawCommands.next() {
        Some(d) => {
            println!("{:?}", d);
            print_commands(drawCommands);
        }
        None => {}
    }
}
