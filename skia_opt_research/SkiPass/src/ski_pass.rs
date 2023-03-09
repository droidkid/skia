use egg::*;
use std::error::Error;

use std::fmt::Write;

use crate::protos;
use crate::protos::{
    SkPaint,
    Bounds,
    SkColor,
    SkRecord, 
    SkiPassInstruction,
    SkiPassProgram, 
    SkiPassRunInfo,
    SkiPassRunResult,
    BlendMode,
    ClipOp,
    sk_paint::Blender,
    sk_paint::ImageFilter,
    sk_paint::ColorFilter,
    sk_paint::PathEffect,
    sk_paint::MaskFilter,
    sk_paint::Shader,
    ski_pass_instruction::SkiPassCopyRecord,
    ski_pass_instruction::Instruction,
    ski_pass_instruction::SaveLayer,
    ski_pass_instruction::Save,
    ski_pass_instruction::Restore,
    ski_pass_instruction::ClipRect, 
};
use crate::ski_lang::SkiLang;
use crate::ski_lang::make_rules;
use crate::ski_lang::SkiLangCostFn;
use crate::build_ski_lang_expr::build_expr;
use crate::build_ski_lang_expr::SkiLangExpr;
use crate::ski_lang_to_program::expr_to_program;

pub fn optimize(record: SkRecord) -> SkiPassRunResult {
    let mut expr = RecExpr::default();

    let mut skiPassRunResult = SkiPassRunResult::default();
    let mut skiRunInfo = SkiPassRunInfo::default();

    skiRunInfo.input_record = Some(record.clone());

    let _id = build_expr(&mut record.records.iter(), &mut expr);

    match run_eqsat_and_extract(&expr, &mut skiRunInfo) {
        Ok(optExpr) => {
            let mut program = SkiPassProgram::default();
            program.instructions = expr_to_program(&optExpr.expr, optExpr.id);
            skiPassRunResult.program = Some(program);
            skiPassRunResult.run_info = Some(skiRunInfo);
        }
        Err(_e) => {}
    }
    skiPassRunResult
}


fn run_eqsat_and_extract(
    expr: &RecExpr<SkiLang>,
    run_info: &mut protos::SkiPassRunInfo,
    ) -> Result<SkiLangExpr, Box<dyn Error>> {
    let runner = Runner::default().with_expr(expr).run(&make_rules());
    let root = runner.roots[0];

    writeln!(&mut run_info.skilang_expr, "{:#}", expr);
    // println!("EXPR: {:#}", expr);

    let extractor = Extractor::new(&runner.egraph, SkiLangCostFn);
    let (_cost, optimized) = extractor.find_best(root);

    writeln!(&mut run_info.extracted_skilang_expr, "{:#}", optimized);
    // println!("OPT: {:#}", optimized);

    // Figure out how to walk a RecExpr without the ID.
    // Until then, use this roundabout way to get the optimized recexpr id.
    let mut egraph = EGraph::<SkiLang, ()>::default();
    let id = egraph.add_expr(&optimized);

    Ok(SkiLangExpr {
        expr: optimized,
        id,
    })
}

