mod lexer;
mod parser;

use crate::lexer::lexer::TokenStream;
use crate::parser::{Parser, Expr};

fn print_expr(expr: &Expr, indent: usize) {
	let pad = " ".repeat(indent);
	match expr {
		Expr::Number { value, .. } => println!("{}Number({})", pad, value),
		Expr::StringLit { value, .. } => println!("{}String(\"{}\")", pad, value),
		Expr::Bool { value, .. } => println!("{}Bool({})", pad, value),
		Expr::Ident { name, .. } => println!("{}Ident({})", pad, name),
		Expr::Call { callee, args, .. } => {
			println!("{}Call", pad);
			print_expr(callee, indent + 2);
			for arg in args {
				print_expr(arg, indent + 2);
			}
		}
		Expr::MethodCall { object, method, args, .. } => {
			println!("{}{}.{}(...)", pad, "object", method);
			print_expr(object, indent + 2);
			for arg in args {
				print_expr(arg, indent + 2);
			}
		}
		Expr::FieldAccess { object, field, .. } => {
			println!("{}{}.{}", pad, "object", field);
			print_expr(object, indent + 2);
		}
		Expr::New { type_name, args, .. } => {
			println!("{}new {}(...)", pad, type_name);
			for arg in args {
				print_expr(arg, indent + 2);
			}
		}
		Expr::SelfRef { .. } => println!("{}self", pad),
		Expr::Base { args, .. } => {
			println!("{}base(...)", pad);
			for arg in args {
				print_expr(arg, indent + 2);
			}
		}
		Expr::BinaryOp { op, left, right, .. } => {
			println!("{}BinOp({:?})", pad, op);
			print_expr(left, indent + 2);
			print_expr(right, indent + 2);
		}
		Expr::UnaryOp { op, operand, .. } => {
			println!("{}UnaryOp({:?})", pad, op);
			print_expr(operand, indent + 2);
		}
		Expr::IsType { expr: e, ty, .. } => {
			println!("{}is {:?}", pad, ty);
			print_expr(e, indent + 2);
		}
		Expr::AsType { expr: e, ty, .. } => {
			println!("{}as {:?}", pad, ty);
			print_expr(e, indent + 2);
		}
		Expr::If { condition, then_expr, elif_branches, else_expr, .. } => {
			println!("{}if", pad);
			print_expr(condition, indent + 2);
			print_expr(then_expr, indent + 2);
			for _elif in elif_branches {
				println!("{}elif", pad);
			}
			println!("{}else", pad);
			print_expr(else_expr, indent + 2);
		}
		Expr::While { condition, body, .. } => {
			println!("{}while", pad);
			print_expr(condition, indent + 2);
			print_expr(body, indent + 2);
		}
		Expr::For { var, iterable, body, .. } => {
			println!("{}for {} in ", pad, var);
			print_expr(iterable, indent + 2);
			print_expr(body, indent + 2);
		}
		Expr::Let { bindings, body, .. } => {
			println!("{}let", pad);
			for _binding in bindings {
				println!("{}  binding", pad);
			}
			print_expr(body, indent + 2);
		}
		Expr::Assign { target, value, .. } => {
			println!("{}{} := ", pad, target);
			print_expr(value, indent + 2);
		}
		Expr::Block { exprs, .. } => {
			println!("{}Block", pad);
			for e in exprs {
				print_expr(e, indent + 2);
			}
		}
		Expr::VectorLit { elements, .. } => {
			println!("{}Vector[...]", pad);
			for e in elements {
				print_expr(e, indent + 2);
			}
		}
		Expr::VectorGen { element, var, iterable, .. } => {
			println!("{}Vector[{} | {} in ...]", pad, "e", var);
			print_expr(element, indent + 2);
			print_expr(iterable, indent + 2);
		}
		Expr::Index { object, index, .. } => {
			println!("{}[index]", pad);
			print_expr(object, indent + 2);
			print_expr(index, indent + 2);
		}
	}
}

fn test_expression(src: &str) {
	println!("\n=== Test: Expression ===");
	println!("Source: {}", src);

	// Tokenization
	let (tokens, lex_errors) = TokenStream::tokenize_all(src);
	println!("\nTokens:");
	for t in &tokens {
		println!("  {:?} -> {}", t.token, t.span);
	}
	if !lex_errors.is_empty() {
		println!("\nLexer Errors:");
		for e in &lex_errors {
			println!("  {}", e);
		}
	}

	// Parse
	let ts = TokenStream::new(src);
	let mut parser = Parser::new(ts);
	match parser.parse_expr() {
		Some(expr) => {
			println!("\nAST:");
			print_expr(&expr, 0);
		}
		None => {
			println!("\nParser Error:");
			for e in parser.errors {
				println!("  {}", e);
			}
		}
	}
}

fn main() {
	test_expression("2 * (3 + 4)");
	test_expression("1 + 2 * 3 ^ 2");
	test_expression("[1, 2, 3]");
	test_expression("x | y");
	test_expression("a & b");
	test_expression("-5");
	test_expression("!true");
}

