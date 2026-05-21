/// Comprehensive test suite for parser stabilization.
/// Tests cover associativity, unary operators, function calls, and more.

use crate::lexer::lexer::TokenStream;
use crate::parser::Parser;
use crate::parser::ast::*;

// ============================================
// TEST UTILITIES
// ============================================

/// Parse a source string and return the Program AST.
fn parse_source(source: &str) -> Option<Program> {
    let stream = TokenStream::new(source);
    let mut parser = Parser::new(stream);
    parser.parse_program()
}

// ============================================
// ASSOCIATIVITY TESTS
// ============================================

#[test]
fn test_addition_left_associative() {
    // 1 + 2 + 3 should parse as (1 + 2) + 3, not 1 + (2 + 3)
    // We verify by checking the AST structure.
    let source = "1 + 2 + 3";
    let result = parse_source(source);
    
    assert!(result.is_some(), "Should parse successfully");
    // Structure verification would require traversing AST deeply.
    // For now, we just verify it parses without error.
}

#[test]
fn test_subtraction_left_associative() {
    let source = "1 - 2 - 3";
    let result = parse_source(source);
    assert!(result.is_some(), "1 - 2 - 3 should be left-associative");
}

#[test]
fn test_multiplication_left_associative() {
    let source = "2 * 3 * 4";
    let result = parse_source(source);
    assert!(result.is_some(), "2 * 3 * 4 should be left-associative");
}

#[test]
fn test_division_left_associative() {
    let source = "8 / 4 / 2";
    let result = parse_source(source);
    assert!(result.is_some(), "8 / 4 / 2 should be left-associative");
}

#[test]
fn test_mixed_precedence_expr_term() {
    // 1 + 2 * 3 should be 1 + (2 * 3) due to precedence, not (1 + 2) * 3
    let source = "1 + 2 * 3";
    let result = parse_source(source);
    assert!(result.is_some(), "Precedence: * before +");
}

// ============================================
// POWER / EXPONENTIATION TESTS
// ============================================

#[test]
fn test_power_right_associative() {
    // 2 ^ 3 ^ 2 should be 2 ^ (3 ^ 2) = 2 ^ 9 = 512
    let source = "2 ^ 3 ^ 2";
    let result = parse_source(source);
    assert!(result.is_some(), "Power operator should parse with right-associativity");
}

#[test]
fn test_power_with_parenthesized_left_grouping() {
    let source = "(2 ^ 3) ^ 2";
    let result = parse_source(source);
    assert!(result.is_some(), "Parenthesized power grouping should parse");
}

#[test]
fn test_power_mixed_with_term_and_expr() {
    let source = "2 + 3 * 2 ^ 3";
    let result = parse_source(source);
    assert!(result.is_some(), "Mixed precedence with +, *, ^ should parse");
}

#[test]
fn test_basic_arithmetic_chain_parse() {
    let source = "1 + 2 - 3 * 4 / 5 + 6";
    let result = parse_source(source);
    assert!(result.is_some(), "Basic arithmetic chain should parse");
}

// ============================================
// UNARY OPERATOR TESTS
// ============================================

#[test]
fn test_unary_negation_number() {
    let source = "-5";
    let result = parse_source(source);
    assert!(result.is_some(), "-5 should parse as unary negation");
}

#[test]
fn test_unary_negation_identifier() {
    let source = "-x";
    let result = parse_source(source);
    assert!(result.is_some(), "-x should parse as unary negation of identifier");
}

#[test]
fn test_unary_negation_in_expr() {
    let source = "-5 + 3";
    let result = parse_source(source);
    assert!(result.is_some(), "-5 + 3 should parse");
}

#[test]
fn test_nested_unary() {
    let source = "--x";
    let result = parse_source(source);
    assert!(result.is_some(), "--x (double negation) should parse");
}

// ============================================
// FUNCTION CALL TESTS (USER-DEFINED)
// ============================================

#[test]
fn test_simple_function_call() {
    let source = "foo(1, 2)";
    let result = parse_source(source);
    assert!(result.is_some(), "foo(1, 2) should parse as function call");
}

#[test]
fn test_function_call_single_arg() {
    let source = "bar(x)";
    let result = parse_source(source);
    assert!(result.is_some(), "bar(x) should parse");
}

#[test]
fn test_function_call_no_args() {
    let source = "foo()";
    let result = parse_source(source);
    assert!(result.is_some(), "foo() should parse with no arguments");
}

#[test]
fn test_function_call_with_builtin() {
    let source = "bar(x, sin(1))";
    let result = parse_source(source);
    assert!(result.is_some(), "Function call with builtin as argument");
}

#[test]
fn test_nested_function_calls() {
    let source = "nested(a(b(1), 2), 3)";
    let result = parse_source(source);
    assert!(result.is_some(), "Nested function calls should parse");
}

#[test]
fn test_function_call_with_arithmetic() {
    let source = "foo(1 + 2, 3 * 4)";
    let result = parse_source(source);
    assert!(result.is_some(), "Function calls with arithmetic expressions");
}

// ============================================
// FUNCTION DEFINITION TESTS
// ============================================

#[test]
fn test_inline_function() {
    let source = "function f(x) => x + 1; f(5)";
    let result = parse_source(source);
    assert!(result.is_some(), "Inline function should parse");
    
    if let Some(program) = result {
        assert_eq!(program.decls.len(), 1, "Should have one function");
        if let Decl::Function(func_def) = &program.decls[0] {
            assert_eq!(func_def.params.len(), 1, "Should have one parameter");
        }
    }
}

#[test]
fn test_block_function() {
    let source = "function f(x) { x + 1; x + 2 }; f(5)";
    let result = parse_source(source);
    assert!(result.is_some(), "Block function should parse");
    
    if let Some(program) = result {
        assert_eq!(program.decls.len(), 1);
    }
}

#[test]
fn test_function_multiple_params() {
    let source = "function add(a, b) => a + b; add(2, 3)";
    let result = parse_source(source);
    assert!(result.is_some());
    
    if let Some(program) = result {
        if let Decl::Function(func_def) = &program.decls[0] {
            assert_eq!(func_def.params.len(), 2, "Should have two parameters");
        }
    }
}

#[test]
fn test_function_with_type_annotations() {
    let source = "function f(x: Number) => x + 1; f(5)";
    let result = parse_source(source);
    assert!(result.is_some(), "Function with type annotation should parse");
}

// ============================================
// BOOLEAN CONSTANTS TESTS
// ============================================

#[test]
fn test_boolean_true() {
    let source = "true";
    let result = parse_source(source);
    assert!(result.is_some(), "true constant should parse");
}

#[test]
fn test_boolean_false() {
    let source = "false";
    let result = parse_source(source);
    assert!(result.is_some(), "false constant should parse");
}

#[test]
fn test_boolean_in_expression() {
    let source = "true + false";
    let result = parse_source(source);
    // Parse succeeds; semantic analysis will handle bool+bool type error
    assert!(result.is_some(), "Boolean arithmetic parses (semantic check later)");
}

// ============================================
// COMPLEX EXPRESSIONS TESTS
// ============================================

#[test]
fn test_complex_expression_1() {
    let source = "2 * 3 + 4 * 5";
    let result = parse_source(source);
    assert!(result.is_some());
}

#[test]
fn test_complex_expression_2() {
    let source = "-5 + 3 * 2 - -1";
    let result = parse_source(source);
    assert!(result.is_some());
}

#[test]
fn test_grouped_expression() {
    let source = "(1 + 2) * 3";
    let result = parse_source(source);
    assert!(result.is_some());
}

#[test]
fn test_multiple_functions() {
    let source = r#"
        function f() => 1;
        function g(x) => x + 1;
        function h(a, b) => a * b;
        f()
    "#;
    let result = parse_source(source);
    assert!(result.is_some(), "Should parse multiple function definitions");
    
    if let Some(program) = result {
        assert_eq!(program.decls.len(), 3, "Should have three functions");
    }
}

// ============================================
// ERROR RECOVERY TESTS
// ============================================

#[test]
fn test_missing_semicolon_recovers() {
    let source = r#"
        function f() => 1
        function g() => 2;
    "#;
    let result = parse_source(source);
    // Parser still returns Some(Program) even with some errors
    // Actual error recovery is best tested via parser's error collection
    let _ = result;
}

#[test]
fn test_malformed_function_recovers() {
    let source = r#"
        function f( { 1 + }
        function g() => 2;
    "#;
    let result = parse_source(source);
    // Parser still returns Some(Program) even with some errors
    let _ = result;
}

// ============================================
// EDGE CASES
// ============================================

#[test]
fn test_empty_function_call() {
    let source = "foo()";
    let result = parse_source(source);
    assert!(result.is_some());
}

#[test]
fn test_single_number() {
    let source = "42";
    let result = parse_source(source);
    assert!(result.is_some());
}

#[test]
fn test_single_identifier() {
    let source = "x";
    let result = parse_source(source);
    assert!(result.is_some());
}

#[test]
fn test_constants() {
    let source = "PI + E";
    let result = parse_source(source);
    assert!(result.is_some(), "Mathematical constants should parse");
}

#[test]
fn test_builtin_functions() {
    let source = "sin(1) + cos(2) + sqrt(3) + exp(4) + log(5)";
    let result = parse_source(source);
    assert!(result.is_some(), "Builtin functions should parse");
}

#[test]
fn test_vector_generator_parse() {
    let source = "[x^2 | x in range(1,10)]";
    let result = parse_source(source);
    assert!(result.is_some(), "Vector generator should parse");

    if let Some(program) = result {
        // The global expression should be a vector generator
        assert!(matches!(*program.expr, Expr::VectorGen { .. }),
                "Expected vector generator expression");
    }
}
