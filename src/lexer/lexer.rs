#![allow(dead_code)]

use logos::{Logos, SpannedIter};

// ---------------------------------------------
// SOURCE POSITION (1-BASED LINE/COLUMN)
// ---------------------------------------------

/// Position in the source code. Both values are 1-based.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Pos {
    pub line: usize,
    pub col: usize,
}

impl std::fmt::Display for Pos {
    /// Format the position as line and column.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.line, self.col)
    }
}

/// Span of a token: start and end position in the source.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Span {
    pub start: Pos,
    pub end: Pos,
}

impl std::fmt::Display for Span {
    /// Format the span as start-end.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}-{}", self.start, self.end)
    }
}

// ---------------------------------------------
// LINE TABLE: BYTE OFFSET TO (LINE, COLUMN)
// ---------------------------------------------

/// Index of start offsets for each line.
/// Allows converting byte-offset → (line, column) in O(log n).
pub struct LineIndex {
    /// starts[i] = byte offset where line i+1 starts (1-based).
    starts: Vec<usize>,
}

impl LineIndex {
    /// Create a new LineIndex from the source text.
    pub fn new(source: &str) -> Self {
        let mut starts = vec![0usize];
        for (i, b) in source.bytes().enumerate() {
            if b == b'\n' {
                starts.push(i + 1);
            }
        }
        Self { starts }
    }

    /// Convert a byte offset to a `Pos` (1-based line and column).
    pub fn pos(&self, offset: usize) -> Pos {
        let line_idx = self.starts.partition_point(|&s| s <= offset) - 1;
        let col = offset - self.starts[line_idx] + 1;
        Pos {
            line: line_idx + 1,
            col,
        }
    }

    /// Convert a byte range into a `Span`.
    pub fn span(&self, range: std::ops::Range<usize>) -> Span {
        Span {
            start: self.pos(range.start),
            // End points to the token last byte, not the next one.
            end: self.pos(range.end.saturating_sub(1)),
        }
    }

    /// Position of EOF: one character after the last.
    pub fn eof_pos(&self, source_len: usize) -> Pos {
        if source_len == 0 {
            Pos { line: 1, col: 1 }
        } else {
            let last = self.pos(source_len - 1);
            Pos {
                line: last.line,
                col: last.col + 1,
            }
        }
    }
}

// ---------------------------------------------
// TOKENS FOR THE HULK LANGUAGE
// ---------------------------------------------

#[derive(Logos, Debug, Clone, PartialEq, Eq, Hash)]
#[logos(skip r"[ \t\r\n]+")]  // Skip whitespace.
#[logos(skip r"//[^\n]*")]    // Skip line comments.
pub enum Token {

    // ---------------------------------------------
    // KEYWORDS
    // ---------------------------------------------
    #[token("let")]      Let,
    #[token("in")]       In,
    #[token("if")]       If,
    #[token("elif")]     Elif,
    #[token("else")]     Else,
    
    #[token("while")]    While,
    #[token("for")]      For,
    #[token("function")] Function,
    #[token("type")]     Type,
    #[token("new")]      New,
    #[token("inherits")] Inherits,
    #[token("is")]       Is,
    #[token("as")]       As,
    #[token("true")]     True,
    #[token("false")]    False,
    #[token("base")]     Base,
    #[token("protocol")] Protocol,
    #[token("extends")]  Extends,

    // ---------------------------------------------
    // LITERALS
    // ---------------------------------------------

    /// Numeric literal: integer or float.
    #[regex(r"[0-9]+(\.[0-9]+)?", |lex| lex.slice().to_owned())]
    Number(String),

    /// String literal with escapes: \" \n \t \\
    #[regex(r#""([^\n"\\]|\\.)*""#, lex_string)]
    StringLit(String),

    // ---------------------------------------------
    // IDENTIFIERS
    // ---------------------------------------------

    /// Valid HULK identifier: starts with a letter, then letters,
    /// digits, or underscore.
    /// Examples: x  x0  camelCase  TitleCase  snake_case
    #[regex(r"[a-zA-Z][a-zA-Z0-9_]*", |lex| lex.slice().to_owned())]
    Ident(String),

    /// Identifier that starts with '_'.
    ///
    /// In user code this is a semantic error (not lexical).
    /// The compiler generates these internally in transpilation
    /// (_total, _IsOddWrapper, etc.), so they must be valid tokens
    /// so the same lexer can re-tokenize transpiled code.
    ///
    /// The parser rejects `InternalIdent` in user code positions
    /// and accepts it only in compiler-generated code.
    #[regex(r"_[a-zA-Z0-9_]*", |lex| lex.slice().to_owned())]
    InternalIdent(String),

    // ---------------------------------------------------
    // MULTI-CHARACTER OPERATORS (BEFORE SINGLE-CHARACTER)
    // ---------------------------------------------------

    #[token(":=")] ColonAssign,   // Destructive assignment.
    #[token("=>")] Arrow,         // Inline function/lambda body.
    #[token("@@")] ConcatSpace,   // Concat with space (@ " " @).
    #[token("==")] EqEq,          // Equality.
    #[token("!=")] BangEq,        // Inequality.
    #[token("<=")] LtEq,          // Less than or equal.
    #[token(">=")] GtEq,          // Greater than or equal.
    #[token("->")] ThinArrow,     // Return type arrow in lambda headers.

    // ---------------------------------------------
    // SINGLE-CHARACTER OPERATORS
    // ---------------------------------------------

    #[token("+")] Plus,
    #[token("-")] Minus,
    #[token("*")] Star,
    #[token("/")] Slash,
    #[token("%")] Percent,
    #[token("^")] Caret,       // Power.
    #[token("@")] At,          // Simple concat.
    #[token("&")] Amp,         // Boolean AND.
    #[token("|")] Pipe,        // Boolean OR / implicit vector separator.
    #[token("!")] Bang,        // Boolean NOT.
    #[token("<")] Lt,
    #[token(">")] Gt,
    #[token("=")] Eq,          // Assignment in let/attributes.

    // ---------------------------------------------
    // PUNCTUATION
    // ---------------------------------------------

    #[token("(")] LParen,
    #[token(")")] RParen,
    #[token("{")] LBrace,
    #[token("}")] RBrace,
    #[token("[")] LBracket,
    #[token("]")] RBracket,
    #[token(";")] Semicolon,
    #[token(",")] Comma,
    #[token(".")] Dot,
    #[token(":")] Colon,       // Type annotation: x: Number.

    // ---------------------------------------------
    // EOF SENTINEL
    // ---------------------------------------------
    //
    // Not produced by `logos` (no #[token] or #[regex]). The `TokenStream`
    // wrapper inserts this token when the inner iterator is exhausted.
    // The parser uses this token to detect EOF without handling `Option`
    // on every peek()/advance() call.
    Eof,
}

// ---------------------------------------------
// LOGOS CALLBACKS
// ---------------------------------------------


/// Expand escape sequences in a string literal.
/// `logos` already ensures the pattern matches `"([^"\\]|\\.)*"`.
fn lex_string(lex: &mut logos::Lexer<Token>) -> Option<String> {
    let raw = lex.slice();
    let inner = &raw[1..raw.len() - 1]; // Remove outer quotes.
    let mut out = String::with_capacity(inner.len());
    let mut chars = inner.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next()? {
                '"'  => out.push('"'),
                'n'  => out.push('\n'),
                't'  => out.push('\t'),
                '\\' => out.push('\\'),
                // Unknown escape: keep it literal.
                other => { out.push('\\'); out.push(other); }
            }
        } else {
            out.push(c);
        }
    }
    Some(out)
}

// ---------------------------------------------
// LEXICAL ERROR
// ---------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub struct LexError {
    pub msg: String,
    pub span: Span,
    /// Source text that caused the error.
    pub slice: String,
}

impl std::fmt::Display for LexError {
    /// Format the lexical error for display.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[LexError {}] {} (encontrado {:?})", self.span, self.msg, self.slice)
    }
}

impl std::error::Error for LexError {}

// ---------------------------------------------
// SPANNED TOKEN
// ---------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SpannedToken {
    pub token: Token,
    pub span: Span,
    /// Original source text. Empty for Token::Eof.
    pub slice: String,
}

pub type LexResult = Result<SpannedToken, LexError>;

// ----------------------------------------
// INTERNAL LEXER: WRAPS `logos` AND MAPS OFFSETS
// ---------------------------------------------

struct InnerLexer<'src> {
    inner: SpannedIter<'src, Token>,
    index: LineIndex,
    source: &'src str,
}

impl<'src> InnerLexer<'src> {
    /// Create a new internal lexer for the given source.
    fn new(source: &'src str) -> Self {
        Self {
            inner: Token::lexer(source).spanned(),
            index: LineIndex::new(source),
            source,
        }
    }
}

impl<'src> Iterator for InnerLexer<'src> {
    type Item = LexResult;

    /// Return the next lexing result with span info.
    fn next(&mut self) -> Option<Self::Item> {
        let (result, byte_range) = self.inner.next()?;
        let span  = self.index.span(byte_range.clone());
        let slice = self.source[byte_range].to_owned();

        match result {
            Ok(token) => Some(Ok(SpannedToken { token, span, slice })),
            Err(())   => Some(Err(LexError {
                msg: "carácter inesperado".into(),
                span,
                slice,
            })),
        }
    }
}

// --------------------------------------------
// TOKEN STREAM PUBLIC INTERFACE
// ---------------------------------------------

// The only type the parser should use. Guarantees:
//   1. The final token is always `Token::Eof` (never returns `None`).
//   2. Lexical errors are collected in `errors` and do not interrupt
//      parsing, allowing the parser to continue and report all errors.
// ---------------------------------------------

pub struct TokenStream<'src> {
    inner:   InnerLexer<'src>,
    index:   LineIndex,          // Needed to compute EOF position.
    src_len: usize,
    /// Lexical errors found during tokenization.
    /// The parser can inspect them after parsing.
    pub errors: Vec<LexError>,
    /// Buffer to allow limited lookahead without consuming the inner iterator.
    buffer: std::collections::VecDeque<SpannedToken>,
}

impl<'src> TokenStream<'src> {
    pub fn new(source: &'src str) -> Self {
        Self {
            inner:       InnerLexer::new(source),
            index:       LineIndex::new(source),
            src_len:     source.len(),
            errors:      Vec::new(),
            buffer:      std::collections::VecDeque::new(),
        }
    }

    /// Tokenize the entire source at once.
    /// Returns the tokens (including EOF at the end) and collects errors.
    pub fn tokenize_all(source: &'src str) -> (Vec<SpannedToken>, Vec<LexError>) {
        // Tokenize via an internal lexer to collect all tokens and errors.
        let mut inner = InnerLexer::new(source);
        let mut tokens = Vec::new();
        let mut errors = Vec::new();
        loop {
            match inner.next() {
                Some(Ok(tok)) => tokens.push(tok),
                Some(Err(err)) => errors.push(err),
                None => break,
            }
        }
        // Append EOF sentinel
        let index = LineIndex::new(source);
        let eof_pos = index.eof_pos(source.len());
        tokens.push(SpannedToken { token: Token::Eof, span: Span { start: eof_pos, end: eof_pos }, slice: String::new() });
        (tokens, errors)
    }

    /// Advance and return the next token.
    /// Never returns an error: lexical errors are pushed to `self.errors`
    /// and the lexer skips the problematic character to continue.
    /// Guarantees it eventually returns `Token::Eof`.
    pub fn next_token(&mut self) -> SpannedToken {
        // Ensure buffer has at least one token.
        self.fill_buffer_up_to(0);
        let tok = self.buffer.pop_front().unwrap();
        // If EOF was popped, push it back so subsequent calls still return EOF.
        if tok.token == Token::Eof {
            self.buffer.push_front(tok.clone());
        }
        tok
    }

    /// Fill internal buffer up to index `n` (0-based) by consuming the inner iterator.
    fn fill_buffer_up_to(&mut self, n: usize) {
        while self.buffer.len() <= n {
            match self.inner.next() {
                Some(Ok(tok)) => self.buffer.push_back(tok),
                Some(Err(err)) => self.errors.push(err),
                None => {
                    let eof_pos = self.index.eof_pos(self.src_len);
                    let eof_tok = SpannedToken { token: Token::Eof, span: Span { start: eof_pos, end: eof_pos }, slice: String::new() };
                    self.buffer.push_back(eof_tok);
                    break;
                }
            }
        }
    }

    /// Peek ahead `n` tokens without consuming. `n=0` returns the next token.
    pub fn peek_n(&mut self, n: usize) -> Option<SpannedToken> {
        self.fill_buffer_up_to(n);
        self.buffer.get(n).cloned()
    }

    /// Are there accumulated lexical errors?
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }
}