use egg::*;
use std::error::Error;

use crate::protos::{SkRecord, SkiPassProgram, SkiPassRunResult};
use crate::sk_record_to_ski_lang::convert_sk_record_to_ski_lang_expr;
use crate::ski_lang::SkiLang;
use crate::ski_lang::SkiLangCostFn;
use crate::ski_lang::make_rules;
use crate::ski_lang_to_program::expr_to_program;

pub fn optimize(record: SkRecord) -> SkiPassRunResult {
    let expr = convert_sk_record_to_ski_lang_expr(&mut record.records.iter());
    match run_eqsat_and_extract(&expr) {
        Ok(optimized_expr) => {
            let mut optimized_program = SkiPassProgram::default();
            optimized_program.instructions = expr_to_program(&optimized_expr);
            let mut ski_pass_run_result = SkiPassRunResult::default();
            ski_pass_run_result.optimized_program = Some(optimized_program);
            ski_pass_run_result 
        }
        Err(_e) => {
            panic!("Failed to run eqsat and extraction");
        }
    }
}

fn run_eqsat_and_extract(expr: &RecExpr<SkiLang>) -> Result<RecExpr<SkiLang>, Box<dyn Error>> {
    let runner = Runner::default()
        .with_explanations_enabled()
        .with_expr(expr)
        .run(&make_rules());
    let root = runner.roots[0];
    let extractor = Extractor::new(&runner.egraph, SkiLangCostFn);
    let (_cost, optimized_expr) = extractor.find_best(root);
    Ok(optimized_expr)
}