//! LL(1) Recursive Descent Parser for HULK
//!
//! Phase 1: Expression parsing
//! Phases 2-4: Declarations and associated constructs (not yet implemented)

use crate::lexer::lexer::{Token, SpannedToken, TokenStream, Span};
use crate::parser::ast::*;

// ══════════════════════════════════════════════════════════════════════════════
// PARSER STRUCT AND SETUP
// ══════════════════════════════════════════════════════════════════════════════

pub struct Parser<'src> {
    tokens:  TokenStream<'src>,
    current: SpannedToken,
    pub errors: Vec<String>,
}


impl<'src> Parser<'src> {
    /// Create a new parser from a token stream.
    pub fn new(mut tokens: TokenStream<'src>) -> Self {
        let first = tokens.next_token();
        Self {
            tokens,
            current: first,
            errors: Vec::new(),
        }
    }

    // ──────────────────────────────────────────────────────────────────────────
    // CORE NAVIGATION (unchanged from old parser)
    // ──────────────────────────────────────────────────────────────────────────

    /// Advance to the next token.
    pub fn advance(&mut self) {
        self.current = self.tokens.next_token();
    }

    /// Current token without consuming it.
    pub fn peek(&self) -> &Token {
        &self.current.token
    }

    /// Return true at EOF.
    pub fn is_at_end(&self) -> bool {
        self.current.token == Token::Eof
    }

    // ──────────────────────────────────────────────────────────────────────────
    // BASIC MATCHING (unchanged from old parser)
    // ──────────────────────────────────────────────────────────────────────────

    /// Check exact token match.
    pub fn check(&self, token: &Token) -> bool {
        &self.current.token == token
    }

    /// Peek the next token (after current) without consuming it.
    fn peek_next_token(&mut self) -> Option<Token> {
        self.tokens.peek_n(0).map(|t| t.token)
    }

    /// Does a token represent the start of an expression?
    fn token_starts_expr(token: &Token) -> bool {
        matches!(
            token,
            Token::Number(_)
                | Token::StringLit(_)
                | Token::True
                | Token::False
                | Token::Ident(_)
                | Token::SelfKw
                | Token::Base
                | Token::New
                | Token::LParen
                | Token::LBrace
                | Token::LBracket
                | Token::Let
                | Token::If
                | Token::While
                | Token::For
                | Token::Minus
                | Token::Bang
        )
    }

    /// Does the next token start a new expression?
    fn next_starts_expr(&mut self) -> bool {
        self.tokens
            .peek_n(0)
            .map_or(false, |t| Self::token_starts_expr(&t.token))
    }

    /// Consume the token if it matches exactly.
    pub fn matches(&mut self, expected: &Token) -> bool {
        if self.check(expected) {
            self.advance();
            true
        } else {
            false
        }
    }

    /// Consume one of several tokens and return it.
    pub fn match_any(&mut self, tokens: &[Token]) -> Option<SpannedToken> {
        for t in tokens {
            if &self.current.token == t {
                let tok = self.current.clone();
                self.advance();
                return Some(tok);
            }
        }
        None
    }

    // KIND MATCHING helpers removed in Phase 1 (not used).

    // ──────────────────────────────────────────────────────────────────────────
    // EXPECT AND ERROR HANDLING (unchanged from old parser)
    // ──────────────────────────────────────────────────────────────────────────

    /// Expect and consume a specific token, or record an error.
    pub fn expect(&mut self, expected: &Token, msg: &str) -> Option<SpannedToken> {
        self.expect_internal(expected, msg, true)
    }

    /// Expect and consume a specific token, with optional synchronization on error.
    fn expect_with_sync(&mut self, expected: &Token, msg: &str, sync: bool) -> Option<SpannedToken> {
        self.expect_internal(expected, msg, sync)
    }

    /// Internal expect implementation with customizable recovery scope.
    fn expect_internal(&mut self, expected: &Token, msg: &str, sync: bool) -> Option<SpannedToken> {
        if self.check(expected) {
            let tok = self.current.clone();
            self.advance();
            return Some(tok);
        }

        // Record error but try to recover locally without consuming closing delimiters.
        self.error_no_sync(msg);

        let mut recovery = vec![
            expected.clone(),
            Token::Semicolon,
            Token::RBrace,
            Token::RParen,
            Token::RBracket,
        ];
        if sync {
            recovery.push(Token::Function);
            recovery.push(Token::Type);
            recovery.push(Token::Protocol);
        }

        let _ = self.recover_to(&recovery);
        if self.check(expected) {
            let tok = self.current.clone();
            self.advance();
            return Some(tok);
        }

        Some(SpannedToken {
            token: expected.clone(),
            span: self.current.span,
            slice: String::new(),
        })
    }

    // `expect_kind` removed in Phase 1; use `expect` or reintroduce when needed.

    /// Record a parsing error with the current token's span.
    fn error(&mut self, msg: &str) {
        let span = self.current.span;
        let full = format!("[ParseError {}] {}", span, msg);
        self.errors.push(full);
        self.synchronize();
    }

    /// Record a parsing error with the current token's span, without synchronizing.
    fn error_no_sync(&mut self, msg: &str) {
        let span = self.current.span;
        let full = format!("[ParseError {}] {}", span, msg);
        self.errors.push(full);
    }

    /// Synchronize to a safe point for error recovery (panic-mode).
    /// Skips tokens until a likely statement/declaration boundary is found.
    fn synchronize(&mut self) {
        if self.is_at_end() {
            return;
        }
        self.advance();
        while !self.is_at_end() {
            match self.peek() {
                Token::Semicolon => {
                    self.advance();
                    break;
                }
                Token::RBrace | Token::RParen | Token::RBracket => {
                    self.advance();
                    break;
                }
                Token::Function | Token::Type | Token::Protocol => break,
                _ => self.advance(),
            }
        }
    }

    /// Advance until a recovery token or EOF is found. Leaves the recovery token unconsumed.
    fn recover_to(&mut self, recovery: &[Token]) -> bool {
        while !self.is_at_end() {
            if recovery.iter().any(|t| self.check(t)) {
                return true;
            }
            self.advance();
        }
        false
    }

    /// Create a placeholder expression after a parse error.
    fn error_expr(&self) -> Expr {
        Expr::Error { span: self.current.span }
    }

    // ══════════════════════════════════════════════════════════════════════════
    // PHASE 1: EXPRESSION PARSING
    // ══════════════════════════════════════════════════════════════════════════

    /// Parse an expression. Entry point for expression parsing.
    ///
    /// **Precedence cascade:**
    /// Expr → parse_assign
    pub fn parse_expr(&mut self) -> Option<Expr> {
        match self.parse_assign() {
            Some(expr) => Some(expr),
            None => Some(self.error_expr()),
        }
    }

    /// Parse assignment (destructive assignment, right-associative).
    ///
    /// **Grammar:** LVALUE ":=" parse_assign | OrExpr
    ///
    /// LVALUE can be: Ident, FieldAccess (x.field), or Index (x[i])
    /// Parse OrExpr first, then check if result is a valid lvalue and next token is ":=".
    fn parse_assign(&mut self) -> Option<Expr> {
        let left = self.parse_or()?;

        if self.matches(&Token::ColonAssign) {
            // Check if left is a valid assignment target (Ident, FieldAccess, or Index)
            match &left {
                Expr::Ident { .. } | Expr::FieldAccess { .. } | Expr::Index { .. } => {
                    let value = self.parse_assign()?;
                    let span = Span {
                        start: left.span().start,
                        end: value.span().end,
                    };
                    return Some(Expr::Assign {
                        target: Box::new(left),
                        value: Box::new(value),
                        span,
                    });
                }
                _ => {
                    self.error("assignment target must be an identifier, field access (e.g., x.field), or index (e.g., x[i])");
                    return Some(Expr::Error { span: left.span() });
                }
            }
        }

        Some(left)
    }

    /// Parse logical OR (left-associative).
    ///
    /// **Grammar:** AndExpr ("|" AndExpr)*
    fn parse_or(&mut self) -> Option<Expr> {
        let mut left = self.parse_and()?;

        // Accept the symbol `|` (Token::Pipe) as the OR operator.
        while self.matches(&Token::Pipe) {
            let right = self.parse_and()?;
            let span = Span {
                start: left.span().start,
                end: right.span().end,
            };
            left = Expr::BinaryOp {
                op: BinOp::Or,
                left: Box::new(left),
                right: Box::new(right),
                span,
            };
        }

        Some(left)
    }

    /// Parse logical AND (left-associative).
    ///
    /// **Grammar:** NotExpr ("&" NotExpr)*
    fn parse_and(&mut self) -> Option<Expr> {
        let mut left = self.parse_not()?;

        // Accept the symbol `&` (Token::Amp) as the AND operator.
        while self.matches(&Token::Amp) {
            let right = self.parse_not()?;
            let span = Span {
                start: left.span().start,
                end: right.span().end,
            };
            left = Expr::BinaryOp {
                op: BinOp::And,
                left: Box::new(left),
                right: Box::new(right),
                span,
            };
        }

        Some(left)
    }

    /// Parse logical NOT (prefix, right-associative).
    ///
    /// **Grammar:** "!" NotExpr | CmpExpr
    fn parse_not(&mut self) -> Option<Expr> {
        if self.check(&Token::Bang) {
            let start = self.current.span;
            self.advance();
            let operand = self.parse_not()?;
            let span = Span {
                start: start.start,
                end: operand.span().end,
            };
            return Some(Expr::UnaryOp {
                op: UnaryOp::Not,
                operand: Box::new(operand),
                span,
            });
        }

        self.parse_cmp()
    }

    /// Parse comparison (non-associative, single operator only).
    ///
    /// **Grammar:** CatExpr (("==" | "!=" | "<" | ">" | "<=" | ">=") CatExpr)?
    fn parse_cmp(&mut self) -> Option<Expr> {
        let left = self.parse_cat()?;

        // Try to match a comparison operator
        if let Some(op_tok) = self.match_any(&[
            Token::EqEq,
            Token::BangEq,
            Token::Lt,
            Token::Gt,
            Token::LtEq,
            Token::GtEq,
        ]) {
            let op = match op_tok.token {
                Token::EqEq => BinOp::Eq,
                Token::BangEq => BinOp::NotEq,
                Token::Lt => BinOp::Lt,
                Token::Gt => BinOp::Gt,
                Token::LtEq => BinOp::LtEq,
                Token::GtEq => BinOp::GtEq,
                _ => unreachable!(),
            };

            let right = self.parse_cat()?;
            let span = Span {
                start: left.span().start,
                end: right.span().end,
            };
            return Some(Expr::BinaryOp {
                op,
                left: Box::new(left),
                right: Box::new(right),
                span,
            });
        }

        Some(left)
    }

    /// Parse string concatenation (left-associative).
    ///
    /// **Grammar:** AddExpr (("@" | "@@") AddExpr)*
    fn parse_cat(&mut self) -> Option<Expr> {
        let mut left = self.parse_add()?;

        while let Some(op_tok) = self.match_any(&[Token::At, Token::ConcatSpace]) {
            let op = match op_tok.token {
                Token::At => BinOp::Concat,
                Token::ConcatSpace => BinOp::ConcatSpace,
                _ => unreachable!(),
            };

            let right = self.parse_add()?;
            let span = Span {
                start: left.span().start,
                end: right.span().end,
            };
            left = Expr::BinaryOp {
                op,
                left: Box::new(left),
                right: Box::new(right),
                span,
            };
        }

        Some(left)
    }

    /// Parse addition and subtraction (left-associative).
    ///
    /// **Grammar:** MulExpr (("+" | "-") MulExpr)*
    fn parse_add(&mut self) -> Option<Expr> {
        let mut left = self.parse_mul()?;

        while let Some(op_tok) = self.match_any(&[Token::Plus, Token::Minus]) {
            let op = match op_tok.token {
                Token::Plus => BinOp::Add,
                Token::Minus => BinOp::Sub,
                _ => unreachable!(),
            };

            let right = self.parse_mul()?;
            let span = Span {
                start: left.span().start,
                end: right.span().end,
            };
            left = Expr::BinaryOp {
                op,
                left: Box::new(left),
                right: Box::new(right),
                span,
            };
        }

        Some(left)
    }

    /// Parse multiplication, division, and modulo (left-associative).
    ///
    /// **Grammar:** PowerExpr (("*" | "/" | "%") PowerExpr)*
    fn parse_mul(&mut self) -> Option<Expr> {
        let mut left = self.parse_power()?;

        while let Some(op_tok) = self.match_any(&[Token::Star, Token::Slash, Token::Percent]) {
            let op = match op_tok.token {
                Token::Star => BinOp::Mul,
                Token::Slash => BinOp::Div,
                Token::Percent => BinOp::Mod,
                _ => unreachable!(),
            };

            let right = self.parse_power()?;
            let span = Span {
                start: left.span().start,
                end: right.span().end,
            };
            left = Expr::BinaryOp {
                op,
                left: Box::new(left),
                right: Box::new(right),
                span,
            };
        }

        Some(left)
    }

    /// Parse exponentiation (right-associative).
    ///
    /// **Grammar:** UnaryExpr ("^" PowerExpr)?
    ///
    /// Right-associativity is implemented via direct recursion.
    fn parse_power(&mut self) -> Option<Expr> {
        let left = self.parse_unary()?;

        if self.matches(&Token::Caret) {
            let right = self.parse_power()?; // Right-associative recursion
            let span = Span {
                start: left.span().start,
                end: right.span().end,
            };
            return Some(Expr::BinaryOp {
                op: BinOp::Pow,
                left: Box::new(left),
                right: Box::new(right),
                span,
            });
        }

        Some(left)
    }

    /// Parse unary minus (prefix, right-associative).
    ///
    /// **Grammar:** "-" UnaryExpr | PostfixExpr
    fn parse_unary(&mut self) -> Option<Expr> {
        if self.check(&Token::Minus) {
            let start = self.current.span;
            self.advance();
            let operand = self.parse_unary()?;
            let span = Span {
                start: start.start,
                end: operand.span().end,
            };
            return Some(Expr::UnaryOp {
                op: UnaryOp::Neg,
                operand: Box::new(operand),
                span,
            });
        }

        self.parse_postfix()
    }

    /// Parse postfix operations (left-associative): field access, method calls, indexing.
    ///
    /// **Grammar:** PrimaryExpr
    ///             ( "." IDENT ("(" ArgList ")")?
    ///             | "[" Expr "]"
    ///             | "is" TypeExpr     (Phase 3)
    ///             | "as" TypeExpr     (Phase 3)
    ///             )*
    fn parse_postfix(&mut self) -> Option<Expr> {
        let mut expr = self.parse_primary()?;

        loop {
            match self.peek() {
                Token::Dot => {
                    self.advance();
                    let field_span = self.current.span;
                    let field_name = match self.peek() {
                        Token::Ident(name) => {
                            let n = name.clone();
                            self.advance();
                            n
                        }
                        _ => {
                            self.error("expected field name after '.'");
                            return Some(Expr::Error { span: field_span });
                        }
                    };

                    // Check if this is a method call
                    if self.check(&Token::LParen) {
                        // Parse arguments inline to capture RParen span
                        self.expect(&Token::LParen, "expected '('")?;
                        let mut args = Vec::new();
                        while !self.check(&Token::RParen) && !self.is_at_end() {
                            args.push(self.parse_expr()?);
                            if !self.matches(&Token::Comma) {
                                break;
                            }
                        }
                        let rparen_tok = self.expect(&Token::RParen, "expected ')' to close argument list")?;
                        
                        let span = Span {
                            start: expr.span().start,
                            end: rparen_tok.span.end,
                        };
                        expr = Expr::MethodCall {
                            object: Box::new(expr),
                            method: field_name,
                            args,
                            span,
                        };
                    } else {
                        let span = Span {
                            start: expr.span().start,
                            end: field_span.end,
                        };
                        expr = Expr::FieldAccess {
                            object: Box::new(expr),
                            field: field_name,
                            span,
                        };
                    }
                }

                Token::LBracket => {
                    self.advance();
                    let index = self.parse_expr()?;
                    let rbracket_tok = self.expect(&Token::RBracket, "expected ']' after index expression")?;
                    let span = Span {
                        start: expr.span().start,
                        end: rbracket_tok.span.end,
                    };
                    expr = Expr::Index {
                        object: Box::new(expr),
                        index: Box::new(index),
                        span,
                    };
                }

                // Phase 3: Type operations (is, as)
                Token::Is => {
                    self.advance();
                    let (ty, end_span) = self.parse_type_expr_or_error();
                    let span = Span {
                        start: expr.span().start,
                        end: end_span.end,
                    };
                    expr = Expr::IsType { expr: Box::new(expr), ty, span };
                }

                Token::As => {
                    self.advance();
                    let (ty, end_span) = self.parse_type_expr_or_error();
                    let span = Span {
                        start: expr.span().start,
                        end: end_span.end,
                    };
                    expr = Expr::AsType { expr: Box::new(expr), ty, span };
                }

                _ => break,
            }
        }

        Some(expr)
    }

    /// Parse primary expressions (terminals and constructs).
    ///
    /// **Grammar:** NUMBER | STRING | "true" | "false" | "self" | IDENT ("(" ArgList ")")?
    ///            | "base" ("(" ArgList ")")?
    ///            | "new" IDENT ("(" ArgList ")")?
    ///            | "(" Expr ")"
    ///            | Block
    ///            | VectorExpr
    ///            | LetExpr (Phase 2)
    ///            | IfExpr (Phase 2)
    ///            | WhileExpr (Phase 2)
    ///            | ForExpr (Phase 2)
    fn parse_primary(&mut self) -> Option<Expr> {
        let span = self.current.span;

        match self.peek().clone() {
            Token::Number(value) => {
                let value = value.clone();
                self.advance();
                Some(Expr::Number { value, span })
            }

            Token::StringLit(value) => {
                let value = value.clone();
                self.advance();
                Some(Expr::StringLit { value, span })
            }

            Token::True => {
                self.advance();
                Some(Expr::Bool { value: true, span })
            }

            Token::False => {
                self.advance();
                Some(Expr::Bool { value: false, span })
            }

            Token::SelfKw => {
                self.advance();
                Some(Expr::SelfRef { span })
            }

            Token::Ident(name) => {
                let name = name.clone();
                self.advance();

                // Check for function call
                if self.check(&Token::LParen) {
                    let (args, rparen_span) = self.parse_arg_list()?;
                    let span = Span {
                        start: span.start,
                        end: rparen_span.end,
                    };
                    Some(Expr::Call {
                        callee: Box::new(Expr::Ident { name, span }),
                        args,
                        span,
                    })
                } else {
                    Some(Expr::Ident { name, span })
                }
            }

            Token::Base => {
                self.advance();
                let (args, rparen_span) = self.parse_arg_list()?;
                let span = Span {
                    start: span.start,
                    end: rparen_span.end,
                };
                Some(Expr::Base { args, span })
            }

            Token::New => {
                self.advance();
                let type_name = match self.peek() {
                    Token::Ident(name) => {
                        let n = name.clone();
                        self.advance();
                        n
                    }
                    _ => {
                        self.error("expected type name after 'new'");
                        return Some(Expr::Error { span });
                    }
                };
                    let (args, rparen_span) = self.parse_arg_list()?;
                    let span = Span {
                        start: span.start,
                        end: rparen_span.end,
                    };
                Some(Expr::New {
                    type_name,
                    args,
                    span,
                })
            }

            Token::LParen => {
                // Possible lambda or grouped expression. We do a safe lookahead
                // using TokenStream::peek_n to detect the pattern:
                //   ( params ) -> Type => body
                //   ( params ) => body
                let lparen_span = self.current.span;
                // Look ahead to find matching ')' index and token after it.
                let mut idx = 1usize;
                let mut depth = 0i32;
                let mut found_rparen: Option<usize> = None;
                while let Some(tok) = self.tokens.peek_n(idx) {
                    match tok.token {
                        Token::LParen => depth += 1,
                        Token::RParen => {
                            if depth == 0 {
                                found_rparen = Some(idx);
                                break;
                            } else {
                                depth -= 1;
                            }
                        }
                        Token::Eof => break,
                        _ => {}
                    }
                    idx += 1;
                }

                if let Some(ridx) = found_rparen {
                    if let Some(after) = self.tokens.peek_n(ridx + 1) {
                        match after.token {
                            Token::ThinArrow | Token::Arrow => {
                                // It's a lambda: consume '(' and parse param list
                                self.advance(); // consume '('
                                let mut params = Vec::new();
                                if !self.check(&Token::RParen) {
                                    params.push(self.parse_param()?);
                                    while self.matches(&Token::Comma) {
                                        params.push(self.parse_param()?);
                                    }
                                }
                                let _rparen = self.expect(&Token::RParen, "expected ')' after parameter list")?;

                                // Optional return type after '->'
                                let return_type = if self.matches(&Token::ThinArrow) {
                                    let (t, _) = self.parse_type_expr_or_error();
                                    Some(t)
                                } else {
                                    None
                                };

                                // Body must follow '=>' (Arrow)
                                if self.matches(&Token::Arrow) {
                                    // Inline expression body
                                    let inline_expr = self.parse_expr()?;
                                    let body = FuncBody::Inline(Box::new(inline_expr));
                                    let body_end = match &body {
                                        FuncBody::Inline(e) => e.span().end,
                                        FuncBody::Block(e) => e.span().end,
                                    };
                                    let span = Span { start: lparen_span.start, end: body_end };
                                    return Some(Expr::Lambda { params, return_type, body, span });
                                } else if self.check(&Token::LBrace) {
                                    let block = self.parse_block()?;
                                    let body = FuncBody::Block(Box::new(block));
                                    let body_end = match &body {
                                        FuncBody::Inline(e) => e.span().end,
                                        FuncBody::Block(e) => e.span().end,
                                    };
                                    let span = Span { start: lparen_span.start, end: body_end };
                                    return Some(Expr::Lambda { params, return_type, body, span });
                                } else {
                                    self.error("expected '=>' or '{' after lambda header");
                                    return Some(Expr::Error { span: lparen_span });
                                }
                            }
                            _ => {}
                        }
                    }
                }

                // Fallback: regular grouped expression OR parenthesized block
                // Allow parentheses to contain one or more expressions separated by ';',
                // so forms like `( a := a + 1; a )` or `(expr)` are accepted.
                self.advance(); // consume '('
                let inner = match self.parse_expr() {
                    Some(e) => e,
                    None => self.error_expr(),
                };
                let rparen = self.expect(&Token::RParen, "expected ')' after expression")?;
                let span = Span { start: lparen_span.start, end: rparen.span.end };
                Some(Self::set_expr_span(inner, span))
                
            }

            Token::LBrace => {
                self.parse_block()
            }

            Token::LBracket => {
                self.parse_vector()
            }

            // Phase 2: Binding and control flow
            Token::Let => self.parse_let(),

            Token::If => self.parse_if(),

            Token::While => self.parse_while(),

            Token::For => self.parse_for(),

            _ => {
                self.error("expected expression");
                Some(Expr::Error { span })
            }
        }
    }

    /// Parse a block expression: "{" ( Expr ";" )* Expr "}"
    ///
    /// **Grammar:** "{" ( Expr ";" )* Expr? "}"
    ///
    /// The block value is the last expression (or unit if empty, which is a semantic error).
    /// Empty blocks are reported as an error.
    fn parse_block(&mut self) -> Option<Expr> {
        let start_span = self.current.span;
        self.expect(&Token::LBrace, "expected '{'")?;

        let mut exprs = Vec::new();

        while !self.check(&Token::RBrace) && !self.is_at_end() {
            let expr = match self.parse_expr() {
                Some(e) => e,
                None => self.error_expr(),
            };
            exprs.push(expr);

            if self.matches(&Token::Semicolon) {
                continue;
            }
            if self.check(&Token::RBrace) {
                break;
            }

            self.error_no_sync("expected ';' or '}' after block expression");
            if !self.recover_to(&[Token::Semicolon, Token::RBrace]) {
                break;
            }
            if self.matches(&Token::Semicolon) {
                continue;
            }
        }

        let end_span = if self.check(&Token::RBrace) {
            let rbrace = self.current.clone();
            self.advance();
            rbrace.span.end
        } else {
            self.error("expected '}' to close block");
            self.current.span.end
        };

        let span = Span { start: start_span.start, end: end_span };

        Some(Expr::Block { exprs, span })
    }

    /// Parse a vector expression: "[" ... "]"
    ///
    /// **Grammar:**
    /// - Literal: "[" Expr ("," Expr)* "]"
    /// - Generator: "[" Expr "|" IDENT "in" Expr "]"
    /// - Empty: "[]"
    fn parse_vector(&mut self) -> Option<Expr> {
        let start_span = self.current.span;
        self.expect(&Token::LBracket, "expected '['")?;

        // Check for empty vector
        if self.check(&Token::RBracket) {
            self.advance();
            let span = Span {
                start: start_span.start,
                end: self.current.span.end,
            };
            return Some(Expr::VectorLit {
                elements: vec![],
                span,
            });
        }

        // Parse first element
        let first_expr = match self.parse_expr() {
            Some(e) => e,
            None => self.error_expr(),
        };
        let first_is_error = matches!(first_expr, Expr::Error { .. });

        // The generator syntax uses `|`, which is also the boolean OR token.
        // If parsing consumed it as `left | ident` and the next token is `in`,
        // recover that shape as a vector generator.
        if !first_is_error && self.check(&Token::In) {
            if let Expr::BinaryOp { op: BinOp::Or, left, right, .. } = &first_expr {
                if let Expr::Ident { name: var, .. } = right.as_ref() {
                    if self.expect(&Token::In, "expected 'in' after generator variable").is_none() {
                        self.recover_to(&[Token::RBracket]);
                        let end = if self.check(&Token::RBracket) {
                            let rbracket = self.current.clone();
                            self.advance();
                            rbracket.span.end
                        } else {
                            self.current.span.end
                        };
                        return Some(Expr::Error { span: Span { start: start_span.start, end } });
                    }
                    let iterable = match self.parse_expr() {
                        Some(e) => e,
                        None => self.error_expr(),
                    };
                    let rbracket = match self.expect(&Token::RBracket, "expected ']' to close vector") {
                        Some(tok) => tok,
                        None => {
                            let end = self.current.span.end;
                            return Some(Expr::Error { span: Span { start: start_span.start, end } });
                        }
                    };

                    let span = Span {
                        start: start_span.start,
                        end: rbracket.span.end,
                    };

                    return Some(Expr::VectorGen {
                        element: left.clone(),
                        var: var.clone(),
                        iterable: Box::new(iterable),
                        span,
                    });
                }
            }
        }

        // Check for generator pattern: "|"
        if !first_is_error && self.matches(&Token::Pipe) {
            let var = match self.peek() {
                Token::Ident(name) => {
                    let n = name.clone();
                    self.advance();
                    n
                }
                _ => {
                    self.error_no_sync("expected variable name after '|'" );
                    self.recover_to(&[Token::RBracket]);
                    let end = if self.check(&Token::RBracket) {
                        let rbracket = self.current.clone();
                        self.advance();
                        rbracket.span.end
                    } else {
                        self.current.span.end
                    };
                    return Some(Expr::Error { span: Span { start: start_span.start, end } });
                }
            };

            if !self.matches(&Token::In) {
                self.error_no_sync("expected 'in' after generator variable");
                self.recover_to(&[Token::RBracket]);
                let end = if self.check(&Token::RBracket) {
                    let rbracket = self.current.clone();
                    self.advance();
                    rbracket.span.end
                } else {
                    self.current.span.end
                };
                return Some(Expr::Error { span: Span { start: start_span.start, end } });
            }
            let iterable = match self.parse_expr() {
                Some(e) => e,
                None => self.error_expr(),
            };
            let rbracket = match self.expect(&Token::RBracket, "expected ']' to close vector") {
                Some(tok) => tok,
                None => {
                    let end = self.current.span.end;
                    return Some(Expr::Error { span: Span { start: start_span.start, end } });
                }
            };

            let span = Span {
                start: start_span.start,
                end: rbracket.span.end,
            };

            return Some(Expr::VectorGen {
                element: Box::new(first_expr),
                var,
                iterable: Box::new(iterable),
                span,
            });
        }

        // Otherwise, parse as literal vector
        let mut elements = vec![first_expr];

        loop {
            if self.matches(&Token::Comma) {
                if self.check(&Token::RBracket) {
                    break;
                }
                let elem = match self.parse_expr() {
                    Some(e) => e,
                    None => self.error_expr(),
                };
                if matches!(elem, Expr::Error { .. }) {
                    self.recover_to(&[Token::Comma, Token::RBracket]);
                }
                elements.push(elem);
                continue;
            }

            if self.check(&Token::RBracket) {
                break;
            }

            self.error_no_sync("expected ',' or ']' after vector element");
            if !self.recover_to(&[Token::Comma, Token::RBracket]) {
                break;
            }
        }

        let end_span = if self.check(&Token::RBracket) {
            let rbracket = self.current.clone();
            self.advance();
            rbracket.span.end
        } else {
            self.error("expected ']' to close vector");
            self.current.span.end
        };

        let span = Span { start: start_span.start, end: end_span };

        Some(Expr::VectorLit { elements, span })
    }

    /// Parse an argument list: "(" (Expr ("," Expr)*)? ")"
    ///
    /// Helper for function/method calls and constructors.
    /// Returns the parsed arguments and the span of the closing ")" token.
    fn parse_arg_list(&mut self) -> Option<(Vec<Expr>, Span)> {
        self.expect(&Token::LParen, "expected '('")?;

        let mut args = Vec::new();

        while !self.check(&Token::RParen) && !self.is_at_end() {
            args.push(self.parse_expr()?);

            if !self.matches(&Token::Comma) {
                break;
            }
        }

        let rparen = self.expect(&Token::RParen, "expected ')' to close argument list")?;

        Some((args, rparen.span))
    }

    // -------------------------
    // Phase 2 helpers & forms
    // -------------------------

    // Parse a simple TypeExpr for Phase 2/3 (Named, Iterable, Vector, Functor)
    // Returns (TypeExpr, Span) where Span is the span of the last consumed token.
    fn parse_type_expr(&mut self) -> Option<(TypeExpr, Span)> {
        self.parse_type_expr_internal(true)
    }

    // Parse a TypeExpr without synchronizing on error.
    fn parse_type_expr_soft(&mut self) -> Option<(TypeExpr, Span)> {
        self.parse_type_expr_internal(false)
    }

    // Parse a TypeExpr, optionally synchronizing on error.
    fn parse_type_expr_internal(&mut self, sync: bool) -> Option<(TypeExpr, Span)> {
        // Functor type: (A, B) -> C
        if self.check(&Token::LParen) {
            self.expect_with_sync(&Token::LParen, "expected '(' starting functor type", sync)?;
            let mut params = Vec::new();
            // At least one type required inside functor params
            let (t, _) = self.parse_type_expr_internal(sync)?;
            params.push(t);
            while self.matches(&Token::Comma) {
                let (t, _) = self.parse_type_expr_internal(sync)?;
                params.push(t);
            }
            self.expect_with_sync(&Token::RParen, "expected ')' after functor param list", sync)?;
            self.expect_with_sync(&Token::ThinArrow, "expected '->' after functor param list", sync)?;
            let (returns, end_span) = self.parse_type_expr_internal(sync)?;
            return Some((TypeExpr::Functor { params, returns: Box::new(returns) }, end_span));
        }

        // Named types and postfix operators
        match self.peek().clone() {
            Token::Ident(name) => {
                let name = name.clone();
                let name_span = self.current.span;  // capture BEFORE advance
                self.advance();

                // Iterable: IDENT '*'
                if self.check(&Token::Star) {
                    let star_span = self.current.span;  // capture BEFORE advance
                    self.advance();
                    return Some((TypeExpr::Iterable(Box::new(TypeExpr::Named(name))), star_span));
                }

                // Vector: IDENT '[]'
                if self.check(&Token::LBracket) {
                    // consume '[' then expect ']'
                    self.expect_with_sync(&Token::LBracket, "expected '[' in type vector", sync)?;
                    let rbracket = self.expect_with_sync(&Token::RBracket, "expected ']' in type vector", sync)?;
                    return Some((TypeExpr::Vector(Box::new(TypeExpr::Named(name))), rbracket.span));
                }

                // Simple named type
                Some((TypeExpr::Named(name), name_span))
            }
            _ => {
                self.error_no_sync("expected type expression");
                let mut recovery = vec![
                    Token::Comma,
                    Token::RParen,
                    Token::ThinArrow,
                    Token::RBracket,
                    Token::Colon,
                    Token::Eq,
                    Token::Semicolon,
                    Token::RBrace,
                ];
                if sync {
                    recovery.push(Token::Function);
                    recovery.push(Token::Type);
                    recovery.push(Token::Protocol);
                }
                let _ = self.recover_to(&recovery);
                Some((TypeExpr::Named("__parse_error__".to_string()), self.current.span))
            }
        }
    }

    // Parse a type expression, or return a placeholder to keep parsing.
    fn parse_type_expr_or_error(&mut self) -> (TypeExpr, Span) {
        match self.parse_type_expr() {
            Some((ty, span)) => (ty, span),
            None => (TypeExpr::Named("__parse_error__".to_string()), self.current.span),
        }
    }

    // Parse a single let binding: IDENT TypeAnnotation? '=' Expr
    fn parse_let_binding(&mut self) -> Option<LetBinding> {
        let (name, name_span, can_continue) = match self.peek().clone() {
            Token::Ident(n) => {
                let n = n.clone();
                let name_span = self.current.span;
                self.advance();
                (n, name_span, true)
            }
            Token::InternalIdent(_) => {
                let name_span = self.current.span;
                self.error_no_sync("internal identifiers not allowed in user code");
                self.advance();
                ("__parse_error__".to_string(), name_span, true)
            }
            _ => {
                let name_span = self.current.span;
                self.error_no_sync("expected identifier in let binding");
                let _ = self.recover_to(&[Token::Comma, Token::In, Token::Eq, Token::Semicolon]);
                let init = self.error_expr();
                let span = Span { start: name_span.start, end: init.span().end };
                return Some(LetBinding {
                    name: "__parse_error__".to_string(),
                    ty: None,
                    init: Box::new(init),
                    span,
                });
            }
        };

        let ty = if self.matches(&Token::Colon) {
            let (t, _) = self.parse_type_expr_or_error();
            Some(t)
        } else {
            None
        };

        if can_continue {
            self.expect(&Token::Eq, "expected '=' in let binding")?;
        }

        let init = match self.parse_expr() {
            Some(e) => e,
            None => self.error_expr(),
        };

        let span = Span { start: name_span.start, end: init.span().end };

        Some(LetBinding { name, ty, init: Box::new(init), span })
    }

    // Parse `let` expression: "let" LetBinding ("," LetBinding)* "in" Expr
    fn parse_let(&mut self) -> Option<Expr> {
    let start_span = self.current.span;
    self.advance(); // consume 'let'

    let mut bindings = Vec::new();
    bindings.push(self.parse_let_binding()?);

    while self.matches(&Token::Comma) {
        bindings.push(self.parse_let_binding()?);
    }

    self.expect(&Token::In, "expected 'in' after let bindings")?;

    let body = match self.parse_expr() {
        Some(e) => e,
        None => self.error_expr(),
    };

    let span = Span { start: start_span.start, end: body.span().end };
    Some(Expr::Let { bindings, body: Box::new(body), span })
    }

    // Parse `if` expression with mandatory else: if (cond) then_expr (elif (cond) expr)* else expr
    fn parse_if(&mut self) -> Option<Expr> {
        let start_span = self.current.span;
        self.advance(); // consume 'if'

        self.expect(&Token::LParen, "expected '(' after 'if'")?;
        let condition = self.parse_expr()?;
        self.expect(&Token::RParen, "expected ')' after if condition")?;

        let then_expr = self.parse_expr()?;

        let mut elif_branches: Vec<ElifBranch> = Vec::new();
        while self.check(&Token::Elif) {
            self.advance();
            self.expect(&Token::LParen, "expected '(' after 'elif'")?;
            let cond = self.parse_expr()?;
            self.expect(&Token::RParen, "expected ')' after elif condition")?;
            let body = self.parse_expr()?;
            let span = Span { start: cond.span().start, end: body.span().end };
            elif_branches.push(ElifBranch { condition: Box::new(cond), body: Box::new(body), span });
        }

        self.expect(&Token::Else, "expected 'else' clause on if expression")?;
        let else_expr = self.parse_expr()?;

        let span = Span { start: start_span.start, end: else_expr.span().end };
        Some(Expr::If {
            condition: Box::new(condition),
            then_expr: Box::new(then_expr),
            elif_branches,
            else_expr: Box::new(else_expr),
            span,
        })
    }

    // Parse `while` expression: while (cond) body
    fn parse_while(&mut self) -> Option<Expr> {
        let start_span = self.current.span;
        self.advance(); // consume 'while'

        self.expect(&Token::LParen, "expected '(' after 'while'")?;
        let condition = self.parse_expr()?;
        self.expect(&Token::RParen, "expected ')' after while condition")?;

        let body = self.parse_expr()?;
        let span = Span { start: start_span.start, end: body.span().end };
        Some(Expr::While { condition: Box::new(condition), body: Box::new(body), span })
    }

    // Parse `for` expression: for (IDENT in Expr) Expr
    fn parse_for(&mut self) -> Option<Expr> {
        let start_span = self.current.span;
        self.advance(); // consume 'for'

        self.expect(&Token::LParen, "expected '(' after 'for'")?;

        let var = match self.peek().clone() {
            Token::Ident(name) => {
                let n = name.clone();
                self.advance();
                n
            }
            _ => {
                self.error_no_sync("expected identifier in for loop header");
                let _ = self.recover_to(&[Token::In, Token::RParen]);
                "__parse_error__".to_string()
            }
        };

        self.expect(&Token::In, "expected 'in' in for loop header")?;
        let iterable = self.parse_expr()?;
        self.expect(&Token::RParen, "expected ')' after for loop header")?;

        let body = self.parse_expr()?;
        let span = Span { start: start_span.start, end: body.span().end };
        Some(Expr::For { var, iterable: Box::new(iterable), body: Box::new(body), span })
    }

    // Replace the span in an expression with `new_span`.
    // This consumes `expr` and returns a new Expr with the same shape
    // but updated span field. Keeps Boxed subexpressions intact.
    fn set_expr_span(expr: Expr, new_span: Span) -> Expr {
        match expr {
            Expr::Number { value, .. } => Expr::Number { value, span: new_span },
            Expr::StringLit { value, .. } => Expr::StringLit { value, span: new_span },
            Expr::Bool { value, .. } => Expr::Bool { value, span: new_span },
            Expr::Ident { name, .. } => Expr::Ident { name, span: new_span },
            Expr::Call { callee, args, .. } => Expr::Call { callee, args, span: new_span },
            Expr::New { type_name, args, .. } => Expr::New { type_name, args, span: new_span },
            Expr::FieldAccess { object, field, .. } => Expr::FieldAccess { object, field, span: new_span },
            Expr::MethodCall { object, method, args, .. } => Expr::MethodCall { object, method, args, span: new_span },
            Expr::SelfRef { .. } => Expr::SelfRef { span: new_span },
            Expr::Base { args, .. } => Expr::Base { args, span: new_span },
            Expr::BinaryOp { op, left, right, .. } => Expr::BinaryOp { op, left, right, span: new_span },
            Expr::UnaryOp { op, operand, .. } => Expr::UnaryOp { op, operand, span: new_span },
            Expr::IsType { expr: e, ty, .. } => Expr::IsType { expr: e, ty, span: new_span },
            Expr::AsType { expr: e, ty, .. } => Expr::AsType { expr: e, ty, span: new_span },
            Expr::If { condition, then_expr, elif_branches, else_expr, .. } => Expr::If { condition, then_expr, elif_branches, else_expr, span: new_span },
            Expr::While { condition, body, .. } => Expr::While { condition, body, span: new_span },
            Expr::For { var, iterable, body, .. } => Expr::For { var, iterable, body, span: new_span },
            Expr::Let { bindings, body, .. } => Expr::Let { bindings, body, span: new_span },
            Expr::Assign { target, value, .. } => Expr::Assign { target, value, span: new_span },
            Expr::Block { exprs, .. } => Expr::Block { exprs, span: new_span },
            Expr::VectorLit { elements, .. } => Expr::VectorLit { elements, span: new_span },
            Expr::VectorGen { element, var, iterable, .. } => Expr::VectorGen { element, var, iterable, span: new_span },
            Expr::Index { object, index, .. } => Expr::Index { object, index, span: new_span },
            Expr::Lambda { params, return_type, body, .. } => Expr::Lambda { params, return_type, body, span: new_span },
            Expr::Error { .. } => Expr::Error { span: new_span },
        }
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // PHASE 3: TYPE OPERATIONS & DECLARATIONS
    // ═══════════════════════════════════════════════════════════════════════════

    /// Parse a complete HULK program: decls followed by global expression.
    pub fn parse_program(&mut self) -> Option<Program> {
    let start_span = self.current.span;
    let mut decls = Vec::new();

    loop {
        match self.peek() {
            Token::Function => {
                match self.parse_func_decl() {
                    Some(f) => decls.push(Decl::Function(f)),
                    None => {}
                }
            }
            Token::Type => {
                match self.parse_type_decl() {
                    Some(t) => decls.push(Decl::Type(t)),
                    None => {}
                }
            }
            Token::Protocol => {
                match self.parse_protocol_decl() {
                    Some(p) => decls.push(Decl::Protocol(p)),
                    None => {}
                }
            }
            _ => break,
        }
    }

    if self.is_at_end() {
        self.error_no_sync("expected a global expression but found end of file");
        return None;
    }

    let expr = Box::new(self.parse_expr()?);
    self.matches(&Token::Semicolon);

    let span = Span { start: start_span.start, end: expr.span().end };
    Some(Program { decls, expr, span })
    }

    // Parse a parameter for functions/methods/type params
    fn parse_param(&mut self) -> Option<Param> {
        let (name, param_span) = match self.peek().clone() {
            Token::Ident(n) => {
                let span = self.current.span;
                self.advance();
                (n, span)
            }
            Token::InternalIdent(_) => {
                let span = self.current.span;
                self.error_no_sync("internal identifiers not allowed in user code");
                self.advance();
                ("__parse_error__".to_string(), span)
            }
            _ => {
                let span = self.current.span;
                self.error_no_sync("expected parameter name");
                let _ = self.recover_to(&[Token::Comma, Token::RParen]);
                return Some(Param { name: "__parse_error__".to_string(), ty: None, span });
            }
        };

        let ty = if self.matches(&Token::Colon) {
            let (t, _) = self.parse_type_expr_or_error();
            Some(t)
        } else {
            None
        };

        Some(Param { name, ty, span: param_span })
    }

    /// Parse function declaration: "function" IDENT "(" ParamList ")" ReturnType? FuncBody
    fn parse_func_decl(&mut self) -> Option<FuncDecl> {
        let start_span = self.current.span;
        self.expect(&Token::Function, "expected 'function'")?;

        let name = match self.peek().clone() {
            Token::Ident(n) => {
                self.advance();
                n
            }
            _ => {
                self.error_no_sync("expected function name");
                let _ = self.recover_to(&[Token::LParen, Token::Arrow, Token::LBrace]);
                "__parse_error__".to_string()
            }
        };

        self.expect(&Token::LParen, "expected '(' after function name")?;

        // Parse parameters
        let mut params = Vec::new();
        if !self.check(&Token::RParen) {
            params.push(self.parse_param()?);
            while self.matches(&Token::Comma) {
                params.push(self.parse_param()?);
            }
        }
        self.expect(&Token::RParen, "expected ')' after parameter list")?;

        // Optional return type
        let return_type = if self.matches(&Token::Colon) {
            let (t, _) = self.parse_type_expr_or_error();
            Some(t)
        } else {
            None
        };

        // Parse function body
        let body = if self.matches(&Token::Arrow) {
            // Inline: => expr;
            // Disallow "=> { ... }" — record error but continue by parsing the block as the body
            if self.check(&Token::LBrace) {
                let brace_span = self.current.span;
                self.errors.push(format!("[ParseError {}] inline function body cannot be a block; use '{{...}}' without '=>' or an expression after '=>'", brace_span));
                // Parse the block anyway so we can continue parsing subsequent items
                let block_expr = self.parse_block()?;
                if self.check(&Token::Semicolon) {
                    let semi_span = self.current.span;
                    self.errors.push(format!("[ParseError {}] unexpected ';' after function block body", semi_span));
                    self.advance();
                }
                FuncBody::Block(Box::new(block_expr))
            } else {
                let inline_expr = self.parse_expr()?;
                self.expect(&Token::Semicolon, "expected ';' after inline function body")?;
                FuncBody::Inline(Box::new(inline_expr))
            }
        } else if self.check(&Token::LBrace) {
            // Block body: must not be followed by a trailing semicolon
            let block_expr = self.parse_block()?;
            if self.check(&Token::Semicolon) {
                // Report error but consume semicolon to continue parsing
                let semi_span = self.current.span;
                self.errors.push(format!("[ParseError {}] unexpected ';' after function block body", semi_span));
                self.advance();
            }
            FuncBody::Block(Box::new(block_expr))
        } else {
            self.error_no_sync("expected '=>' or '{' for function body");
            let _ = self.recover_to(&[Token::Semicolon, Token::RBrace, Token::Function, Token::Type, Token::Protocol]);
            self.matches(&Token::Semicolon);
            FuncBody::Inline(Box::new(self.error_expr()))
        };

        // Use the body expression's span end instead of self.current.span.end
        let body_end = match &body {
            FuncBody::Inline(e) => e.span().end,
            FuncBody::Block(e) => e.span().end,
        };
        let span = Span { start: start_span.start, end: body_end };
        Some(FuncDecl { name, params, return_type, body, span })
    }

    /// Parse type declaration: "type" IDENT TypeArgs? Inheritance? "{" TypeMember* "}"
    fn parse_type_decl(&mut self) -> Option<TypeDecl> {
        let start_span = self.current.span;
        self.expect(&Token::Type, "expected 'type'")?;

        let name = match self.peek().clone() {
            Token::Ident(n) => {
                self.advance();
                n
            }
            _ => {
                self.error_no_sync("expected type name");
                let _ = self.recover_to(&[Token::LParen, Token::Inherits, Token::LBrace]);
                "__parse_error__".to_string()
            }
        };

        // Optional type parameters
        let mut type_params = Vec::new();
        if self.check(&Token::LParen) {
            self.advance();
            if !self.check(&Token::RParen) {
                type_params.push(self.parse_param()?);
                while self.matches(&Token::Comma) {
                    type_params.push(self.parse_param()?);
                }
            }
            self.expect(&Token::RParen, "expected ')' after type parameters")?;
        }

        // Optional inheritance clause
        let inherits = if self.matches(&Token::Inherits) {
            let parent = match self.peek().clone() {
                Token::Ident(p) => {
                    self.advance();
                    p
                }
                _ => {
                    self.error_no_sync("expected parent type name");
                    let _ = self.recover_to(&[Token::LParen, Token::LBrace]);
                    "__parse_error__".to_string()
                }
            };

            // Optional constructor arguments
            let args = if self.check(&Token::LParen) {
                let (a, _) = self.parse_arg_list()?;
                a
            } else {
                Vec::new()
            };

            let inh_span = Span { start: self.current.span.start, end: self.current.span.end };
            Some(InheritsClause { parent, args, span: inh_span })
        } else {
            None
        };

        self.expect(&Token::LBrace, "expected '{' for type body")?;

        // Parse members
        let mut members = Vec::new();
        while !self.check(&Token::RBrace) && !self.is_at_end() {
            let member_start = self.current.span;

            let member_name = match self.peek().clone() {
                Token::Ident(n) => {
                    self.advance();
                    n
                }
                _ => {
                    self.error_no_sync("expected member name");
                    if !self.recover_to(&[Token::Semicolon, Token::RBrace]) {
                        break;
                    }
                    if self.matches(&Token::Semicolon) {
                        continue;
                    }
                    if self.check(&Token::RBrace) {
                        break;
                    }
                    continue;
                }
            };

            // Peek at next token to determine if method or attribute
            match self.peek() {
                Token::LParen => {
                    // Method: name "(" ParamList ")" ReturnType? Body
                    self.advance();
                    let mut params = Vec::new();
                    if !self.check(&Token::RParen) {
                        params.push(self.parse_param()?);
                        while self.matches(&Token::Comma) {
                            params.push(self.parse_param()?);
                        }
                    }
                    self.expect(&Token::RParen, "expected ')' after method parameters")?;

                    let return_type = if self.matches(&Token::Colon) {
                        let (t, _) = self.parse_type_expr_or_error();
                        Some(t)
                    } else {
                        None
                    };

                    let body = if self.matches(&Token::Arrow) {
                        // Disallow "=> { ... }" for inline method bodies — record error but continue
                        if self.check(&Token::LBrace) {
                            let brace_span = self.current.span;
                            self.errors.push(format!("[ParseError {}] inline method body cannot be a block; use '{{...}}' without '=>' or an expression after '=>'", brace_span));
                            // Parse the block to continue parsing members
                            let block = self.parse_block()?;
                            if self.check(&Token::Semicolon) {
                                let semi_span = self.current.span;
                                self.errors.push(format!("[ParseError {}] unexpected ';' after method block body", semi_span));
                                self.advance();
                            }
                            FuncBody::Block(Box::new(block))
                        } else {
                            let inline = self.parse_expr()?;
                            self.expect(&Token::Semicolon, "expected ';' after method inline body")?;
                            FuncBody::Inline(Box::new(inline))
                        }
                    } else if self.check(&Token::LBrace) {
                        // Block body: must not be followed by a trailing semicolon
                        let block = self.parse_block()?;
                        if self.check(&Token::Semicolon) {
                            let semi_span = self.current.span;
                            self.errors.push(format!("[ParseError {}] unexpected ';' after method block body", semi_span));
                            self.advance();
                        }
                        FuncBody::Block(Box::new(block))
                    } else {
                        self.error_no_sync("expected '=>' or '{' for method body");
                        let _ = self.recover_to(&[Token::Semicolon, Token::RBrace]);
                        self.matches(&Token::Semicolon);
                        FuncBody::Inline(Box::new(self.error_expr()))
                    };

                    // Use the body expression's span end instead of self.current.span.end
                    let method_end = match &body {
                        FuncBody::Inline(e) => e.span().end,
                        FuncBody::Block(e) => e.span().end,
                    };
                    let method_span = Span { start: member_start.start, end: method_end };
                    members.push(TypeMember::Method(MethodDef { name: member_name, params, return_type, body, span: method_span }));
                }
                Token::Colon | Token::Eq => {
                    // Attribute: name [":" TypeExpr] "=" Expr ";"
                    let attr_type = if self.matches(&Token::Colon) {
                        let (t, _) = self.parse_type_expr_or_error();
                        Some(t)
                    } else {
                        None
                    };

                    self.expect(&Token::Eq, "expected '=' in attribute definition")?;
                    let init = self.parse_expr()?;
                    let semi = self.expect(&Token::Semicolon, "expected ';' after attribute initializer")?;

                    let attr_span = Span { start: member_start.start, end: semi.span.end };
                    members.push(TypeMember::Attribute(AttrDef { name: member_name, ty: attr_type, init: Box::new(init), span: attr_span }));
                }
                _ => {
                    self.error_no_sync("expected ':' or '=' or '(' after member name");
                    if !self.recover_to(&[Token::Semicolon, Token::RBrace]) {
                        break;
                    }
                    if self.matches(&Token::Semicolon) {
                        continue;
                    }
                    if self.check(&Token::RBrace) {
                        break;
                    }
                }
            }
        }

        let rbrace = self.expect(&Token::RBrace, "expected '}' after type body")?;
        let span = Span { start: start_span.start, end: rbrace.span.end };
        Some(TypeDecl { name, type_params, inherits, members, span })
    }

    /// Parse protocol declaration: "protocol" IDENT ["extends" IDENT] "{" MethodSig* "}"
    fn parse_protocol_decl(&mut self) -> Option<ProtocolDecl> {
        let start_span = self.current.span;
        self.expect(&Token::Protocol, "expected 'protocol'")?;

        let name = match self.peek().clone() {
            Token::Ident(n) => {
                self.advance();
                n
            }
            _ => {
                self.error_no_sync("expected protocol name");
                let _ = self.recover_to(&[Token::Extends, Token::LBrace]);
                "__parse_error__".to_string()
            }
        };

        // Optional extends clause
        let extends = if self.matches(&Token::Extends) {
            let parent = match self.peek().clone() {
                Token::Ident(e) => {
                    self.advance();
                    e
                }
                _ => {
                    self.error_no_sync("expected parent protocol name");
                    let _ = self.recover_to(&[Token::LBrace]);
                    "__parse_error__".to_string()
                }
            };
            Some(parent)
        } else {
            None
        };

        self.expect(&Token::LBrace, "expected '{' for protocol body")?;

        // Parse method signatures
        let mut methods = Vec::new();
        while !self.check(&Token::RBrace) && !self.is_at_end() {
            let sig_start = self.current.span;

            let method_name = match self.peek().clone() {
                Token::Ident(n) => {
                    self.advance();
                    n
                }
                _ => {
                    self.error_no_sync("expected method name in protocol");
                    if !self.recover_to(&[Token::Semicolon, Token::RBrace]) {
                        break;
                    }
                    if self.matches(&Token::Semicolon) {
                        continue;
                    }
                    if self.check(&Token::RBrace) {
                        break;
                    }
                    continue;
                }
            };

            self.expect(&Token::LParen, "expected '(' after method name in protocol")?;

            // Parse signature parameters
            let mut sig_params = Vec::new();
            if !self.check(&Token::RParen) {
                let param_start = self.current.span;
                let param_name = match self.peek().clone() {
                    Token::Ident(n) => {
                        self.advance();
                        n
                    }
                    _ => {
                        self.error_no_sync("expected parameter name in protocol method");
                        let _ = self.recover_to(&[Token::Comma, Token::RParen]);
                        "__parse_error__".to_string()
                    }
                };

                let (param_ty, end_span) = if self.matches(&Token::Colon) {
                    let (ty, span) = self.parse_type_expr_or_error();
                    (Some(ty), span)
                } else {
                    (None, self.current.span)
                };
                let p_span = Span { start: param_start.start, end: end_span.end };
                sig_params.push(SigParam { name: param_name, ty: param_ty, span: p_span });

                while self.matches(&Token::Comma) {
                    let param_start = self.current.span;
                    let pname = match self.peek().clone() {
                        Token::Ident(n) => {
                            self.advance();
                            n
                        }
                        _ => {
                            self.error_no_sync("expected parameter name in protocol method");
                            let _ = self.recover_to(&[Token::Comma, Token::RParen]);
                            "__parse_error__".to_string()
                        }
                    };

                    let (pty, end_span) = if self.matches(&Token::Colon) {
                        let (ty, span) = self.parse_type_expr_or_error();
                        (Some(ty), span)
                    } else {
                        (None, self.current.span)
                    };
                    let ps = Span { start: param_start.start, end: end_span.end };
                    sig_params.push(SigParam { name: pname, ty: pty, span: ps });
                }
            }

            self.expect(&Token::RParen, "expected ')' after protocol method parameters")?;
            self.expect(&Token::Colon, "expected ':' for protocol method return type")?;
            let (return_type, _) = self.parse_type_expr_or_error();
            let semi = self.expect(&Token::Semicolon, "expected ';' after protocol method signature")?;

            let sig_span = Span { start: sig_start.start, end: semi.span.end };
            methods.push(MethodSig { name: method_name, params: sig_params, return_type, span: sig_span });
        }

        let rbrace = self.expect(&Token::RBrace, "expected '}' after protocol body")?;
        let span = Span { start: start_span.start, end: rbrace.span.end };
        Some(ProtocolDecl { name, extends, methods, span })
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// HELPER TRAIT: Span retrieval for Expr
// ══════════════════════════════════════════════════════════════════════════════

/// Trait to extract the span from any expression node.
/// This is used during parsing to compute spans for binary operators, etc.
trait HasSpan {
    fn span(&self) -> Span;
}

impl HasSpan for Expr {
    fn span(&self) -> Span {
        match self {
            Expr::Number { span, .. } => *span,
            Expr::StringLit { span, .. } => *span,
            Expr::Bool { span, .. } => *span,
            Expr::Ident { span, .. } => *span,
            Expr::Call { span, .. } => *span,
            Expr::New { span, .. } => *span,
            Expr::FieldAccess { span, .. } => *span,
            Expr::MethodCall { span, .. } => *span,
            Expr::SelfRef { span } => *span,
            Expr::Base { span, .. } => *span,
            Expr::BinaryOp { span, .. } => *span,
            Expr::UnaryOp { span, .. } => *span,
            Expr::IsType { span, .. } => *span,
            Expr::AsType { span, .. } => *span,
            Expr::If { span, .. } => *span,
            Expr::While { span, .. } => *span,
            Expr::For { span, .. } => *span,
            Expr::Let { span, .. } => *span,
            Expr::Assign { span, .. } => *span,
            Expr::Block { span, .. } => *span,
            Expr::VectorLit { span, .. } => *span,
            Expr::VectorGen { span, .. } => *span,
            Expr::Index { span, .. } => *span,
            Expr::Lambda { span, .. } => *span,
            Expr::Error { span } => *span,
        }
    }
}
