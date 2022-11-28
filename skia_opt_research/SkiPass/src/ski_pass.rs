use egg::*;
use std::error::Error;
use std::fmt;
use prost::Message;

use crate::protos;
use crate::protos::{
    SkRecord, 
    SkRecords, 
    SkiPassInstruction,
    SkiPassProgram, 
    SkiPassRunInfo,
    SkiPassRunResult,
    ski_pass_instruction::SkiPassCopyRecord,
    ski_pass_instruction::Instruction,
    sk_records::Command, 
};

pub fn optimize(record: SkRecord) -> SkiPassRunResult {
    let mut expr = RecExpr::default();
    let blankSurface = expr.add(SkiLang::Blank);
    let id = build_expr(&mut record.records.iter(), blankSurface, &mut expr);

    let mut skiPassRunResult = SkiPassRunResult::default();
    let mut skiRunInfo = SkiPassRunInfo::default();

    match run_eqsat_and_extract(&expr, &mut skiRunInfo) {
        Ok(optExpr) => {
            let mut program = SkiPassProgram::default();
            program.instructions = build_program(&optExpr.expr, optExpr.id);
            skiPassRunResult.program = Some(program);
        }
        Err(e) => {}
    }
    skiPassRunResult
}

define_language! {
    enum SkiLang {
        DrawCommand(i32), // skRecords index
        "blank" = Blank,
        "clipRect" = ClipRect([Id; 1]), // layer
        "srcOver" = SrcOver([Id; 2]), // dst, src
    }
}

fn make_rules() -> Vec<Rewrite<SkiLang, ()>> {
    vec![
        rewrite!("remove-blank-dst-savelayers"; "(srcOver ?a blank)" => "?a"),
        rewrite!("remove-blank-src-savelayers"; "(srcOver blank ?a)" => "?a"),
    ]
}

fn run_eqsat_and_extract(
    expr: &RecExpr<SkiLang>,
    run_info: &mut protos::SkiPassRunInfo,
) -> Result<SkiLangExpr, Box<dyn Error>> {

    let mut runner = Runner::default().with_expr(expr).run(&make_rules());
    let root = runner.roots[0];

    let extractor = Extractor::new(&runner.egraph, AstSize);
    let (cost, mut optimized) = extractor.find_best(root);

    // Figure out how to walk a RecExpr without the ID.
    // Until then, use this roundabout way to get the optimized recexpr id.
    let mut egraph = EGraph::<SkiLang, ()>::default();
    let id = egraph.add_expr(&optimized);

    Ok(SkiLangExpr {
        expr: optimized,
        id,
    })
}

struct SkiLangExpr {
    pub expr: RecExpr<SkiLang>,
    pub id: Id,
}

fn build_expr<'a, I>(skRecordsIter: &mut I, dst: Id, expr: &mut RecExpr<SkiLang>) -> Id
where
    I: Iterator<Item = &'a SkRecords> + 'a,
{
    match skRecordsIter.next() {
        Some(skRecords) => {
            match &skRecords.command {
                Some(Command::DrawCommand(draw_command)) => {
                    let drawCommand = expr.add(SkiLang::DrawCommand(skRecords.index));
                    let nextDst = expr.add(SkiLang::SrcOver([dst, drawCommand]));
                    build_expr(skRecordsIter, nextDst, expr)
                },
                _ => {
                    panic!("Not implemented yet!")
                }
                None => {
                    panic!("Empty command!")
                }
            }
        }
        None => dst,
    }
}

fn build_program(expr: &RecExpr<SkiLang>, id: Id) -> Vec<SkiPassInstruction> {
    let node = &expr[id];
    match node {
        SkiLang::DrawCommand(index) => {
            let instruction = SkiPassInstruction {
                // oneof -> Option of a Enum
                instruction: Some(Instruction::CopyRecord(
                    SkiPassCopyRecord {
                        index: *index
                    }
                ))
            };
            vec![instruction]
        },
        SkiLang::SrcOver(ids) => {
            let mut src = build_program(&expr, ids[0]);
            let mut dst = build_program(&expr, ids[1]);

            let mut commands: Vec<SkiPassInstruction> = vec![];

            // TODO: Implement saveLayer, save, clipRect
            commands.append(&mut src);
            commands.append(&mut dst);

            commands
        },
        _ => {
            vec![]
        }
    }
}
