use crate::lexer::lexer::{Token, SpannedToken, TokenStream};
use crate::parser::ast::{BinaryOp, BuiltinFn, ConstValue, Expression, Factor, Term};

// ─────────────────────────────────────────────────────────────────────────────
// Parser
// ─────────────────────────────────────────────────────────────────────────────

pub struct Parser<'src> {
    tokens: TokenStream<'src>,
    current: SpannedToken,
    pub errors: Vec<String>,
}

impl<'src> Parser<'src> {

    // ─────────────────────────────────────────────────────────────────────────
    // Inicialización
    // ─────────────────────────────────────────────────────────────────────────

    pub fn new(mut tokens: TokenStream<'src>) -> Self {
        let first = tokens.next_token();

        Self {
            tokens,
            current: first,
            errors: Vec::new(),
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Core navigation
    // ─────────────────────────────────────────────────────────────────────────

    /// Avanza al siguiente token
    pub fn advance(&mut self) {
        self.current = self.tokens.next_token();
    }

    /// Token actual (sin consumir)
    pub fn peek(&self) -> &Token {
        &self.current.token
    }

    /// ¿Estamos en EOF?
    pub fn is_at_end(&self) -> bool {
        self.current.token == Token::Eof
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Matching básico
    // ─────────────────────────────────────────────────────────────────────────

    /// Verifica si el token actual es igual (exact match)
    pub fn check(&self, token: &Token) -> bool {
        &self.current.token == token
    }

    /// Consume si coincide exactamente
    pub fn matches(&mut self, expected: &Token) -> bool {
        if self.check(expected) {
            self.advance();
            true
        } else {
            false
        }
    }

    /// Consume uno de varios tokens posibles y devuelve el token consumido
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

    // ─────────────────────────────────────────────────────────────────────────
    // Matching por "tipo" (clave para tokens con datos)
    // ─────────────────────────────────────────────────────────────────────────

    /// Verifica por patrón 
    pub fn check_kind(&self, f: fn(&Token) -> bool) -> bool {
        f(&self.current.token)
    }

    /// Consume si cumple el patrón
    pub fn match_kind(&mut self, f: fn(&Token) -> bool) -> bool {
        if f(&self.current.token) {
            self.advance();
            true
        } else {
            false
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Expect (obligatorio)
    // ─────────────────────────────────────────────────────────────────────────

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

    /// Expect por patrón (muy importante en lenguajes reales)
    pub fn expect_kind(
        &mut self,
        f: fn(&Token) -> bool,
        msg: &str,
    ) -> Option<SpannedToken> {
        if f(&self.current.token) {
            let tok = self.current.clone();
            self.advance();
            Some(tok)
        } else {
            self.error(msg);
            None
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Errores
    // ─────────────────────────────────────────────────────────────────────────

    fn error(&mut self, msg: &str) {
        let span = self.current.span;
        let full = format!("[ParseError {}] {}", span, msg);
        self.errors.push(full);
    }


    // ─────────────────────────────────────────────────────────────────────────
    // Parsing de expresiones
    // ─────────────────────────────────────────────────────────────────────────

    pub fn parse_expr(&mut self) -> Option<Expression> {
        let left = Expression::Term(self.parse_term()?);

        if let Some(op_tok) = self.match_any(&[Token::Plus, Token::Minus]) {
            let op = Self::binary_op_from_token(&op_tok.token)?;
            let right = self.parse_expr()?;
            return Some(Expression::Binary {
                left: Box::new(left),
                op,
                right: Box::new(right),
            });
        }

        Some(left)
    }

    pub fn parse_term(&mut self) -> Option<Term> {
        let left = Term::Factor(self.parse_factor()?);

        if let Some(op_tok) = self.match_any(&[Token::Star, Token::Slash]) {
            let op = Self::binary_op_from_token(&op_tok.token)?;
            let right = self.parse_term()?;
            return Some(Term::Binary {
                left: Box::new(left),
                op,
                right: Box::new(right),
            });
        }

        Some(left)
    }

    pub fn parse_factor(&mut self) -> Option<Factor> {

        let base = match self.peek() {
            Token::Number(value) => {
                let value = value.clone();
                self.advance();
                Factor::Number(value)
            }

            Token::LParen => {
                self.advance();
                let expr = self.parse_expr()?;
                self.expect(&Token::RParen, "se esperaba ')' al cerrar expresion")?;
                Factor::Group(Box::new(expr))
            }

            Token::Sin => {
                self.advance();
                let args = self.parse_call_args("sin")?;
                Factor::BuiltinCall { func: BuiltinFn::Sin, args }
            }
            Token::Cos => {
                self.advance();
                let args = self.parse_call_args("cos")?;
                Factor::BuiltinCall { func: BuiltinFn::Cos, args }
            }
            Token::Log => {
                self.advance();
                let args = self.parse_call_args("log")?;
                Factor::BuiltinCall { func: BuiltinFn::Log, args }
            }
            Token::Sqrt => {
                self.advance();
                let args = self.parse_call_args("sqrt")?;
                Factor::BuiltinCall { func: BuiltinFn::Sqrt, args }
            }
            Token::Exp => {
                self.advance();
                let args = self.parse_call_args("exp")?;
                Factor::BuiltinCall { func: BuiltinFn::Exp, args }
            }
            Token::Rand => {
                self.advance();
                let args = self.parse_call_args("rand")?;
                Factor::BuiltinCall { func: BuiltinFn::Rand, args }
            }
            Token::Pi => {
                self.advance();
                Factor::Const(ConstValue::Pi)
            }
            Token::E => {
                self.advance();
                Factor::Const(ConstValue::E)
            }

            _ => {
                self.error("se esperaba un factor");
                return None;
            }
        };

        // Exponenciación (^) tiene mayor precedencia y es asociativa a la derecha.
        if self.matches(&Token::Caret) {
            let right = self.parse_factor()?;
            let right_expr = Expression::Term(Term::Factor(right));
            return Some(Factor::Binary { left: Box::new(base), op: BinaryOp::Pow, right: Box::new(right_expr) });
        }

        Some(base)
    }

    fn parse_call_args(&mut self, ctx: &str) -> Option<Vec<Expression>> {
        let msg = format!("se esperaba '(' despues de {}", ctx);
        self.expect(&Token::LParen, &msg)?;

        let mut args = Vec::new();
        if !self.check(&Token::RParen) {
            loop {
                let expr = self.parse_expr()?;
                args.push(expr);

                if self.matches(&Token::Comma) {
                    continue;
                }

                break;
            }
        }

        self.expect(&Token::RParen, "se esperaba ')' al cerrar llamada")?;
        Some(args)
    }

    fn binary_op_from_token(token: &Token) -> Option<BinaryOp> {
        match token {
            Token::Plus => Some(BinaryOp::Add),
            Token::Minus => Some(BinaryOp::Sub),
            Token::Star => Some(BinaryOp::Mul),
            Token::Slash => Some(BinaryOp::Div),
            Token::Caret => Some(BinaryOp::Pow),
            _ => None,
        }
    }
}