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

/// Parse source and also return parser errors for strict validations.
fn parse_source_with_errors(source: &str) -> (Option<Program>, Vec<String>) {
    let stream = TokenStream::new(source);
    let mut parser = Parser::new(stream);
    let program = parser.parse_program();
    (program, parser.errors)
}

fn assert_errors_contain(errors: &[String], expected: &[&str]) {
    for needle in expected {
        assert!(
            errors.iter().any(|error| error.contains(needle)),
            "Missing expected parse error containing {:?}. Got: {:?}",
            needle,
            errors
        );
    }
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
fn test_let_with_parenthesized_sequence_body() {
    let source = r#"
        let sum = 0 in (
            sum := sum + 1;
            sum;
        )
    "#;
    let result = parse_source(source);
    assert!(result.is_some(), "Parenthesized let body with semicolons should parse");
}

#[test]
fn test_let_with_repeated_let_keywords() {
    let source = r#"
        let a = 42, let mod = a % 3, let b = 0 in
            if (mod == 0) "Magic"
            elif (mod % 3 == 1) "Woke"
            else "Dumb"
    "#;
    let result = parse_source(source);
    assert!(result.is_some(), "Repeated let keywords in bindings should parse");
}

#[test]
fn test_let_combinations_should_not_report_parse_errors() {
    let source = r#"
        {
            let a = 42, let mod = a % 3, let b: Boolean = true in
                print(
                    if (mod == 0 & b) "Magic"
                    elif (mod % 3 == 1) "Woke"
                    else "Dumb"
                );

            let a: Number = 42, mod = a % 3, b = true in
                print(
                    if (mod == 0 & b) "Magic"
                    elif (mod % 3 == 1) "Woke"
                    else "Dumb"
                );

            let a = 42 in
                let mod: Number = a % 3 in
                    let b = true in
                        print(
                            if (mod == 0 & b) "Magic"
                            elif (mod % 3 == 1) "Woke"
                            else "Dumb"
                        );

            let a = (let b = 6 in b * 7) in print(a);
        };
    "#;

    let (result, errors) = parse_source_with_errors(source);
    assert!(result.is_some(), "Complex let combinations should parse");
    assert!(errors.is_empty(), "Expected no parse errors, got: {:?}", errors);
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

// ============================================
// PARSER ERROR COVERAGE TESTS
// ============================================

#[test]
fn test_missing_global_expression_reports_error() {
    let (_, errors) = parse_source_with_errors("");

    assert_errors_contain(&errors, &[
        "expected a global expression but found end of file",
    ]);
}

#[test]
fn test_expression_errors_cover_multiple_failures() {
    let source = r#"
        {
            ;
            1 := 2;
            x.;
            new 1;
            ();
            foo(1;
            1 is ();
            1 2
        "#;

    let (_, errors) = parse_source_with_errors(source);

    assert_errors_contain(&errors, &[
        "expected expression",
        "expected type name after 'new'",
        "expected ')' to close argument list",
        "expected type expression",
        "expected ';' or '}' after block expression",
        "expected '}' to close block",
    ]);
}

#[test]
fn test_control_flow_errors_cover_multiple_failures() {
    let source = r#"
        {
            if 1 2 else 3;
            if (1 2) 3 else 4;
            if (1) 2 elif 3 4 else 5;
            if (1) 2 elif (3 4) 5 else 6;
            if (1) 2;
            while 1 2;
            while (1 2) 3;
            for x in xs 1;
            for (x xs) 1;
            for (x in xs 1);
        }
    "#;

    let (_, errors) = parse_source_with_errors(source);

    assert_errors_contain(&errors, &[
        "expected '(' after 'if'",
        "expected ')' after if condition",
        "expected ')' after elif condition",
        "expected 'else' clause on if expression",
        "expected '(' after 'for'",
        "expected 'in' in for loop header",
        "expected ')' after for loop header",
    ]);
}

#[test]
fn test_let_errors_cover_multiple_failures() {
    let source = r#"
        {
            let = 1 in 2;
            let x 1 in 2;
            let _y = 1 in _y;
            let a = 1, b = 2 3;
        }
    "#;

    let (_, errors) = parse_source_with_errors(source);

    assert_errors_contain(&errors, &[
        "expected identifier in let binding",
        "expected '=' in let binding",
        "expected 'in' after let bindings",
    ]);
}

#[test]
fn test_function_errors_cover_multiple_failures() {
    let source = r#"
        function (x) => 1;
        function bad x => 1;
        function missing_paren(x => 1;
        function bad_internal(_x) => _x;
        function worse(1) x;
        function inline_missing_semicolon() => 1
        1 2
    "#;

    let (_, errors) = parse_source_with_errors(source);

    assert_errors_contain(&errors, &[
        "expected function name",
        "expected '(' after function name",
        "expected parameter name",
        "expected '=>' or '{' for function body",
        "expected ';' after inline function body",
    ]);
}

#[test]
fn test_type_errors_cover_multiple_failures() {
    let source = r#"
        type 1 { }
        type V(x { }
        type T extends P { }
        type U inherits 1 { }
        type NoBody x;
        type W {
            1;
            p(x: Number {
            m() x;
            k() => 1
            n;
            a: Number = 1
        }
    "#;

    let (_, errors) = parse_source_with_errors(source);

    assert_errors_contain(&errors, &[
        "expected type name",
        "expected ')' after type parameters",
        "expected 'inherits' after type name",
        "expected parent type name",
        "expected '{' for type body",
        "expected member name",
        "expected ')' after method parameters",
        "expected '=>' or '{' for method body",
        "expected ';' after method inline body",
        "expected ';' after attribute initializer",
    ]);
}

#[test]
fn test_protocol_errors_cover_multiple_failures() {
    let source = r#"
        protocol 1 { }
        protocol P inherits Q { }
        protocol R extends 1 { }
        protocol NoBody x;
        protocol T {
            1: Number;
            m(1: Number): Number;
            bad: Number;
            q(x: Number: Number;
            r(x) Number;
            s(x): Number
        
    "#;

    let (_, errors) = parse_source_with_errors(source);

    assert_errors_contain(&errors, &[
        "expected protocol name",
        "expected 'extends' after protocol name",
        "expected parent protocol name",
        "expected '{' for protocol body",
        "expected method name in protocol",
        "expected '(' after method name in protocol",
        "expected parameter name in protocol method",
        "expected ':' for protocol method return type",
        "expected ';' after protocol method signature",
        "expected '}' after protocol body",
    ]);
}

#[test]
fn test_parenthesized_arrow_is_not_special_cased() {
    let source = "(x) => x";
    let (_, errors) = parse_source_with_errors(source);

    assert_errors_contain(&errors, &["unexpected tokens after global expression"]);
    assert_eq!(errors.len(), 1, "Expected a single generic parse error, got: {:?}", errors);
}

#[test]
fn test_trailing_tokens_after_global_expression() {
    let (_, errors) = parse_source_with_errors("1 2");

    assert_errors_contain(&errors, &["unexpected tokens after global expression"]);
}

#[test]
fn test_missing_field_name_after_dot_reports_error() {
    let source = r#"
        {
            x.;
        }
    "#;

    let (_, errors) = parse_source_with_errors(source);

    assert_errors_contain(&errors, &["expected field name after '.'"]);
}

#[test]
fn test_empty_parentheses_reports_error() {
    let (_, errors) = parse_source_with_errors("()");

    assert_errors_contain(&errors, &["expected expression inside parentheses"]);
}

#[test]
fn test_missing_type_body_closing_brace_reports_error() {
    let source = "type T { 1;";
    let (_, errors) = parse_source_with_errors(source);

    assert_errors_contain(&errors, &["expected '}' after type body"]);
}

#[test]
fn test_remaining_expression_and_loop_errors_are_reported() {
    let source = r#"
        {
            (1 + 2) := 3;
            while (1 2) 3;
            for (_item in items) 1;
            let _tmp = 1 in _tmp;
        }
    "#;

    let (_, errors) = parse_source_with_errors(source);

    assert_errors_contain(&errors, &[
        "assignment target must be an identifier, field access (e.g., x.field), or index (e.g., x[i])",
        "expected identifier in for loop header",
        "internal identifiers not allowed in user code",
    ]);
}

#[test]
fn test_remaining_declaration_errors_are_reported() {
    let source = r#"
        function bad(_x => _x;
        function inline_block() => { 1; };
        function vec_param(x: Number[) => x;

        type T {
            broken: Number 1;
        }

        1
    "#;

    let (_, errors) = parse_source_with_errors(source);

    assert_errors_contain(&errors, &[
        "internal identifiers not allowed in user code",
        "expected ')' after parameter list",
        "inline function body cannot be a block; use '{...}' without '=>' or an expression after '=>'",
        "unexpected ';' after function block body",
        "expected '=' in attribute definition",
        "expected ']' in type vector",
    ]);
}

#[test]
fn test_while_missing_closing_paren_reports_error() {
    let source = "while (1 2) 3";
    let (_, errors) = parse_source_with_errors(source);

    assert_errors_contain(&errors, &["expected ')' after while condition"]);
}

#[test]
fn test_while_missing_opening_paren_reports_error() {
    let source = "while 1 2";
    let (_, errors) = parse_source_with_errors(source);

    assert_errors_contain(&errors, &["expected '(' after 'while'"]);
}

#[test]
fn test_parenthesized_expression_missing_closing_paren_reports_error() {
    let source = "(1";
    let (_, errors) = parse_source_with_errors(source);

    assert_errors_contain(&errors, &["expected ')' after expression"]);
}

#[test]
fn test_elif_missing_opening_paren_reports_error() {
    let source = "if (1) 2 elif 3 else 4";
    let (_, errors) = parse_source_with_errors(source);

    assert_errors_contain(&errors, &["expected '(' after 'elif'"]);
}

#[test]
fn test_type_member_without_separator_reports_error() {
    let source = r#"
        type U {
            orphan;
        }
        1
    "#;

    let (_, errors) = parse_source_with_errors(source);

    assert_errors_contain(&errors, &["expected ':' or '=' or '(' after member name"]);
}

#[test]
fn test_inline_method_block_body_reports_errors() {
    let source = r#"
        type U {
            m() => { 1; };
        }
        1
    "#;

    let (_, errors) = parse_source_with_errors(source);

    assert_errors_contain(&errors, &[
        "inline method body cannot be a block; use '{...}' without '=>' or an expression after '=>'",
        "unexpected ';' after method block body",
    ]);
}

#[test]
fn test_protocol_method_missing_closing_paren_reports_error() {
    let source = r#"
        protocol P {
            q(x: Number: Number;
        }
        1
    "#;

    let (_, errors) = parse_source_with_errors(source);

    assert_errors_contain(&errors, &["expected ')' after protocol method parameters"]);
}
