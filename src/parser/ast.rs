//! Abstract Syntax Tree for the HULK language.
//!
//! Design principles:
//!   - One node per semantic construct, not per grammar rule.
//!   - Precedence is encoded in tree structure, not in node types.
//!   - Spans are preserved on every node for error reporting.
//!   - No CST artifacts: parentheses, semicolons, and grammar
//!     stratification levels (Term, Factor) do not appear here.

use crate::lexer::lexer::Span;

// ══════════════════════════════════════════════
// PROGRAM
// ══════════════════════════════════════════════

/// A complete HULK program: zero or more declarations
/// followed by a mandatory global expression.
#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub decls:  Vec<Decl>,
    pub expr:   Box<Expr>,
    pub span:   Span,
}

// ══════════════════════════════════════════════
// DECLARATIONS
// ══════════════════════════════════════════════

#[derive(Debug, Clone, PartialEq)]
pub enum Decl {
    Function(FuncDecl),
    Type(TypeDecl),
    Protocol(ProtocolDecl),
}

// ── Function ─────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub struct FuncDecl {
    pub name:        String,
    pub params:      Vec<Param>,
    pub return_type: Option<TypeExpr>,
    pub body:        FuncBody,
    pub span:        Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Param {
    pub name: String,
    pub ty:   Option<TypeExpr>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FuncBody {
    /// function f(x) => expr;
    Inline(Box<Expr>),
    /// function f(x) { expr; expr; expr }
    Block(Box<Expr>),
}

// ── Type ─────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub struct TypeDecl {
    pub name:        String,
    pub type_params: Vec<Param>,
    pub inherits:    Option<InheritsClause>,
    pub members:     Vec<TypeMember>,
    pub span:        Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct InheritsClause {
    pub parent: String,
    pub args:   Vec<Expr>,
    pub span:   Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TypeMember {
    Attribute(AttrDef),
    Method(MethodDef),
}

#[derive(Debug, Clone, PartialEq)]
pub struct AttrDef {
    pub name: String,
    pub ty:   Option<TypeExpr>,
    pub init: Box<Expr>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MethodDef {
    pub name:        String,
    pub params:      Vec<Param>,
    pub return_type: Option<TypeExpr>,
    pub body:        FuncBody,
    pub span:        Span,
}

// ── Protocol ──────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub struct ProtocolDecl {
    pub name:    String,
    pub extends: Option<String>,
    pub methods: Vec<MethodSig>,
    pub span:    Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MethodSig {
    pub name:        String,
    pub params:      Vec<SigParam>,
    pub return_type: TypeExpr,
    pub span:        Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SigParam {
    pub name: String,
    pub ty:   Option<TypeExpr>,
    pub span: Span,
}

// ══════════════════════════════════════════════
// EXPRESSIONS
// ══════════════════════════════════════════════

/// Every syntactic construct in HULK is an expression.
/// Span is included in every variant for error reporting.
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {

    // ── Literals ─────────────────────────────

    /// Numeric literal. Stored as String to avoid parser-time
    /// float parsing — the semantic phase converts it to f64.
    Number { value: String, span: Span },

    /// String literal with escapes already expanded.
    StringLit { value: String, span: Span },

    /// Boolean literal.
    Bool { value: bool, span: Span },

    // ── Variables and calls ───────────────────

    /// Variable reference or function call.
    Ident { name: String, span: Span },

    /// Function or method call.
    Call {
        callee: Box<Expr>,
        args:   Vec<Expr>,
        span:   Span,
    },

    // ── Object oriented ───────────────────────

    /// `new TypeName(args)`
    New {
        type_name: String,
        args:      Vec<Expr>,
        span:      Span,
    },

    /// `expr.member` or `expr.method(args)`
    FieldAccess {
        object: Box<Expr>,
        field:  String,
        span:   Span,
    },

    MethodCall {
        object: Box<Expr>,
        method: String,
        args:   Vec<Expr>,
        span:   Span,
    },

    

    /// `base(args)` — call to parent implementation
    Base { args: Vec<Expr>, span: Span },

    // ── Operators ─────────────────────────────

    BinaryOp {
        op:    BinOp,
        left:  Box<Expr>,
        right: Box<Expr>,
        span:  Span,
    },

    UnaryOp {
        op:      UnaryOp,
        operand: Box<Expr>,
        span:    Span,
    },

    // ── Type operations ───────────────────────

    /// `expr is TypeExpr`
    IsType {
        expr:    Box<Expr>,
        ty:      TypeExpr,
        span:    Span,
    },

    /// `expr as TypeExpr`
    AsType {
        expr: Box<Expr>,
        ty:   TypeExpr,
        span: Span,
    },

    // ── Control flow ──────────────────────────

    /// `if (cond) then elif* else`
    If {
        condition: Box<Expr>,
        then_expr: Box<Expr>,
        elif_branches: Vec<ElifBranch>,
        else_expr: Box<Expr>,
        span:      Span,
    },

    /// `while (cond) body`
    While {
        condition: Box<Expr>,
        body:      Box<Expr>,
        span:      Span,
    },

    /// `for (var in iterable) body`
    For {
        var:      String,
        iterable: Box<Expr>,
        body:     Box<Expr>,
        span:     Span,
    },

    // ── Binding ───────────────────────────────

    /// `let x = e1, y = e2 in body`
    Let {
        bindings: Vec<LetBinding>,
        body:     Box<Expr>,
        span:     Span,
    },

    /// `var := expr` — destructive assignment (target can be Ident, FieldAccess)
    Assign {
        target: Box<Expr>,
        value:  Box<Expr>,
        span:   Span,
    },

    // ── Blocks ────────────────────────────────

    /// `{ expr; expr; expr }`
    /// The value is the last expression.
    Block {
        exprs: Vec<Expr>,
        span:  Span,
    },

    /// Error placeholder used for recovery after a parse error.
    Error {
        span: Span,
    },
}

// ── Supporting types ──────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub struct ElifBranch {
    pub condition: Box<Expr>,
    pub body:      Box<Expr>,
    pub span:      Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LetBinding {
    pub name: String,
    pub ty:   Option<TypeExpr>,
    pub init: Box<Expr>,
    pub span: Span,
}

// ══════════════════════════════════════════════
// OPERATORS
// ══════════════════════════════════════════════

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinOp {
    // Arithmetic
    Add, Sub, Mul, Div, Mod, Pow,
    // Comparison
    Eq, NotEq, Lt, Gt, LtEq, GtEq,
    // Boolean
    And, Or,
    // String
    Concat,       // @
    ConcatSpace,  // @@
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    Neg,  // -
    Not,  // !
}

// ══════════════════════════════════════════════
// TYPE EXPRESSIONS
// ══════════════════════════════════════════════

#[derive(Debug, Clone, PartialEq)]
pub enum TypeExpr {
    /// Simple nominal type: Number, String, MyType
    Named(String),
    /// Iterable of T: Number*
    Iterable(Box<TypeExpr>),
    /// Vector of T: Number[]
    Vector(Box<TypeExpr>),
}