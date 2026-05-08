mod lexer;
mod parser;

use crate::lexer::lexer::TokenStream;
use crate::parser::{Parser, Expression, Term, Factor, FunctionDef, FunctionBody, Statement};

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
			print_factor(right, indent + 2);
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
		Factor::Unary { op, operand } => {
			println!("{}Unary {:?}", pad, op);
			print_factor(operand, indent + 2);
		}
	}
}

fn test_expression_example(src: &str) {
	println!("=== PRUEBA 1: Expresión Matemática ===");
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

fn print_function_def(func: &FunctionDef, indent: usize) {
	let pad = " ".repeat(indent);
	
	// Extraer el nombre del token
	if let crate::lexer::lexer::Token::Ident(name) = &func.name.token {
		println!("{}FunctionDef: {}", pad, name);
	}
	
	// Parámetros
	println!("{}  Parámetros:", pad);
	for param in &func.params {
		if let crate::lexer::lexer::Token::Ident(pname) = &param.name.token {
			let ptype = match &param.ty {
				Some(ty) => match &ty.token {
					crate::lexer::lexer::Token::Ident(name)
					| crate::lexer::lexer::Token::InternalIdent(name) => Some(name.clone()),
					crate::lexer::lexer::Token::TypNumber => Some("Number".to_string()),
					crate::lexer::lexer::Token::TypString => Some("String".to_string()),
					crate::lexer::lexer::Token::TypBool => Some("Boolean".to_string()),
					other => Some(format!("{:?}", other)),
				},
				None => None,
			};

			if let Some(ptype) = ptype {
				println!("{}    - {}: {}", pad, pname, ptype);
			} else {
				println!("{}    - {}", pad, pname);
			}
		}
	}
	
	// Cuerpo
	println!("{}  Cuerpo:", pad);
	match &func.body {
		FunctionBody::Inline(expr) => {
			println!("{}    Inline:", pad);
			print_expression(expr, indent + 6);
		}
		FunctionBody::Block(exprs) => {
			println!("{}    Block:", pad);
			for expr in exprs {
				print_expression(expr, indent + 6);
			}
		}
	}
}

fn test_function_definition() {
	let src = "function suma(a: Number, b: Number) => a + b;";
	println!("\n=== PRUEBA 2: Definición de Función ===");
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
	match parser.parse_function() {
		Some(func) => {
			println!("\nÁrbol AST:");
			print_function_def(&func, 0);
		}
		None => {
			println!("\nError al parsear. Errores:");
			for e in parser.errors {
				println!("  {}", e);
			}
		}
	}
}

fn test_let_in() {
	let src = "let a = 1, b = 2 in a + b;";
	println!("\n=== PRUEBA 3: Let-In ===");
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

	// Parseo usando el parser (usamos parse_let para manejar la sentencia let-in)
	let ts = TokenStream::new(src);
	let mut parser = Parser::new(ts);
	match parser.parse_let() {
		Some(stmt) => {
			println!("\nÁrbol AST:");
			print_statement(&stmt, 0);
		}
		None => {
			println!("\nError al parsear. Errores:");
			for e in parser.errors {
				println!("  {}", e);
			}
		}
	}
}

fn print_statement(stmt: &Statement, indent: usize) {
	let pad = " ".repeat(indent);
	match stmt {
		Statement::Assign { assignments, body } => {
			println!("{}Statement::Assign", pad);
			println!("{}  Assignments:", pad);
			for (name_tok, expr) in assignments {
				// extraer nombre
				if let crate::lexer::lexer::Token::Ident(n) = &name_tok.token {
					println!("{}    - {}:", pad, n);
				} else if let crate::lexer::lexer::Token::InternalIdent(n) = &name_tok.token {
					println!("{}    - {}:", pad, n);
				} else {
					println!("{}    - {:?}:", pad, name_tok.token);
				}
				print_expression(expr, indent + 6);
			}
			println!("{}  Body:", pad);
			print_expression(body, indent + 4);
		}
	}
}

fn main() {
	//test_expression_example("sin(2 * PI) ^ 2 + cos(3 * PI / log(4, 64))");
	//test_expression_example("2*(3+4)");
	//test_function_definition();
	test_let_in();
}

