use egg::*;
use std::error::Error;
use std::time::Instant;

use crate::protos::{SkRecord, SkiPassProgram, SkiPassRunResult, SkiPassDurationInfo};
use crate::sk_record_to_ski_lang::convert_sk_record_to_ski_lang_expr;
use crate::ski_lang::SkiLang;
use crate::ski_lang_rules::make_rules;
use crate::ski_lang::SkiLangCostFn;
use crate::ski_lang_to_program::expr_to_program;

pub fn optimize(record: SkRecord) -> SkiPassRunResult {
    let sk_record_to_ski_lang_start = Instant::now();
    let expr = convert_sk_record_to_ski_lang_expr(&mut record.records.iter());
    let sk_record_to_ski_lang_duration = sk_record_to_ski_lang_start.elapsed();

    let eqsat_start = Instant::now();
    match run_eqsat_and_extract(&expr) {
        Ok(optimized_expr) => {
            let eqsat_duration = eqsat_start.elapsed();
            let mut optimized_program = SkiPassProgram::default();

            let expr_to_program_start = Instant::now();
            optimized_program.instructions = expr_to_program(&optimized_expr);
            let expr_to_program_duration = expr_to_program_start.elapsed();

            let mut ski_pass_run_result = SkiPassRunResult::default();
            ski_pass_run_result.optimized_program = Some(optimized_program);
            ski_pass_run_result.duration_info = Some(SkiPassDurationInfo {
                sk_record_ski_lang_duration_nano: sk_record_to_ski_lang_duration.as_nanos() as u64,
                eqsat_duration_nano: eqsat_duration.as_nanos() as u64,
                ski_lang_to_instructions_duration_nano: expr_to_program_duration.as_nanos() as u64
            });
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