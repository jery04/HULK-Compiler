mod lexer;
mod parser;

use crate::lexer::lexer::TokenStream;
use crate::parser::{Parser, Expression, Term, Factor};

fn print_expression(expr: &Expression, indent: usize) {
	let pad = " ".repeat(indent);
	match expr {
		Expression::Term(t) => {
			println!("{}Expression::Term", pad);
			print_term(t, indent + 2);
		}
		Expression::Binary { left, op, right } => {
			println!("{}Expression::Binary {:?}", pad, op);
			print_expression(left, indent + 2);
			print_expression(right, indent + 2);
		}
	}
}

fn print_term(term: &Term, indent: usize) {
	let pad = " ".repeat(indent);
	match term {
		Term::Factor(f) => {
			println!("{}Term::Factor", pad);
			print_factor(f, indent + 2);
		}
		Term::Binary { left, op, right } => {
			println!("{}Term::Binary {:?}", pad, op);
			print_term(left, indent + 2);
			print_term(right, indent + 2);
		}
	}
}

fn print_factor(f: &Factor, indent: usize) {
	let pad = " ".repeat(indent);
	match f {
		Factor::Number(n) => println!("{}Number({})", pad, n),
		Factor::Group(e) => {
			println!("{}Group", pad);
			print_expression(e, indent + 2);
		}
		Factor::Binary { left, op, right } => {
			println!("{}Factor::Binary {:?}", pad, op);
			print_factor(left, indent + 2);
			print_expression(right, indent + 2);
		}
		Factor::Ident(id) => println!("{}Ident({})", pad, id),
		Factor::Call { callee, args } => {
			println!("{}Call {}", pad, callee);
			for a in args {
				print_expression(a, indent + 2);
			}
		}
		Factor::BuiltinCall { func, args } => {
			println!("{}BuiltinCall {:?}", pad, func);
			for a in args {
				print_expression(a, indent + 2);
			}
		}
		Factor::Const(c) => println!("{}Const {:?}", pad, c),
	}
}

fn main() {
	let src = "sin(2 * PI) ^ 2 + cos(3 * PI / log(4, 64))";
	println!("Fuente: {}", src);

	// Tokenización (muestra tokens y errores léxicos)
	let (tokens, lex_errors) = TokenStream::tokenize_all(src);
	println!("\nTokens:");
	for t in &tokens {
		println!("  {:?} -> {}", t.token, t.span);
	}
	if !lex_errors.is_empty() {
		println!("\nErrores léxicos:");
		for e in &lex_errors {
			println!("  {}", e);
		}
	}

	// Parseo usando el parser
	let ts = TokenStream::new(src);
	let mut parser = Parser::new(ts);
	match parser.parse_expr() {
		Some(expr) => {
			println!("\nÁrbol AST:");
			print_expression(&expr, 0);
		}
		None => {
			println!("\nError al parsear. Errores:");
			for e in parser.errors {
				println!("  {}", e);
			}
		}
	}
}
