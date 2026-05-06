#[derive(Debug, Clone, PartialEq)]
pub enum Expression {
    Term(Term),
    Binary {
        left: Box<Expression>,
        op: BinaryOp,
        right: Box<Expression>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum Term {
    Factor(Factor),
    Binary {
        left: Box<Term>,
        op: BinaryOp,
        right: Box<Term>,
    },
}

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConstValue {
    Pi,
    E,
}
