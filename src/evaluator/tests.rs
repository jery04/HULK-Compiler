//! Unit tests for the arithmetic evaluator.
//!
//! These tests verify that the evaluator correctly computes results for
//! various expressions, including:
//! - Basic arithmetic and operator precedence.
//! - Parenthesized expressions.
//! - Mathematical functions (`sqrt`, `exp`, `sin`, `cos`, `log`).
//! - The constant `PI`.
//! - Error cases (division by zero, invalid logarithms, negative square root).

use crate::evaluator::evaluator::evaluate;

// ============================================
// TESTS AVANZADOS (RAICES, EXP, TRIG, PI, E)
// ============================================

#[test]
fn test_advanced_sqrt_exp_cos_pi() {
    // sqrt(81) + exp(1) + cos(PI) = 9 + e - 1 ~= 10.718 -> 10
    let result = evaluate("sqrt(81) + exp(1) + cos(PI)");
    assert_eq!(result, Some(10), "sqrt(81)+exp(1)+cos(PI) debe truncar a 10");
}

#[test]
fn test_advanced_trig_identity_with_root() {
    // sin(PI/2)^2 + cos(0)^2 + sqrt(49) = 1 + 1 + 7 = 9
    let result = evaluate("sin(PI/2)^2 + cos(0)^2 + sqrt(49)");
    assert_eq!(result, Some(9), "sin(PI/2)^2 + cos(0)^2 + sqrt(49) debe evaluar a 9");
}

#[test]
fn test_advanced_log_with_e_and_exp() {
    // log(2,1024) + sqrt(144)/exp(1) = 10 + (12/e) ~= 14.41 -> 14
    let result = evaluate("log(2,1024) + sqrt(144)/exp(1)");
    assert_eq!(result, Some(14), "log(2,1024) + sqrt(144)/exp(1) debe truncar a 14");
}

#[test]
fn test_advanced_exp_power_root_combo() {
    // exp(2) - sqrt(2^10) + sin(3*PI/2) = e^2 - 32 - 1 ~= -25.61 -> -25
    let result = evaluate("exp(2) - sqrt(2^10) + sin(3*PI/2)");
    assert_eq!(result, Some(-25), "exp(2)-sqrt(2^10)+sin(3*PI/2) debe truncar a -25");
}

#[test]
fn test_advanced_nested_with_pi_and_e_base_log() {
    // sqrt(16+9) * cos(2*PI) + log(exp(1),exp(4)) = 5*1 + 4 = 9
    let result = evaluate("sqrt(16+9) * cos(2*PI) + log(exp(1),exp(4))");
    assert_eq!(result, Some(9), "sqrt(16+9)*cos(2*PI)+log(exp(1),exp(4)) debe evaluar a 9");
}

#[test]
fn test_advanced_pi_cycle_and_log10() {
    // sin(PI) + cos(PI) + exp(0) + sqrt(64) - log(10,1000) = 0 - 1 + 1 + 8 - 3 = 5
    let result = evaluate("sin(PI) + cos(PI) + exp(0) + sqrt(64) - log(10,1000)");
    assert_eq!(result, Some(5), "sin(PI)+cos(PI)+exp(0)+sqrt(64)-log(10,1000) debe evaluar a 5");
}

#[test]
fn test_advanced_exp_square_inside_root() {
    // sqrt(exp(2)^2 + 9) - 1 ~= sqrt(63.59) - 1 ~= 6.97 -> 6
    let result = evaluate("sqrt(exp(2)^2 + 9) - 1");
    assert_eq!(result, Some(6), "sqrt(exp(2)^2 + 9) - 1 debe truncar a 6");
}

#[test]
fn test_advanced_complex_error_log_negative_value() {
    let result = evaluate("sqrt(49) + log(exp(1), -4)");
    assert_eq!(result, None, "log(exp(1), -4) debe retornar None por valor negativo");
}

// ============================================
// TESTS COMPLEJOS CON PARÉNTESIS
// ============================================

#[test]
fn test_parenthesized_addition() {
    // 2 * (5 + 8) = 2 * 13 = 26
    let result = evaluate("2*(5+8)");
    assert_eq!(result, Some(26), "2*(5+8) debe evaluar a 26");
}

#[test]
fn test_nested_parentheses() {
    // ((2 + 3) * 4) - 5 = (5 * 4) - 5 = 20 - 5 = 15
    let result = evaluate("((2+3)*4)-5");
    assert_eq!(result, Some(15), "((2+3)*4)-5 debe evaluar a 15");
}

#[test]
fn test_complex_expression_1() {
    // 3 * 4 + 2 * 5 = 12 + 10 = 22
    let result = evaluate("3*4+2*5");
    assert_eq!(result, Some(22), "3*4+2*5 debe evaluar a 22");
}

#[test]
fn test_complex_expression_2() {
    // 10 - 2 * 3 = 10 - 6 = 4
    let result = evaluate("10-2*3");
    assert_eq!(result, Some(4), "10-2*3 debe evaluar a 4");
}

// ============================================
// TESTS CON MÚLTIPLES OPERACIONES
// ============================================

#[test]
fn test_chain_additions() {
    // 1 + 2 + 3 + 4 = 10
    let result = evaluate("1+2+3+4");
    assert_eq!(result, Some(10), "1+2+3+4 debe evaluar a 10");
}

#[test]
fn test_chain_subtractions() {
    // 20 - 5 - 3 - 2 = ((20 - 5) - 3) - 2 = (15 - 3) - 2 = 12 - 2 = 10
    let result = evaluate("20-5-3-2");
    assert_eq!(result, Some(10), "20-5-3-2 debe evaluar a 10");
}

#[test]
fn test_mixed_operations() {
    // 2 + 3 * 4 - 5 = 2 + 12 - 5 = 9
    let result = evaluate("2+3*4-5");
    assert_eq!(result, Some(9), "2+3*4-5 debe evaluar a 9");
}

// ============================================
// TESTS CON OPERADOR DE POTENCIA
// ============================================

#[test]
fn test_power_parenthesized_left_grouping() {
    // (2^3)^2 = 8^2 = 64
    let result = evaluate("(2^3)^2");
    assert_eq!(result, Some(64), "(2^3)^2 debe evaluar a 64");
}

#[test]
fn test_power_precedence_over_multiplication() {
    // 2 * 3^2 = 2 * 9 = 18
    let result = evaluate("2*3^2");
    assert_eq!(result, Some(18), "2*3^2 debe evaluar a 18");
}

#[test]
fn test_power_and_mixed_precedence() {
    // 2 + 3 * 2^3 = 2 + 3 * 8 = 26
    let result = evaluate("2+3*2^3");
    assert_eq!(result, Some(26), "2+3*2^3 debe evaluar a 26");
}

// ============================================
// TESTS CON UNARIO NEGACIÓN
// ============================================

#[test]
fn test_negation_in_expression() {
    // -5 + 10 = 5
    let result = evaluate("-5+10");
    assert_eq!(result, Some(5), "-5+10 debe evaluar a 5");
}

#[test]
fn test_negation_with_multiplication() {
    // -3 * 4 = -12
    let result = evaluate("-3*4");
    assert_eq!(result, Some(-12), "-3*4 debe evaluar a -12");
}

#[test]
fn test_negation_double() {
    // --5 = 5
    let result = evaluate("--5");
    assert_eq!(result, Some(5), "--5 debe evaluar a 5");
}

// ============================================
// TESTS CON MÓDULO
// ============================================

#[test]
fn test_modulo_operation() {
    // 17 % 5 = 2
    let result = evaluate("17%5");
    assert_eq!(result, Some(2), "17%5 debe evaluar a 2");
}

#[test]
fn test_modulo_in_complex_expression() {
    // 20 % 6 + 3 = 2 + 3 = 5
    let result = evaluate("20%6+3");
    assert_eq!(result, Some(5), "20%6+3 debe evaluar a 5");
}

// ============================================
// TESTS CON NÚMEROS GRANDES
// ============================================

#[test]
fn test_large_numbers() {
    // 1000 * 1000 = 1000000
    let result = evaluate("1000*1000");
    assert_eq!(result, Some(1000000), "1000*1000 debe evaluar a 1000000");
}

#[test]
fn test_large_expression() {
    // 500 + 300 * 2 - 100 = 500 + 600 - 100 = 1000
    let result = evaluate("500+300*2-100");
    assert_eq!(result, Some(1000), "500+300*2-100 debe evaluar a 1000");
}

// ============================================
// TESTS DE ERROR (Division por cero)
// ============================================

#[test]
fn test_division_by_zero() {
    let result = evaluate("5/0");
    assert_eq!(result, None, "5/0 debe retornar None (error)");
}

#[test]
fn test_modulo_by_zero() {
    let result = evaluate("5%0");
    assert_eq!(result, None, "5%0 debe retornar None (error)");
}

#[test]
fn test_power_right_associativity() {
    // 2^3^2 = 2^(3^2) = 2^9 = 512
    let result = evaluate("2^3^2");
    assert_eq!(result, Some(512), "2^3^2 debe evaluar a 512");
}

#[test]
fn test_left_associativity_subtraction() {
    // 100 - 10 - 5 = (100 - 10) - 5 = 85
    let result = evaluate("100-10-5");
    assert_eq!(result, Some(85), "100-10-5 debe respetar asociatividad izquierda");
}

#[test]
fn test_left_associativity_division() {
    // 64 / 4 / 4 = (64 / 4) / 4 = 4
    let result = evaluate("64/4/4");
    assert_eq!(result, Some(4), "64/4/4 debe respetar asociatividad izquierda");
}

#[test]
fn test_basic_arithmetic_add_sub_mix() {
    // 7 + 5 - 3 = 9
    let result = evaluate("7+5-3");
    assert_eq!(result, Some(9), "7+5-3 debe evaluar a 9");
}

#[test]
fn test_basic_arithmetic_mul_div_mix() {
    // 18 / 3 * 2 = 12
    let result = evaluate("18/3*2");
    assert_eq!(result, Some(12), "18/3*2 debe evaluar a 12");
}

#[test]
fn test_trig_log_expression() {
    let result = evaluate("sin(2 * PI) ^ 2 + cos(3 * PI / log(4, 64))");
    assert_eq!(result, Some(-1), "sin(2*PI)^2 + cos(3*PI/log(4,64)) debe evaluar a -1");
}

// ============================================
// TESTS ADICIONALES: sqrt, exp, combinados complejos
// ============================================

#[test]
fn test_sqrt_basic() {
    let result = evaluate("sqrt(16)");
    assert_eq!(result, Some(4), "sqrt(16) debe evaluar a 4");
}

#[test]
fn test_exp_basic() {
    let result = evaluate("exp(1)");
    assert_eq!(result, Some(2), "exp(1) (e) truncado debe evaluar a 2");
}

#[test]
fn test_sin_cos_pow_sqrt_1() {
    // sin(PI/2)^2 + cos(0) = 1^2 + 1 = 2
    let result = evaluate("sin(PI/2)^2 + cos(0)");
    assert_eq!(result, Some(2), "sin(PI/2)^2 + cos(0) debe evaluar a 2");
}

#[test]
fn test_sqrt_pow_and_sin() {
    // sqrt(9) + 2^3 * sin(PI/6) = 3 + 8 * 0.5 = 7
    let result = evaluate("sqrt(9) + 2^3 * sin(PI/6)");
    assert_eq!(result, Some(7), "sqrt(9) + 2^3 * sin(PI/6) debe evaluar a 7");
}

#[test]
fn test_log_base2_of_8() {
    let result = evaluate("log(2,8)");
    assert_eq!(result, Some(3), "log(2,8) debe evaluar a 3");
}

#[test]
fn test_stress_big_expression() {
    // 2^10 + sqrt(256) - sin(PI) + cos(0) + exp(0) + log(2,1024) = 1052
    let result = evaluate("2^10 + sqrt(256) - sin(PI) + cos(0) + exp(0) + log(2,1024)");
    assert_eq!(result, Some(1052), "Expresión compleja debe evaluar a 1052");
}

#[test]
fn test_sqrt_negative() {
    let result = evaluate("sqrt(-1)");
    assert_eq!(result, None, "sqrt(-1) debe retornar None (error)");
}

#[test]
fn test_exp_pow_combination() {
    // exp(2) ^ 2 ~= 54.598 -> trunc 54
    let result = evaluate("exp(2)^2");
    assert_eq!(result, Some(54), "exp(2)^2 debe truncar a 54");
}

#[test]
fn test_trig_chain_and_mul() {
    // sin(3*PI/2) + cos(PI) * 2 = -1 + (-1)*2 = -3
    let result = evaluate("sin(3*PI/2) + cos(PI) * 2");
    assert_eq!(result, Some(-3), "sin(3*PI/2) + cos(PI)*2 debe evaluar a -3");
}
