use crate::lexer::lexer::SpannedToken;

// ---------------------------------------------
// ARITHMETIC 
// ---------------------------------------------

/// The abstract syntax tree (AST) for the HULKForge language, representing expressions, terms, factors, binary operations, built-in functions, constants, and function definitions.
#[derive(Debug, Clone, PartialEq)]
pub enum Expression {
    Term(Term),
    Binary {
        left: Box<Expression>,
        op: BinaryOp,
        right: Box<Expression>,
    },
}

/// A term can be a factor or a binary operation of factors. This allows for operator precedence, where multiplication and division are evaluated before addition and subtraction.
#[derive(Debug, Clone, PartialEq)]
pub enum Term {
    Factor(Factor),
    Binary {
        left: Box<Term>,
        op: BinaryOp,
        right: Box<Term>,
    },
}

/// A factor can be a number, a grouped expression, a binary operation of factors, an identifier, a function call, a built-in function call, or a constant value. This is the most basic unit of an expression.
#[derive(Debug, Clone, PartialEq)]
pub enum Factor {
    Number(String),
    Group(Box<Expression>),
    Binary {
        left: Box<Factor>,
        op: BinaryOp,
        right: Box<Expression>,
    },
    Ident(String),
    Call {
        callee: String,
        args: Vec<Expression>,
    },
    BuiltinCall {
        func: BuiltinFn,
        args: Vec<Expression>,
    },
    Const(ConstValue),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Pow,
}

/// The built-in functions supported by the HULKForge language, which can be called with the 'builtin' keyword followed by the function name and arguments.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuiltinFn {
    Sin,
    Cos,
    Log,
    Tan,
    Sqrt,
    Exp,
    Rand,
}

/// The constant values supported by the HULKForge language, which can be used in expressions and represent mathematical constants like pi and e.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConstValue {
    Pi,
    E,
}

// ---------------------------------------------
// FUNCTION 
// ---------------------------------------------

/// A function parameter consists of a name and an optional type annotation. The type annotation is represented as a spanned token, which includes the token itself and its position in the source code for error reporting.
#[derive(Debug, Clone, PartialEq)]
pub struct FunctionParam {
    pub name: SpannedToken,
    pub ty: Option<SpannedToken>,
}

/// A function body can be either an inline expression, which is a single expression followed by a semicolon, or a block of expressions enclosed in braces. This allows for both simple one-line functions and more complex multi-line functions.
#[derive(Debug, Clone, PartialEq)]
pub enum FunctionBody {
    Inline(Expression),
    Block(Vec<Expression>),
}

/// A function definition consists of a name, a list of parameters, and a body. The name is represented as a spanned token for error reporting, the parameters are a vector of function parameters, and the body is a function body that can be either inline or block.
#[derive(Debug, Clone, PartialEq)]
pub struct FunctionDef {
    pub name: SpannedToken,
    pub params: Vec<FunctionParam>,
    pub body: FunctionBody,
}
