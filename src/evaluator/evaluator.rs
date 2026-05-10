use crate::lexer::lexer::TokenStream;
use crate::parser::Parser;

/// Evalúa una expresión aritmética simple y retorna el resultado.
pub fn evaluate(source: &str) -> Option<i64> {
    let stream = TokenStream::new(source);
    let mut parser = Parser::new(stream);

    if let Some(program) = parser.parse_program() {
        // Evaluar la expresión principal del programa (internamente en f64)
        eval_expr(&program.expr)
    } else {
        None
    }
}

/// Evalúa una expresión recursivamente.
fn eval_expr(expr: &crate::parser::ast::Expr) -> Option<i64> {
    // Evaluamos en f64 cuando sea necesario y luego convertimos a i64
    fn eval_f64(expr: &crate::parser::ast::Expr) -> Option<f64> {
        use crate::parser::ast::Expr;

        match expr {
            Expr::Number { value, .. } => value.parse::<f64>().ok(),
            Expr::Ident { name, .. } => {
                match name.as_str() {
                    "PI" => Some(std::f64::consts::PI),
                    _ => None,
                }
            }
            Expr::Call { callee, args, .. } => {
                // soportar llamadas simples: sin(x), cos(x), log(base, value)
                if let Expr::Ident { name, .. } = &**callee {
                    let mut eval_args: Vec<f64> = Vec::new();
                    for a in args {
                        eval_args.push(eval_f64(a)?);
                    }
                    match name.as_str() {
                        "sin" if eval_args.len() == 1 => Some(eval_args[0].sin()),
                        "cos" if eval_args.len() == 1 => Some(eval_args[0].cos()),
                        "sqrt" if eval_args.len() == 1 => {
                            let v = eval_args[0];
                            if v < 0.0 { None } else { Some(v.sqrt()) }
                        }
                        "exp" if eval_args.len() == 1 => Some(eval_args[0].exp()),
                        "log" if eval_args.len() == 2 => {
                            let base = eval_args[0];
                            let value = eval_args[1];
                            if base <= 0.0 || value <= 0.0 { None } else { Some(value.log(base)) }
                        }
                        _ => None,
                    }
                } else {
                    None
                }
            }
            Expr::BinaryOp { op, left, right, .. } => {
                let l = eval_f64(left)?;
                let r = eval_f64(right)?;
                use crate::parser::ast::BinOp;
                match op {
                    BinOp::Add => Some(l + r),
                    BinOp::Sub => Some(l - r),
                    BinOp::Mul => Some(l * r),
                    BinOp::Div => {
                        if r == 0.0 { None } else { Some(l / r) }
                    }
                    BinOp::Pow => {
                        // si el exponente es negativo permitimos powf, pero
                        // mantendremos la compatibilidad retornando None si
                        // el resultado no es finito.
                        let res = l.powf(r);
                        if res.is_finite() { Some(res) } else { None }
                    }
                    BinOp::Mod => {
                        if r == 0.0 { None } else { Some(l % r) }
                    }
                    _ => None,
                }
            }
            Expr::UnaryOp { op, operand, .. } => {
                use crate::parser::ast::UnaryOp;
                match op {
                    UnaryOp::Neg => {
                        let v = eval_f64(operand)?;
                        Some(-v)
                    }
                    _ => None,
                }
            }
            _ => None,
        }
    }

    // Evaluar expr como f64 y convertir a i64 (truncando) para mantener
    // compatibilidad con las pruebas existentes que esperan enteros.
    let value_f = eval_f64(expr)?;
    Some(value_f as i64)
}
