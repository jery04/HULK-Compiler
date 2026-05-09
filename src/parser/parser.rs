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
        if self.check(expected) {
            let tok = self.current.clone();
            self.advance();
            Some(tok)
        } else {
            self.error(msg);
            None
        }
    }

    // `expect_kind` removed in Phase 1; use `expect` or reintroduce when needed.

    /// Record a parsing error with the current token's span.
    fn error(&mut self, msg: &str) {
        let span = self.current.span;
        let full = format!("[ParseError {}] {}", span, msg);
        self.errors.push(full);
    }

    /// Synchronize to a safe point for error recovery (panic-mode).
    /// Skips tokens until a likely statement/declaration boundary is found.
    #[allow(dead_code)]
    fn synchronize(&mut self) {
        self.advance();
        while !self.is_at_end() {
            match self.peek() {
                Token::Semicolon | Token::RBrace | Token::Eof 
                | Token::Function | Token::Type | Token::Protocol => {
                    break;
                }
                _ => {
                    self.advance();
                }
            }
        }
    }

    // ══════════════════════════════════════════════════════════════════════════
    // PHASE 1: EXPRESSION PARSING
    // ══════════════════════════════════════════════════════════════════════════

    /// Parse an expression. Entry point for expression parsing.
    ///
    /// **Precedence cascade:**
    /// Expr → parse_assign
    pub fn parse_expr(&mut self) -> Option<Expr> {
        self.parse_assign()
    }

    /// Parse assignment (destructive assignment, right-associative).
    ///
    /// **Grammar:** IDENT ":=" parse_assign | OrExpr
    ///
    /// Parse OrExpr first, then check if result is Ident and next token is ":=".
    fn parse_assign(&mut self) -> Option<Expr> {
        let left = self.parse_or()?;

        if self.matches(&Token::ColonAssign) {
            if let Expr::Ident { name, .. } = &left {
                let name = name.clone();
                let value = self.parse_assign()?;
                let span = Span {
                    start: left.span().start,
                    end: value.span().end,
                };
                return Some(Expr::Assign {
                    target: name,
                    value: Box::new(value),
                    span,
                });
            } else {
                self.error("assignment target must be a simple identifier");
                return None;
            }
        }

        Some(left)
    }

    /// Parse logical OR (left-associative).
    ///
    /// **Grammar:** AndExpr ("|" AndExpr)*
    fn parse_or(&mut self) -> Option<Expr> {
        let mut left = self.parse_and()?;

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
                            return None;
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

                // Phase 3: Type operations (is, as) — stub for now
                Token::Is => {
                    self.error("'is' operator: not yet implemented");
                    return None;
                }

                Token::As => {
                    self.error("'as' operator: not yet implemented");
                    return None;
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
                        return None;
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
                self.advance();
                let expr = self.parse_expr()?;
                self.expect(&Token::RParen, "expected ')' after expression")?;
                // Preserve original span but note that parentheses are gone
                Some(expr)
            }

            Token::LBrace => {
                self.parse_block()
            }

            Token::LBracket => {
                self.parse_vector()
            }

            // Phase 2: Binding and control flow (stubs)
            Token::Let => {
                self.error("'let' expression: not yet implemented");
                None
            }

            Token::If => {
                self.error("'if' expression: not yet implemented");
                None
            }

            Token::While => {
                self.error("'while' expression: not yet implemented");
                None
            }

            Token::For => {
                self.error("'for' expression: not yet implemented");
                None
            }

            _ => {
                self.error("expected expression");
                None
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
            let expr = self.parse_expr()?;
            exprs.push(expr);

            // Consume optional semicolon
            if !self.matches(&Token::Semicolon) {
                // If no semicolon, we expect the next token to be RBrace
                if !self.check(&Token::RBrace) {
                    self.error("expected ';' or '}' after block expression");
                    return None;
                }
            }
        }

        let rbrace = self.expect(&Token::RBrace, "expected '}' to close block")?;

        let span = Span {
            start: start_span.start,
            end: rbrace.span.end,
        };

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
        let first_expr = self.parse_expr()?;

        // Check for generator pattern: "|"
        if self.matches(&Token::Pipe) {
            let var = match self.peek() {
                Token::Ident(name) => {
                    let n = name.clone();
                    self.advance();
                    n
                }
                _ => {
                    self.error("expected variable name after '|'");
                    return None;
                }
            };

            self.expect(&Token::In, "expected 'in' after generator variable")?;
            let iterable = self.parse_expr()?;
            let rbracket = self.expect(&Token::RBracket, "expected ']' to close vector")?;

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

        while self.matches(&Token::Comma) {
            if self.check(&Token::RBracket) {
                break;
            }
            elements.push(self.parse_expr()?);
        }

        let rbracket = self.expect(&Token::RBracket, "expected ']' to close vector")?;

        let span = Span {
            start: start_span.start,
            end: rbracket.span.end,
        };

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
        }
    }
}
