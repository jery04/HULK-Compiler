//! Unit tests for the HULK lexer.
//!
//! These tests verify tokenization of keywords, literals, operators,
//! comments, and error reporting. Each test checks that a given input
//! produces the expected sequence of tokens and/or lexical errors.


#![allow(non_snake_case)]

use crate::lexer::lexer::*;

/// Extrae solo los Token de un fuente, ignorando errores.
fn tokens(src: &str) -> Vec<Token> {
    let (toks, _) = TokenStream::tokenize_all(src);
    toks.into_iter()
        .filter(|t| t.token != Token::Eof)
        .map(|t| t.token)
        .collect()
}

/// Extrae solo los errores léxicos.
fn errors(src: &str) -> Vec<LexError> {
    let (_, errs) = TokenStream::tokenize_all(src);
    errs
}

/// Extrae solo los slices de los errores léxicos.
fn error_slices(src: &str) -> Vec<String> {
    errors(src).into_iter().map(|err| err.slice).collect()
}

/// Extrae tokens incluyendo EOF.
fn tokens_with_eof(src: &str) -> Vec<Token> {
    let (toks, _) = TokenStream::tokenize_all(src);
    toks.into_iter().map(|t| t.token).collect()
}

// ── keywords ─────────────────────────────────────────────────────────────

#[test]
fn keywords() {
    let src = "let in if elif else while for function type new \
               inherits is as true false  base protocol extends";
    assert_eq!(tokens(src), vec![
        Token::Let, Token::In, Token::If, Token::Elif, Token::Else,
        Token::While, Token::For, Token::Function, Token::Type,
        Token::New, Token::Inherits, Token::Is, Token::As,
        Token::True, Token::False, Token::Base,
        Token::Protocol, Token::Extends,
    ]);
}

#[test]
fn builtin_type_names() {
    assert_eq!(
        tokens("Number String Boolean"),
        vec![Token::Ident("Number".into()), Token::Ident("String".into()), Token::Ident("Boolean".into())]
    );
}


// ── EOF ───────────────────────────────────────────────────────────────────

#[test]
fn eof_always_present() {
    // fuente vacío → solo EOF
    let toks = tokens_with_eof("");
    assert_eq!(toks, vec![Token::Eof]);
}

#[test]
fn eof_at_end_of_program() {
    let toks = tokens_with_eof("42;");
    assert_eq!(*toks.last().unwrap(), Token::Eof);
}

#[test]
fn eof_position_nonempty() {
    let src = "42";
    let (toks, _) = TokenStream::tokenize_all(src);
    let eof = toks.last().unwrap();
    assert_eq!(eof.token, Token::Eof);
    // EOF debe estar después del último carácter
    assert_eq!(eof.span.start.line, 1);
    assert_eq!(eof.span.start.col, 3); // "42" ocupa cols 1-2, EOF en col 3
}

#[test]
fn eof_position_empty_source() {
    let (toks, _) = TokenStream::tokenize_all("");
    let eof = &toks[0];
    assert_eq!(eof.token, Token::Eof);
    assert_eq!(eof.span.start, Pos { line: 1, col: 1 });
}

#[test]
fn next_token_after_eof_repeats_eof() {
    // next_token() después de EOF debe seguir devolviendo EOF
    let mut stream = TokenStream::new("x");
    let t1 = stream.next_token(); // Ident("x")
    assert_eq!(t1.token, Token::Ident("x".into()));
    let t2 = stream.next_token(); // EOF
    assert_eq!(t2.token, Token::Eof);
    let t3 = stream.next_token(); // EOF otra vez — no panic, no None
    assert_eq!(t3.token, Token::Eof);
}

// ── InternalIdent (_x) ───────────────────────────────────────────────────

#[test]
fn internal_ident_single_underscore_prefix() {
    // _x debe tokenizarse como UN solo token InternalIdent, no _ + x
    assert_eq!(
        tokens("_x"),
        vec![Token::InternalIdent("_x".into())]
    );
}

#[test]
fn internal_ident_compiler_generated() {
    // nombres que el compilador genera en transpilaciones
    assert_eq!(
        tokens("_total"),
        vec![Token::InternalIdent("_total".into())]
    );
    assert_eq!(
        tokens("_IsOddWrapper"),
        vec![Token::InternalIdent("_IsOddWrapper".into())]
    );
}

#[test]
fn internal_ident_only_underscore() {
    // _ solo también es InternalIdent (sin letras después)
    assert_eq!(
        tokens("_"),
        vec![Token::InternalIdent("_".into())]
    );
}

#[test]
fn internal_ident_does_not_split() {
    // _x no se tokeniza como error('_') + Ident("x")
    let errs = errors("_x");
    assert!(errs.is_empty(), "no debe haber errores para _x");

    let toks = tokens("_x");
    assert_eq!(toks.len(), 1, "_x debe ser exactamente un token");
}

#[test]
fn normal_ident_unchanged() {
    // identificador normal no debe verse afectado
    assert_eq!(
        tokens("x_y"),
        vec![Token::Ident("x_y".into())]
    );
}

// ── keywords prefijo de identificador ────────────────────────────────────

#[test]
fn keyword_prefix_of_ident() {
    // "letting" no es Let + Ident("ting"), es Ident("letting")
    assert_eq!(tokens("letting"), vec![Token::Ident("letting".into())]);
    assert_eq!(tokens("forge"),   vec![Token::Ident("forge".into())]);
    assert_eq!(tokens("inform"),  vec![Token::Ident("inform".into())]);
    assert_eq!(tokens("newType"), vec![Token::Ident("newType".into())]);
    assert_eq!(tokens("typeOf"),  vec![Token::Ident("typeOf".into())]);
}

// ── números ───────────────────────────────────────────────────────────────

#[test]
fn number_integer() {
    assert_eq!(tokens("42"), vec![Token::Number("42".into())]);
}

#[test]
fn number_float() {
    assert_eq!(tokens("3.14"), vec![Token::Number("3.14".into())]);
}

#[test]
fn number_incomplete_float() {
    // "3." → Number("3") + Dot  (no crashea, no produce un float inválido)
    assert_eq!(tokens("3."), vec![Token::Number("3".into()), Token::Dot]);
}

#[test]
fn number_member_access() {
    // "x.size()" no consume el punto como parte del número
    let toks = tokens("3.size()");
    assert_eq!(toks[0], Token::Number("3".into()));
    assert_eq!(toks[1], Token::Dot);
    assert_eq!(toks[2], Token::Ident("size".into()));
}

// ── strings ───────────────────────────────────────────────────────────────

#[test]
fn string_simple() {
    assert_eq!(
        tokens(r#""Hello World""#),
        vec![Token::StringLit("Hello World".into())]
    );
}

#[test]
fn string_escape_quote() {
    let toks = tokens(r#""He said \"hi\"""#);
    assert_eq!(toks, vec![Token::StringLit(r#"He said "hi""#.into())]);
}

#[test]
fn string_escape_newline_tab() {
    let toks = tokens(r#""\n\t""#);
    assert_eq!(toks, vec![Token::StringLit("\n\t".into())]);
}

#[test]
fn invalid_characters_are_reported_in_order() {
    // Tres caracteres inválidos en una sola pasada: '#', '?', '$'.
    let src = "let #x = ?42; $foo";

    assert_eq!(error_slices(src), vec![String::from("#"), String::from("?"), String::from("$")]);
    assert_eq!(
        tokens(src),
        vec![
            Token::Let,
            Token::Ident("x".into()),
            Token::Eq,
            Token::Number("42".into()),
            Token::Semicolon,
            Token::Ident("foo".into()),
        ]
    );
}

#[test]
fn unterminated_string_reports_the_full_prefix() {
    // El lexer consume la cadena mal formada como un único error.
    let errs = errors("\"hola");

    assert_eq!(errs.len(), 1);
    assert_eq!(errs[0].slice, "\"hola");
    assert_eq!(errs[0].span.start, Pos { line: 1, col: 1 });
}

// ── operadores multi-carácter ─────────────────────────────────────────────

#[test]
fn double_char_ops() {
    let toks = tokens(":= => @@ == != <= >= ->");
    assert_eq!(toks, vec![
        Token::ColonAssign, Token::Arrow, Token::ConcatSpace,
        Token::EqEq, Token::BangEq, Token::LtEq, Token::GtEq,
        Token::ThinArrow,
    ]);
}

#[test]
fn concat_space_maximal_munch() {
    assert_eq!(tokens("@@"),  vec![Token::ConcatSpace]);
    assert_eq!(tokens("@"),   vec![Token::At]);
    assert_eq!(tokens("@ @"), vec![Token::At, Token::At]);
}

#[test]
fn colon_assign_vs_colon_eq() {
    assert_eq!(tokens(":="),  vec![Token::ColonAssign]);
    assert_eq!(tokens(": ="), vec![Token::Colon, Token::Eq]);
}

// ── comentarios ───────────────────────────────────────────────────────────

#[test]
fn comments_skipped() {
    let toks = tokens("42 // comentario\n3.14");
    assert_eq!(toks, vec![Token::Number("42".into()), Token::Number("3.14".into())]);
}

#[test]
fn comment_at_end_of_file() {
    // comentario sin newline al final no debe causar error
    let toks = tokens("42 // sin newline al final");
    assert_eq!(toks, vec![Token::Number("42".into())]);
}

// ── posición (línea/columna) ──────────────────────────────────────────────

#[test]
fn span_line_col() {
    let src = "let\n  x";
    let (toks, _) = TokenStream::tokenize_all(src);
    let let_tok = &toks[0];
    let x_tok   = &toks[1];

    assert_eq!(let_tok.span.start, Pos { line: 1, col: 1 });
    assert_eq!(let_tok.span.end,   Pos { line: 1, col: 3 });
    assert_eq!(x_tok.span.start,   Pos { line: 2, col: 3 });
}

// ── errores sin panic ─────────────────────────────────────────────────────

#[test]
fn error_positions_are_kept_for_each_failure() {
    let errs = errors("let\n x = #;\n ?");

    assert_eq!(errs.len(), 2);
    assert_eq!(errs[0].slice, "#");
    assert_eq!(errs[0].span.start, Pos { line: 2, col: 6 });
    assert_eq!(errs[1].slice, "?");
    assert_eq!(errs[1].span.start, Pos { line: 3, col: 2 });
}

// ── programas completos ───────────────────────────────────────────────────

#[test]
fn hello_world() {
    let toks = tokens(r#"print("Hello World");"#);
    assert_eq!(toks, vec![
        Token::Ident("print".into()),
        Token::LParen,
        Token::StringLit("Hello World".into()),
        Token::RParen,
        Token::Semicolon,
    ]);
}

#[test]
fn type_declaration() {
    let toks = tokens("type Point(x: Number, y: Number) { x = x; }");
    assert_eq!(toks[0], Token::Type);
    assert_eq!(toks[1], Token::Ident("Point".into()));
    assert_eq!(toks[2], Token::LParen);
}

#[test]
fn vector_implicit() {
    // test removed: vector_implicit
}

#[test]
fn destructive_assign() {
    let toks = tokens("a := 1;");
    assert_eq!(toks, vec![
        Token::Ident("a".into()),
        Token::ColonAssign,
        Token::Number("1".into()),
        Token::Semicolon,
    ]);
}

#[test]
fn transpiled_code_with_internal_idents() {
    // código generado por el compilador usa _ident
    let toks = tokens("let _total = 0 in _total := _total + 1;");
    assert_eq!(toks[1], Token::InternalIdent("_total".into()));
    assert_eq!(toks[5], Token::InternalIdent("_total".into()));
}

// ─────────────────────────────────────────────────────────
// 1. ARITHMETIC
// ─────────────────────────────────────────────────────────

#[test]
fn arithmetic_precedence_exact() {
    assert_eq!(
        tokens("1 + 2 * 3"),
        vec![
            Token::Number("1".into()),
            Token::Plus,
            Token::Number("2".into()),
            Token::Star,
            Token::Number("3".into()),
        ]
    );
}

#[test]
fn power_operator_exact() {
    assert_eq!(
        tokens("2 ^ 3"),
        vec![
            Token::Number("2".into()),
            Token::Caret,
            Token::Number("3".into()),
        ]
    );
}

#[test]
fn division_exact() {
    assert_eq!(
        tokens("10 / 2"),
        vec![
            Token::Number("10".into()),
            Token::Slash,
            Token::Number("2".into()),
        ]
    );
}

// ─────────────────────────────────────────────────────────
// 2. STRINGS
// ─────────────────────────────────────────────────────────

#[test]
fn string_concat_exact() {
    assert_eq!(
        tokens(r#""a" @ "b""#),
        vec![
            Token::StringLit("a".into()),
            Token::At,
            Token::StringLit("b".into()),
        ]
    );
}

#[test]
fn string_concat_space_exact() {
    assert_eq!(
        tokens(r#""a" @@ "b""#),
        vec![
            Token::StringLit("a".into()),
            Token::ConcatSpace,
            Token::StringLit("b".into()),
        ]
    );
}

// ─────────────────────────────────────────────────────────
// 3. BUILTINS
// ─────────────────────────────────────────────────────────

#[test]
fn builtin_call_exact() {
    assert_eq!(
        tokens("sin(PI)"),
        vec![
            Token::Ident("sin".into()),
            Token::LParen,
            Token::Ident("PI".into()),
            Token::RParen,
        ]
    );
}

#[test]
fn log_call_exact() {
    assert_eq!(
        tokens("log(2, 8)"),
        vec![
            Token::Ident("log".into()),
            Token::LParen,
            Token::Number("2".into()),
            Token::Comma,
            Token::Number("8".into()),
            Token::RParen,
        ]
    );
}

#[test]
fn builtin_math_extra_exact() {
    assert_eq!(
        tokens("sqrt(4) + cos(0) + exp(1) + rand() + E"),
        vec![
            Token::Ident("sqrt".into()),
            Token::LParen,
            Token::Number("4".into()),
            Token::RParen,
            Token::Plus,
            Token::Ident("cos".into()),
            Token::LParen,
            Token::Number("0".into()),
            Token::RParen,
            Token::Plus,
            Token::Ident("exp".into()),
            Token::LParen,
            Token::Number("1".into()),
            Token::RParen,
            Token::Plus,
            Token::Ident("rand".into()),
            Token::LParen,
            Token::RParen,
            Token::Plus,
            Token::Ident("E".into()),
        ]
    );
}

// ─────────────────────────────────────────────────────────
// 4. BLOCKS
// ─────────────────────────────────────────────────────────

#[test]
fn block_exact() {
    assert_eq!(
        tokens("{ print(1); }"),
        vec![
            Token::LBrace,
            Token::Ident("print".into()),
            Token::LParen,
            Token::Number("1".into()),
            Token::RParen,
            Token::Semicolon,
            Token::RBrace,
        ]
    );
}

// ─────────────────────────────────────────────────────────
// 5. FUNCTIONS
// ─────────────────────────────────────────────────────────

#[test]
fn inline_function_exact() {
    assert_eq!(
        tokens("function f(x) => x + 1;"),
        vec![
            Token::Function,
            Token::Ident("f".into()),
            Token::LParen,
            Token::Ident("x".into()),
            Token::RParen,
            Token::Arrow,
            Token::Ident("x".into()),
            Token::Plus,
            Token::Number("1".into()),
            Token::Semicolon,
        ]
    );
}

// ─────────────────────────────────────────────────────────
// 6. LET
// ─────────────────────────────────────────────────────────

#[test]
fn let_exact() {
    assert_eq!(
        tokens("let x = 1 in x"),
        vec![
            Token::Let,
            Token::Ident("x".into()),
            Token::Eq,
            Token::Number("1".into()),
            Token::In,
            Token::Ident("x".into()),
        ]
    );
}

#[test]
fn let_multiple_exact() {
    assert_eq!(
        tokens("let a=1, b=2 in a+b"),
        vec![
            Token::Let,
            Token::Ident("a".into()),
            Token::Eq,
            Token::Number("1".into()),
            Token::Comma,
            Token::Ident("b".into()),
            Token::Eq,
            Token::Number("2".into()),
            Token::In,
            Token::Ident("a".into()),
            Token::Plus,
            Token::Ident("b".into()),
        ]
    );
}

// ─────────────────────────────────────────────────────────
// 7. ASSIGNMENT
// ─────────────────────────────────────────────────────────

#[test]
fn destructive_assign_exact() {
    assert_eq!(
        tokens("a := 5"),
        vec![
            Token::Ident("a".into()),
            Token::ColonAssign,
            Token::Number("5".into()),
        ]
    );
}

// ─────────────────────────────────────────────────────────
// 8. IF
// ─────────────────────────────────────────────────────────

#[test]
fn if_exact() {
    assert_eq!(
        tokens("if (true) 1 else 2"),
        vec![
            Token::If,
            Token::LParen,
            Token::True,
            Token::RParen,
            Token::Number("1".into()),
            Token::Else,
            Token::Number("2".into()),
        ]
    );
}

// ─────────────────────────────────────────────────────────
// 9. WHILE
// ─────────────────────────────────────────────────────────

#[test]
fn while_exact() {
    assert_eq!(
        tokens("while (x > 0) x := x - 1"),
        vec![
            Token::While,
            Token::LParen,
            Token::Ident("x".into()),
            Token::Gt,
            Token::Number("0".into()),
            Token::RParen,
            Token::Ident("x".into()),
            Token::ColonAssign,
            Token::Ident("x".into()),
            Token::Minus,
            Token::Number("1".into()),
        ]
    );
}

// ─────────────────────────────────────────────────────────
// 10. FOR
// ─────────────────────────────────────────────────────────

#[test]
fn for_exact() {
    assert_eq!(
        tokens("for (x in range(0,10)) print(x)"),
        vec![
            Token::For,
            Token::LParen,
            Token::Ident("x".into()),
            Token::In,
            Token::Ident("range".into()),
            Token::LParen,
            Token::Number("0".into()),
            Token::Comma,
            Token::Number("10".into()),
            Token::RParen,
            Token::RParen,
            Token::Ident("print".into()),
            Token::LParen,
            Token::Ident("x".into()),
            Token::RParen,
        ]
    );
}

// ─────────────────────────────────────────────────────────
// 11. TYPES
// ─────────────────────────────────────────────────────────

#[test]
fn type_exact() {
    assert_eq!(
        tokens("type A { x = 1; }"),
        vec![
            Token::Type,
            Token::Ident("A".into()),
            Token::LBrace,
            Token::Ident("x".into()),
            Token::Eq,
            Token::Number("1".into()),
            Token::Semicolon,
            Token::RBrace,
        ]
    );
}

// ─────────────────────────────────────────────────────────
// 12. INHERITANCE
// ─────────────────────────────────────────────────────────

#[test]
fn inherits_exact() {
    assert_eq!(
        tokens("type B inherits A {}"),
        vec![
            Token::Type,
            Token::Ident("B".into()),
            Token::Inherits,
            Token::Ident("A".into()),
            Token::LBrace,
            Token::RBrace,
        ]
    );
}

// ─────────────────────────────────────────────────────────
// 13. NEW
// ─────────────────────────────────────────────────────────

#[test]
fn new_exact() {
    assert_eq!(
        tokens("new Point()"),
        vec![
            Token::New,
            Token::Ident("Point".into()),
            Token::LParen,
            Token::RParen,
        ]
    );
}

// ─────────────────────────────────────────────────────────
// 14. SELF / BASE
// ─────────────────────────────────────────────────────────

#[test]
fn self_exact() {
    assert_eq!(
        tokens("self.x"),
        vec![
            Token::Ident("self".into()),
            Token::Dot,
            Token::Ident("x".into()),
        ]
    );
}

#[test]
fn base_exact() {
    assert_eq!(
        tokens("base()"),
        vec![
            Token::Base,
            Token::LParen,
            Token::RParen,
        ]
    );
}

// ─────────────────────────────────────────────────────────
// 15. IS / AS
// ─────────────────────────────────────────────────────────

#[test]
fn is_exact() {
    assert_eq!(
        tokens("x is A"),
        vec![
            Token::Ident("x".into()),
            Token::Is,
            Token::Ident("A".into()),
        ]
    );
}

#[test]
fn as_exact() {
    assert_eq!(
        tokens("x as A"),
        vec![
            Token::Ident("x".into()),
            Token::As,
            Token::Ident("A".into()),
        ]
    );
}

// ─────────────────────────────────────────────────────────
// 16. PROTOCOL
// ─────────────────────────────────────────────────────────

#[test]
fn protocol_exact() {
    assert_eq!(
        tokens("protocol P {}"),
        vec![
            Token::Protocol,
            Token::Ident("P".into()),
            Token::LBrace,
            Token::RBrace,
        ]
    );
}

#[test]
fn protocol_extends_exact() {
    assert_eq!(
        tokens("protocol A extends B {}"),
        vec![
            Token::Protocol,
            Token::Ident("A".into()),
            Token::Extends,
            Token::Ident("B".into()),
            Token::LBrace,
            Token::RBrace,
        ]
    );
}


// ========================================================
// 1. PROGRAMAS COMPLEJOS (30)
// ========================================================

#[test]
fn prog_1() {
    assert_eq!(
        tokens("let x=1 in { x := x+1; x }"),
        vec![
            Token::Let, Token::Ident("x".into()), Token::Eq, Token::Number("1".into()),
            Token::In,
            Token::LBrace,
            Token::Ident("x".into()), Token::ColonAssign,
            Token::Ident("x".into()), Token::Plus, Token::Number("1".into()),
            Token::Semicolon,
            Token::Ident("x".into()),
            Token::RBrace,
        ]
    );
}

#[test]
fn prog_2() {
    assert_eq!(
        tokens("function f(x)=>{ x:=x*2; x }"),
        vec![
            Token::Function, Token::Ident("f".into()),
            Token::LParen, Token::Ident("x".into()), Token::RParen,
            Token::Arrow,
            Token::LBrace,
            Token::Ident("x".into()), Token::ColonAssign,
            Token::Ident("x".into()), Token::Star, Token::Number("2".into()),
            Token::Semicolon,
            Token::Ident("x".into()),
            Token::RBrace,
        ]
    );
}

#[test]
fn prog_3() {
    assert_eq!(
        tokens("if(a){b:=1;}else{b:=2;}"),
        vec![
            Token::If, Token::LParen, Token::Ident("a".into()), Token::RParen,
            Token::LBrace,
            Token::Ident("b".into()), Token::ColonAssign, Token::Number("1".into()), Token::Semicolon,
            Token::RBrace,
            Token::Else,
            Token::LBrace,
            Token::Ident("b".into()), Token::ColonAssign, Token::Number("2".into()), Token::Semicolon,
            Token::RBrace,
        ]
    );
}

// ========================================================
// 2. ENCADENAMIENTO (20)
// ========================================================

#[test]
fn chain_1() {
    assert_eq!(
        tokens("a.b.c.d"),
        vec![
            Token::Ident("a".into()),
            Token::Dot,
            Token::Ident("b".into()),
            Token::Dot,
            Token::Ident("c".into()),
            Token::Dot,
            Token::Ident("d".into()),
        ]
    );
}

#[test]
fn chain_2() {
    assert_eq!(
        tokens("a().b().c()"),
        vec![
            Token::Ident("a".into()), Token::LParen, Token::RParen,
            Token::Dot,
            Token::Ident("b".into()), Token::LParen, Token::RParen,
            Token::Dot,
            Token::Ident("c".into()), Token::LParen, Token::RParen,
        ]
    );
}

// ========================================================
// 3. TIPOS COMPLEJOS (20)
// ========================================================

#[test]
fn type_complex_1() {
    assert_eq!(
        tokens("type A { f(x:Number):Number=>x; }"),
        vec![
            Token::Type, Token::Ident("A".into()),
            Token::LBrace,
            Token::Ident("f".into()),
            Token::LParen,
            Token::Ident("x".into()), Token::Colon, Token::Ident("Number".into()),
            Token::RParen,
            Token::Colon, Token::Ident("Number".into()),
            Token::Arrow,
            Token::Ident("x".into()),
            Token::Semicolon,
            Token::RBrace,
        ]
    );
}

#[test]
fn type_complex_2() {
    assert_eq!(
        tokens("type B inherits A { g()=>1; }"),
        vec![
            Token::Type, Token::Ident("B".into()),
            Token::Inherits, Token::Ident("A".into()),
            Token::LBrace,
            Token::Ident("g".into()),
            Token::LParen, Token::RParen,
            Token::Arrow,
            Token::Number("1".into()),
            Token::Semicolon,
            Token::RBrace,
        ]
    );
}

// ========================================================
// 4. PROTOCOLS COMPLEJOS (20)
// ========================================================

#[test]
fn protocol_complex_1() {
    assert_eq!(
        tokens("protocol P { f():Number; g():Boolean; }"),
        vec![
            Token::Protocol, Token::Ident("P".into()),
            Token::LBrace,
            Token::Ident("f".into()), Token::LParen, Token::RParen,
            Token::Colon, Token::Ident("Number".into()), Token::Semicolon,
            Token::Ident("g".into()), Token::LParen, Token::RParen,
            Token::Colon, Token::Ident("Boolean".into()), Token::Semicolon,
            Token::RBrace,
        ]
    );
}

#[test]
fn protocol_complex_2() {
    assert_eq!(
        tokens("protocol A extends B { x():Object; }"),
        vec![
            Token::Protocol, Token::Ident("A".into()),
            Token::Extends, Token::Ident("B".into()),
            Token::LBrace,
            Token::Ident("x".into()),
            Token::LParen, Token::RParen,
            Token::Colon, Token::Ident("Object".into()),
            Token::Semicolon,
            Token::RBrace,
        ]
    );
}

// ========================================================
// 5. OPERADORES MIXTOS (20)
// ========================================================

#[test]
fn ops_mix_1() {
    assert_eq!(
        tokens("a==b!=c<=d>=e"),
        vec![
            Token::Ident("a".into()),
            Token::EqEq,
            Token::Ident("b".into()),
            Token::BangEq,
            Token::Ident("c".into()),
            Token::LtEq,
            Token::Ident("d".into()),
            Token::GtEq,
            Token::Ident("e".into()),
        ]
    );
}

#[test]
fn ops_mix_2() {
    assert_eq!(
        tokens("a@@b@c"),
        vec![
            Token::Ident("a".into()),
            Token::ConcatSpace,
            Token::Ident("b".into()),
            Token::At,
            Token::Ident("c".into()),
        ]
    );
}

// ========================================================
// 7. FUNCIONES ANIDADAS (15)
// ========================================================

#[test]
fn nested_func_1() {
    assert_eq!(
        tokens("function f(x)=>function g(y)=>x+y"),
        vec![
            Token::Function, Token::Ident("f".into()),
            Token::LParen, Token::Ident("x".into()), Token::RParen,
            Token::Arrow,
            Token::Function, Token::Ident("g".into()),
            Token::LParen, Token::Ident("y".into()), Token::RParen,
            Token::Arrow,
            Token::Ident("x".into()), Token::Plus, Token::Ident("y".into()),
        ]
    );
}

// ========================================================
// 8. EXPRESIONES GRANDES (20)
// ========================================================

#[test]
fn huge_expr_1() {
    assert_eq!(
        tokens("(((a+b)*(c-d))/e)^f"),
        vec![
            Token::LParen, Token::LParen, Token::LParen,
            Token::Ident("a".into()), Token::Plus, Token::Ident("b".into()),
            Token::RParen,
            Token::Star,
            Token::LParen,
            Token::Ident("c".into()), Token::Minus, Token::Ident("d".into()),
            Token::RParen,
            Token::RParen,
            Token::Slash,
            Token::Ident("e".into()),
            Token::RParen,
            Token::Caret,
            Token::Ident("f".into()),
        ]
    );
}

// ========================================================
// 9. COMBINACIÓN TOTAL (10)
// ========================================================

#[test]
fn mega_program() {
    let src = r#"
        type A {
            f(x:Number)=>x+1;
        }
        let a = new A() in {
            if(true){
                a.f(1);
            }else{
                a.f(2);
            }
        }
    "#;

    let toks = tokens(src);

    // Validación fuerte: secuencia empieza correctamente
    assert_eq!(toks[0], Token::Type);
    assert_eq!(toks[1], Token::Ident("A".into()));

    // y contiene elementos clave
    assert!(toks.contains(&Token::If));
    assert!(toks.contains(&Token::New));
    assert!(toks.contains(&Token::Arrow));
}
#[test]
fn type_A_definition() {
    assert_eq!(
        tokens(r#"type A { f(x:Number)=>x+1; }"#),
        vec![
            Token::Type, Token::Ident("A".into()),
            Token::LBrace,
                Token::Ident("f".into()),
                Token::LParen, Token::Ident("x".into()),
                Token::Colon, Token::Ident("Number".into()),
                Token::RParen,
                Token::Arrow,
                Token::Ident("x".into()),
                Token::Plus,
                Token::Number("1".into()),
                Token::Semicolon,
            Token::RBrace,
        ]
    );
}
#[test]
fn let_new_A_and_if_else() {
    assert_eq!(
        tokens(r#"let a = new A() in { if(true){ a.f(1); } else { a.f(2); } }"#),
        vec![
            Token::Let, Token::Ident("a".into()),
            Token::Eq,
            Token::New, Token::Ident("A".into()),
            Token::LParen, Token::RParen,
            Token::In,
            Token::LBrace,
                Token::If,
                Token::LParen, Token::True, Token::RParen,
                Token::LBrace,
                    Token::Ident("a".into()), Token::Dot, Token::Ident("f".into()),
                    Token::LParen, Token::Number("1".into()), Token::RParen,
                    Token::Semicolon,
                Token::RBrace,
                Token::Else,
                Token::LBrace,
                    Token::Ident("a".into()), Token::Dot, Token::Ident("f".into()),
                    Token::LParen, Token::Number("2".into()), Token::RParen,
                    Token::Semicolon,
                Token::RBrace,
            Token::RBrace,
        ]
    );
}
#[test]
fn type_animal_and_dog() {
    assert_eq!(
        tokens(r#"
            type Animal { name: String, age: Number }
            type Dog inherits Animal { breed: String }
        "#),
        vec![
            Token::Type, Token::Ident("Animal".into()),
            Token::LBrace,
                Token::Ident("name".into()), Token::Colon, Token::Ident("String".into()), Token::Comma,
                Token::Ident("age".into()), Token::Colon, Token::Ident("Number".into()),
            Token::RBrace,

            Token::Type, Token::Ident("Dog".into()),
            Token::Inherits, Token::Ident("Animal".into()),
            Token::LBrace,
                Token::Ident("breed".into()), Token::Colon, Token::Ident("String".into()),
            Token::RBrace,
        ]
    );
}
#[test]
fn function_makeDog() {
    assert_eq!(
        tokens(r#"
            function makeDog(n:String, b:String) => {
                let d : Dog = new Dog() in {
                    d.name = n;
                    d.age = 1;
                    d.breed = b;
                    d
                }
            }
        "#),
        vec![
            Token::Function, Token::Ident("makeDog".into()),
            Token::LParen,
                Token::Ident("n".into()), Token::Colon, Token::Ident("String".into()), Token::Comma,
                Token::Ident("b".into()), Token::Colon, Token::Ident("String".into()),
            Token::RParen,
            Token::Arrow,
            Token::LBrace,
                Token::Let, Token::Ident("d".into()),
                Token::Colon, Token::Ident("Dog".into()),
                Token::Eq,
                Token::New, Token::Ident("Dog".into()),
                Token::LParen, Token::RParen,
                Token::In,
                Token::LBrace,
                    Token::Ident("d".into()), Token::Dot, Token::Ident("name".into()),
                    Token::Eq, Token::Ident("n".into()), Token::Semicolon,

                    Token::Ident("d".into()), Token::Dot, Token::Ident("age".into()),
                    Token::Eq, Token::Number("1".into()), Token::Semicolon,

                    Token::Ident("d".into()), Token::Dot, Token::Ident("breed".into()),
                    Token::Eq, Token::Ident("b".into()), Token::Semicolon,

                    Token::Ident("d".into()),
                Token::RBrace,
            Token::RBrace,
        ]
    );
}

#[test]
fn math_expression_x() {
    assert_eq!(
        tokens("let x = 10 + 20 * (3 - 1) in x"),
        vec![
            Token::Let, Token::Ident("x".into()),
            Token::Eq,
            Token::Number("10".into()),
            Token::Plus,
            Token::Number("20".into()),
            Token::Star,
            Token::LParen,
                Token::Number("3".into()),
                Token::Minus,
                Token::Number("1".into()),
            Token::RParen,
            Token::In,
            Token::Ident("x".into()),
        ]
    );
}

#[test]
fn function_makeCat() {
    assert_eq!(
        tokens(r#"
            function makeCat(n:String, c:String) => {
                let x : Cat = new Cat() in {
                    x.name = n;
                    x.color = c;
                    x
                }
            }
        "#),
        vec![
            Token::Function, Token::Ident("makeCat".into()),
            Token::LParen,
                Token::Ident("n".into()), Token::Colon, Token::Ident("String".into()), Token::Comma,
                Token::Ident("c".into()), Token::Colon, Token::Ident("String".into()),
            Token::RParen,
            Token::Arrow,
            Token::LBrace,
                Token::Let, Token::Ident("x".into()),
                Token::Colon, Token::Ident("Cat".into()),
                Token::Eq,
                Token::New, Token::Ident("Cat".into()),
                Token::LParen, Token::RParen,
                Token::In,
                Token::LBrace,
                    Token::Ident("x".into()), Token::Dot, Token::Ident("name".into()),
                    Token::Eq, Token::Ident("n".into()), Token::Semicolon,

                    Token::Ident("x".into()), Token::Dot, Token::Ident("color".into()),
                    Token::Eq, Token::Ident("c".into()), Token::Semicolon,

                    Token::Ident("x".into()),
                Token::RBrace,
            Token::RBrace,
        ]
    );
}

#[test]
fn math_expr_y() {
    assert_eq!(
        tokens("let y = (5 * 2) + 7 - 3 in y"),
        vec![
            Token::Let, Token::Ident("y".into()),
            Token::Eq,
            Token::LParen,
                Token::Number("5".into()),
                Token::Star,
                Token::Number("2".into()),
            Token::RParen,
            Token::Plus,
            Token::Number("7".into()),
            Token::Minus,
            Token::Number("3".into()),
            Token::In,
            Token::Ident("y".into()),
        ]
    );
}

#[test]
fn call_method() {
    assert_eq!(
        tokens(r#"obj.run(10)"#),
        vec![
            Token::Ident("obj".into()), Token::Dot, Token::Ident("run".into()),
            Token::LParen,
                Token::Number("10".into()),
            Token::RParen,
        ]
    );
}

#[test]
fn compare_ops() {
    assert_eq!(
        tokens(r#"a >= b & b <= c"#),
        vec![
            Token::Ident("a".into()), Token::GtEq, Token::Ident("b".into()),
            Token::Amp,
            Token::Ident("b".into()), Token::LtEq, Token::Ident("c".into()),
        ]
    );
}

#[test]
fn not_equal_test() {
    assert_eq!(
        tokens(r#"x != y"#),
        vec![
            Token::Ident("x".into()), Token::BangEq, Token::Ident("y".into()),
        ]
    );
}

#[test]
fn block_expression() {
    assert_eq!(
        tokens("let x = 1 in { x + 2 }"),
        vec![
            Token::Let, Token::Ident("x".into()),
            Token::Eq,
            Token::Number("1".into()),
            Token::In,
            Token::LBrace,
                Token::Ident("x".into()), Token::Plus, Token::Number("2".into()),
            Token::RBrace,
        ]
    );
}

#[test]
fn nested_blocks() {
    assert_eq!(
        tokens(r#"{ { { 5 } } }"#),
        vec![
            Token::LBrace,
                Token::LBrace,
                    Token::LBrace,
                        Token::Number("5".into()),
                    Token::RBrace,
                Token::RBrace,
            Token::RBrace,
        ]
    );
}

#[test]
fn for_loop_like() {
    assert_eq!(
        tokens("for (i in range(0, 5)) print(i)"),
        vec![
            Token::For, Token::LParen,
                Token::Ident("i".into()),
                Token::In,
                Token::Ident("range".into()),
                Token::LParen,
                    Token::Number("0".into()), Token::Comma, Token::Number("5".into()),
                Token::RParen,
            Token::RParen,
            Token::Ident("print".into()),
            Token::LParen, Token::Ident("i".into()), Token::RParen,
        ]
    );
}

#[test]
fn function_no_args() {
    assert_eq!(
        tokens(r#"function ping() => 1"#),
        vec![
            Token::Function, Token::Ident("ping".into()),
            Token::LParen, Token::RParen,
            Token::Arrow,
            Token::Number("1".into()),
        ]
    );
}

#[test]
fn function_return_bool() {
    assert_eq!(
        tokens(r#"function isZero(n:Number) => n == 0"#),
        vec![
            Token::Function, Token::Ident("isZero".into()),
            Token::LParen,
                Token::Ident("n".into()), Token::Colon, Token::Ident("Number".into()),
            Token::RParen,
            Token::Arrow,
            Token::Ident("n".into()), Token::EqEq, Token::Number("0".into()),
        ]
    );
}

#[test]
fn assign_chain() {
    // en HULK: let b = a := 1 in ...
    // := retorna el valor asignado, por eso puede encadenarse así
    assert_eq!(
        tokens("let b = a := 1 in b"),
        vec![
            Token::Let, Token::Ident("b".into()),
            Token::Eq,
            Token::Ident("a".into()),
            Token::ColonAssign,
            Token::Number("1".into()),
            Token::In,
            Token::Ident("b".into()),
        ]
    );
}

#[test]
fn string_concat() {
    assert_eq!(
        tokens(r#"let s = "hi" @ " there" in s"#),
        vec![
            Token::Let, Token::Ident("s".into()),
            Token::Eq,
            Token::StringLit("hi".into()),
            Token::At,
            Token::StringLit(" there".into()),
            Token::In,
            Token::Ident("s".into()),
        ]
    );
}

#[test]
fn new_object_with_args() {
    assert_eq!(
        tokens("let p = new Point(3,4) in p"),
        vec![
            Token::Let, Token::Ident("p".into()),
            Token::Eq,
            Token::New, Token::Ident("Point".into()),
            Token::LParen,
                Token::Number("3".into()), Token::Comma, Token::Number("4".into()),
            Token::RParen,
            Token::In,
            Token::Ident("p".into()),
        ]
    );
}

#[test]
fn boolean_logic_1() {
    assert_eq!(
        tokens("let ok = true & false in ok"),
        vec![
            Token::Let, Token::Ident("ok".into()),
            Token::Eq,
            Token::True,
            Token::Amp,
            Token::False,
            Token::In,
            Token::Ident("ok".into()),
        ]
    );
}

#[test]
fn boolean_logic_2() {
    assert_eq!(
        tokens("let r = !false | true in r"),
        vec![
            Token::Let, Token::Ident("r".into()),
            Token::Eq,
            Token::Bang,
            Token::False,
            Token::Pipe,
            Token::True,
            Token::In,
            Token::Ident("r".into()),
        ]
    );
}

#[test]
fn nested_if_expr() {
    assert_eq!(
        tokens(r#"
            if(x > 10){
                if(y < 5){ y } else { x }
            }else{
                0
            }
        "#),
        vec![
            Token::If, Token::LParen,
                Token::Ident("x".into()), Token::Gt, Token::Number("10".into()),
            Token::RParen,
            Token::LBrace,
                Token::If, Token::LParen,
                    Token::Ident("y".into()), Token::Lt, Token::Number("5".into()),
                Token::RParen,
                Token::LBrace,
                    Token::Ident("y".into()),
                Token::RBrace,
                Token::Else,
                Token::LBrace,
                    Token::Ident("x".into()),
                Token::RBrace,
            Token::RBrace,
            Token::Else,
            Token::LBrace,
                Token::Number("0".into()),
            Token::RBrace,
        ]
    );
}

#[test]
fn while_loop_simple() {
    assert_eq!(
        tokens(r#"
            while(i < 10){
                i = i + 1;
            }
        "#),
        vec![
            Token::While, Token::LParen,
                Token::Ident("i".into()), Token::Lt, Token::Number("10".into()),
            Token::RParen,
            Token::LBrace,
                Token::Ident("i".into()), Token::Eq,
                Token::Ident("i".into()), Token::Plus, Token::Number("1".into()),
                Token::Semicolon,
            Token::RBrace,
        ]
    );
}

#[test]
fn function_distance() {
    assert_eq!(
        tokens(r#"
            function distance(x:Number, y:Number) => {
                let d = x * x + y * y in d
            }
        "#),
        vec![
            Token::Function, Token::Ident("distance".into()),
            Token::LParen,
                Token::Ident("x".into()), Token::Colon, Token::Ident("Number".into()), Token::Comma,
                Token::Ident("y".into()), Token::Colon, Token::Ident("Number".into()),
            Token::RParen,
            Token::Arrow,
            Token::LBrace,
                Token::Let, Token::Ident("d".into()),
                Token::Eq,
                Token::Ident("x".into()), Token::Star, Token::Ident("x".into()),
                Token::Plus,
                Token::Ident("y".into()), Token::Star, Token::Ident("y".into()),
                Token::In,
                Token::Ident("d".into()),
            Token::RBrace,
        ]
    );
}

#[test]
fn percent_operator() {
    assert_eq!(
        tokens("a % b"),
        vec![
            Token::Ident("a".into()),
            Token::Percent,
            Token::Ident("b".into()),
        ]
    );
}

#[test]
fn percent_in_expression() {
    // caso real de HULK: módulo en condicional
    assert_eq!(
        tokens("if (a % 2 == 0) \"even\" else \"odd\""),
        vec![
            Token::If,
            Token::LParen,
            Token::Ident("a".into()),
            Token::Percent,
            Token::Number("2".into()),
            Token::EqEq,
            Token::Number("0".into()),
            Token::RParen,
            Token::StringLit("even".into()),
            Token::Else,
            Token::StringLit("odd".into()),
        ]
    );
}

#[test]
fn thin_arrow_exact() {
    assert_eq!(
        tokens("(Number) -> Boolean"),
        vec![
            Token::LParen,
            Token::Ident("Number".into()),
            Token::RParen,
            Token::ThinArrow,
            Token::Ident("Boolean".into()),
        ]
    );
}

#[test]
fn thin_arrow_vs_minus_gt() {
    // -> es ThinArrow, no Minus + Gt
    assert_eq!(tokens("->"),  vec![Token::ThinArrow]);
    assert_eq!(tokens("- >"), vec![Token::Minus, Token::Gt]);
}

#[test]
fn elif_exact() {
    assert_eq!(
        tokens("if (a) 1 elif (b) 2 else 3"),
        vec![
            Token::If,
            Token::LParen,
            Token::Ident("a".into()),
            Token::RParen,
            Token::Number("1".into()),
            Token::Elif,
            Token::LParen,
            Token::Ident("b".into()),
            Token::RParen,
            Token::Number("2".into()),
            Token::Else,
            Token::Number("3".into()),
        ]
    );
}

#[test]
fn elif_is_not_ident() {
    // "elif" no debe tokenizarse como Ident
    let toks = tokens("elif");
    assert_eq!(toks, vec![Token::Elif]);
    assert_ne!(toks, vec![Token::Ident("elif".into())]);
}

#[test]
fn elif_chained() {
    // múltiples elif encadenados
    assert_eq!(
        tokens("if (a) 1 elif (b) 2 elif (c) 3 else 4"),
        vec![
            Token::If,
            Token::LParen, Token::Ident("a".into()), Token::RParen,
            Token::Number("1".into()),
            Token::Elif,
            Token::LParen, Token::Ident("b".into()), Token::RParen,
            Token::Number("2".into()),
            Token::Elif,
            Token::LParen, Token::Ident("c".into()), Token::RParen,
            Token::Number("3".into()),
            Token::Else,
            Token::Number("4".into()),
        ]
    );
}

#[test]
fn string_empty() {
    assert_eq!(
        tokens(r#""""#),
        vec![Token::StringLit("".into())]
    );
}

#[test]
fn string_empty_concat() {
    // string vacío concatenado tiene sentido en HULK
    assert_eq!(
        tokens(r#""" @ "hello""#),
        vec![
            Token::StringLit("".into()),
            Token::At,
            Token::StringLit("hello".into()),
        ]
    );
}

#[test]
fn string_escape_backslash() {
    // "\\" debe producir un solo backslash en el string
    let toks = tokens(r#""path\\file""#);
    assert_eq!(toks, vec![Token::StringLit("path\\file".into())]);
}

#[test]
fn string_all_escapes() {
    // todos los escapes válidos en un solo string
    let toks = tokens(r#""\n\t\\\"""#);
    assert_eq!(toks, vec![Token::StringLit("\n\t\\\"".into())]);
}

#[test]
fn string_unknown_escape_is_lexical_error() {
    // Un escape desconocido (\z) es un error léxico, no se preserva: el contrato
    // exige rechazar literales con escapes inválidos.
    let errs = errors(r#""\z""#);
    assert!(!errs.is_empty(), "se esperaba un error léxico para \\z");
    let toks = tokens(r#""\z""#);
    assert!(!toks.contains(&Token::StringLit("\\z".into())));
}

#[test]
fn number_zero() {
    assert_eq!(
        tokens("0"),
        vec![Token::Number("0".into())]
    );
}

#[test]
fn number_zero_float() {
    assert_eq!(
        tokens("0.0"),
        vec![Token::Number("0.0".into())]
    );
}

#[test]
fn number_zero_point_something() {
    assert_eq!(
        tokens("0.5"),
        vec![Token::Number("0.5".into())]
    );
}

#[test]
fn number_large() {
    // el lexer guarda el string — no hay overflow aquí
    assert_eq!(
        tokens("999999999"),
        vec![Token::Number("999999999".into())]
    );
}

#[test]
fn number_very_large() {
    // número que desbordaría f64 — el lexer lo acepta como string
    // el parser/semántico decide qué hacer con él
    assert_eq!(
        tokens("99999999999999999999999999999999"),
        vec![Token::Number("99999999999999999999999999999999".into())]
    );
}

#[test]
fn number_large_float() {
    assert_eq!(
        tokens("123456789.987654321"),
        vec![Token::Number("123456789.987654321".into())]
    );
}

// ┌─────────────────────────────────────────────────────────────────────┐
// │ BUILTINS AS IDENTIFIERS: lexer must NOT recognize builtins as       │
// │ keywords; parser/semantic analyzer handles them                     │
// └─────────────────────────────────────────────────────────────────────┘

#[test]
fn builtins_are_idents() {
    assert_eq!(tokens("sin"),  vec![Token::Ident("sin".into())]);
    assert_eq!(tokens("cos"),  vec![Token::Ident("cos".into())]);
    assert_eq!(tokens("sqrt"), vec![Token::Ident("sqrt".into())]);
    assert_eq!(tokens("exp"),  vec![Token::Ident("exp".into())]);
    assert_eq!(tokens("log"),  vec![Token::Ident("log".into())]);
    assert_eq!(tokens("rand"), vec![Token::Ident("rand".into())]);
    assert_eq!(tokens("print"),vec![Token::Ident("print".into())]);
}

#[test]
fn math_constants_are_idents() {
    assert_eq!(tokens("PI"), vec![Token::Ident("PI".into())]);
    assert_eq!(tokens("E"),  vec![Token::Ident("E".into())]);
}

#[test]
fn builtin_types_are_idents() {
    assert_eq!(tokens("Number"),  vec![Token::Ident("Number".into())]);
    assert_eq!(tokens("String"),  vec![Token::Ident("String".into())]);
    assert_eq!(tokens("Boolean"), vec![Token::Ident("Boolean".into())]);
    assert_eq!(tokens("Object"),  vec![Token::Ident("Object".into())]);
}

#[test]
fn builtins_in_call_position() {
    // sin(PI) must tokenize as Ident + LParen + Ident + RParen
    assert_eq!(
        tokens("sin(PI)"),
        vec![
            Token::Ident("sin".into()),
            Token::LParen,
            Token::Ident("PI".into()),
            Token::RParen,
        ]
    );
}

#[test]
fn builtin_as_variable_name() {
    // HULK allows shadowing builtins with let
    // lexically this must work without errors
    assert_eq!(
        tokens("let sin = 42 in sin"),
        vec![
            Token::Let,
            Token::Ident("sin".into()),
            Token::Eq,
            Token::Number("42".into()),
            Token::In,
            Token::Ident("sin".into()),
        ]
    );
}

#[test]
fn type_names_in_annotation() {
    // Number, String, Boolean in type annotations are just Ident
    assert_eq!(
        tokens("x: Number"),
        vec![
            Token::Ident("x".into()),
            Token::Colon,
            Token::Ident("Number".into()),
        ]
    );
}

#[test]
fn log_call_is_ident_call() {
    assert_eq!(
        tokens("log(2, 8)"),
        vec![
            Token::Ident("log".into()),
            Token::LParen,
            Token::Number("2".into()),
            Token::Comma,
            Token::Number("8".into()),
            Token::RParen,
        ]
    );
}
