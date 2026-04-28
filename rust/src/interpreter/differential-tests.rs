use crate::interpreter::Interpreter;
use crate::types::Stack;

fn run_with_quantization_mode(code: &str, force_no_quant: bool) -> Stack {
    let mut interp = Interpreter::new();
    interp.set_force_no_quant(force_no_quant);

    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    rt.block_on(async {
        interp.execute(code).await.expect("code should execute");
    });

    interp.get_stack().clone()
}

pub fn run_with_both_paths(code: &str) -> (Stack, Stack) {
    (
        run_with_quantization_mode(code, false),
        run_with_quantization_mode(code, true),
    )
}

#[test]
fn differential_harness_smoke() {
    let code = "[ 1 2 3 4 ] { [ 1 ] + } MAP";
    let (quantized, plain) = run_with_both_paths(code);
    assert_eq!(quantized, plain);
}

#[test]
fn differential_arity_logic_and_compare() {
    let code = "[ TRUE FALSE ] [ TRUE TRUE ] AND";
    let (quantized, plain) = run_with_both_paths(code);
    assert_eq!(quantized, plain);
}

#[test]
fn differential_arity_lte_pair() {
    let code = "[ 1 ] [ 2 ] LTE";
    let (quantized, plain) = run_with_both_paths(code);
    assert_eq!(quantized, plain);
}

#[test]
fn differential_math_module_sqrt() {
    let code = "'math' IMPORT 4 MATH@SQRT";
    let (quantized, plain) = run_with_both_paths(code);
    assert_eq!(quantized, plain);
}
