use egg::*;
use std::error::Error;

use std::fmt::Write;

use crate::build_ski_lang_expr::build_expr;
use crate::build_ski_lang_expr::SkiLangExpr;
use crate::protos;
use crate::protos::{SkRecord, SkiPassDebugInfo, SkiPassProgram, SkiPassRunResult};
use crate::ski_lang::make_rules;
use crate::ski_lang::SkiLang;
use crate::ski_lang::SkiLangCostFn;
use crate::ski_lang_to_program::expr_to_program;

pub fn optimize(record: SkRecord) -> SkiPassRunResult {
    let mut skiPassDebugInfo = SkiPassDebugInfo::default();
    skiPassDebugInfo.input_record = Some(record.clone());

    let mut expr = RecExpr::default();
    let _id = build_expr(&mut record.records.iter(), &mut expr);

    match run_eqsat_and_extract(&expr, &mut skiPassDebugInfo) {
        Ok(optExpr) => {
            let mut optimized_program = SkiPassProgram::default();
            optimized_program.instructions = expr_to_program(&optExpr.expr, optExpr.id);

            let mut skiPassRunResult = SkiPassRunResult::default();
            skiPassRunResult.optimized_program = Some(optimized_program);
            skiPassRunResult.debug_info = Some(skiPassDebugInfo);
            skiPassRunResult
        }
        Err(_e) => {
            panic!("Failed to run eqsat and extraction");
        }
    }
}

fn run_eqsat_and_extract(
    expr: &RecExpr<SkiLang>,
    debug_info: &mut protos::SkiPassDebugInfo,
) -> Result<SkiLangExpr, Box<dyn Error>> {
    let runner = Runner::default()
        .with_explanations_enabled()
        .with_expr(expr)
        .run(&make_rules());
    writeln!(&mut debug_info.skilang_expr, "{:#}", expr);
    let root = runner.roots[0];
    let extractor = Extractor::new(&runner.egraph, SkiLangCostFn);
    let (_cost, optimized) = extractor.find_best(root);
    writeln!(&mut debug_info.extracted_skilang_expr, "{:#}", optimized);

    // TODO: Figure out how to walk a RecExpr without the ID.
    // Until then, use this roundabout way to get the optimized recexpr id.
    let mut egraph = EGraph::<SkiLang, ()>::default();
    let id = egraph.add_expr(&optimized);
    Ok(SkiLangExpr {
        expr: optimized,
        id,
    })
}
