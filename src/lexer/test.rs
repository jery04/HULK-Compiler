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

/// Extrae tokens incluyendo EOF.
fn tokens_with_eof(src: &str) -> Vec<Token> {
    let (toks, _) = TokenStream::tokenize_all(src);
    toks.into_iter().map(|t| t.token).collect()
}

// ── keywords ─────────────────────────────────────────────────────────────

#[test]
fn keywords() {
    let src = "let in if elif else while for function type new \
               inherits is as true false self base protocol extends def";
    assert_eq!(tokens(src), vec![
        Token::Let, Token::In, Token::If, Token::Elif, Token::Else,
        Token::While, Token::For, Token::Function, Token::Type,
        Token::New, Token::Inherits, Token::Is, Token::As,
        Token::True, Token::False, Token::SelfKw, Token::Base,
        Token::Protocol, Token::Extends, Token::Def,
    ]);
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
fn string_unclosed_gives_error() {
    // string sin cerrar en la misma línea → error léxico
    let errs = errors("\"hola mundo");
    assert!(!errs.is_empty());
}

#[test]
fn string_no_multiline() {
    // salto de línea dentro del string → error, no token válido
    let errs = errors("\"hola\nmundo\"");
    assert!(!errs.is_empty());
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
fn error_no_panic() {
    let errs = errors("let x = #42;");
    assert!(!errs.is_empty());
    assert!(errs[0].slice.contains('#'));
}

#[test]
fn error_has_position() {
    let errs = errors("let\n x = #;");
    assert_eq!(errs[0].span.start.line, 2);
}

#[test]
fn errors_dont_stop_tokenization() {
    // '#' es inválido pero los tokens siguientes deben seguir apareciendo
    let (toks, errs) = TokenStream::tokenize_all("let #x = 42;");
    assert!(!errs.is_empty());
    // debe haber tokenizado "let", "x", "=", "42", ";"
    let token_kinds: Vec<_> = toks.iter().map(|t| &t.token).collect();
    assert!(token_kinds.contains(&&Token::Let));
    assert!(token_kinds.contains(&&Token::Ident("x".into())));
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
    let toks = tokens("[x^2 | x in range(1,10)]");
    assert_eq!(toks[0], Token::LBracket);
    assert!(toks.contains(&Token::Pipe));
    assert!(toks.contains(&Token::In));
}

#[test]
fn functor_type() {
    let toks = tokens("(Number) -> Boolean");
    assert!(toks.contains(&Token::ThinArrow));
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
            Token::SelfKw,
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
// 6. ARRAYS / VECTORES (15)
// ========================================================

#[test]
fn vector_1() {
    assert_eq!(
        tokens("[x^2 | x in range(1,10)]"),
        vec![
            Token::LBracket,
            Token::Ident("x".into()),
            Token::Caret,
            Token::Number("2".into()),
            Token::Pipe,
            Token::Ident("x".into()),
            Token::In,
            Token::Ident("range".into()),
            Token::LParen,
            Token::Number("1".into()),
            Token::Comma,
            Token::Number("10".into()),
            Token::RParen,
            Token::RBracket,
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
        tokens(r#"let x := 10 + 20 * (3 - 1)"#),
        vec![
            Token::Let, Token::Ident("x".into()),
            Token::ColonAssign,
            Token::Number("10".into()),
            Token::Plus,
            Token::Number("20".into()),
            Token::Star,
            Token::LParen,
                Token::Number("3".into()),
                Token::Minus,
                Token::Number("1".into()),
            Token::RParen,
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
        tokens(r#"let y := (5 * 2) + 7 - 3"#),
        vec![
            Token::Let, Token::Ident("y".into()),
            Token::ColonAssign,
            Token::LParen,
                Token::Number("5".into()),
                Token::Star,
                Token::Number("2".into()),
            Token::RParen,
            Token::Plus,
            Token::Number("7".into()),
            Token::Minus,
            Token::Number("3".into()),
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
        tokens(r#"{ let x := 1; x + 2 }"#),
        vec![
            Token::LBrace,
                Token::Let, Token::Ident("x".into()), Token::ColonAssign, Token::Number("1".into()), Token::Semicolon,
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
        tokens(r#"
            for(i := 0; i < 5; i = i + 1){
                print(i);
            }
        "#),
        vec![
            Token::For, Token::LParen,
                Token::Ident("i".into()), Token::ColonAssign, Token::Number("0".into()), Token::Semicolon,
                Token::Ident("i".into()), Token::Lt, Token::Number("5".into()), Token::Semicolon,
                Token::Ident("i".into()), Token::Eq,
                Token::Ident("i".into()), Token::Plus, Token::Number("1".into()),
            Token::RParen,
            Token::LBrace,
                Token::Ident("print".into()), Token::LParen,
                    Token::Ident("i".into()),
                Token::RParen, Token::Semicolon,
            Token::RBrace,
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
    assert_eq!(
        tokens(r#"a = b = c = 5"#),
        vec![
            Token::Ident("a".into()), Token::Eq,
            Token::Ident("b".into()), Token::Eq,
            Token::Ident("c".into()), Token::Eq,
            Token::Number("5".into()),
        ]
    );
}

#[test]
fn string_concat() {
    assert_eq!(
        tokens(r#"let s := "hi" @ " there""#),
        vec![
            Token::Let, Token::Ident("s".into()),
            Token::ColonAssign,
            Token::StringLit("hi".into()),
            Token::At,
            Token::StringLit(" there".into()),
        ]
    );
}

#[test]
fn new_object_with_args() {
    assert_eq!(
        tokens(r#"let p := new Point(3,4)"#),
        vec![
            Token::Let, Token::Ident("p".into()),
            Token::ColonAssign,
            Token::New, Token::Ident("Point".into()),
            Token::LParen,
                Token::Number("3".into()), Token::Comma, Token::Number("4".into()),
            Token::RParen,
        ]
    );
}

#[test]
fn array_literal() {
    assert_eq!(
        tokens(r#"[1,2,3]"#),
        vec![
            Token::LBracket,
                Token::Number("1".into()), Token::Comma,
                Token::Number("2".into()), Token::Comma,
                Token::Number("3".into()),
            Token::RBracket,
        ]
    );
}

#[test]
fn index_access() {
    assert_eq!(
        tokens(r#"a[2]"#),
        vec![
            Token::Ident("a".into()),
            Token::LBracket,
                Token::Number("2".into()),
            Token::RBracket,
        ]
    );
}














#[test]
fn boolean_logic_1() {
    assert_eq!(
        tokens(r#"let ok := true & false"#),
        vec![
            Token::Let, Token::Ident("ok".into()),
            Token::ColonAssign,
            Token::True,
            Token::Amp,
            Token::False,
        ]
    );
}

#[test]
fn boolean_logic_2() {
    assert_eq!(
        tokens(r#"let r := !false | true"#),
        vec![
            Token::Let, Token::Ident("r".into()),
            Token::ColonAssign,
            Token::Bang,
            Token::False,
            Token::Pipe,
            Token::True,
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
                let d := x * x + y * y;
                d
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
                Token::ColonAssign,
                Token::Ident("x".into()), Token::Star, Token::Ident("x".into()),
                Token::Plus,
                Token::Ident("y".into()), Token::Star, Token::Ident("y".into()),
                Token::Semicolon,

                Token::Ident("d".into()),
            Token::RBrace,
        ]
    );
}
