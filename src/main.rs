#![allow(dead_code)]

mod lexer;
mod parser;
mod evaluator;

use crate::lexer::lexer::TokenStream;
use crate::parser::{Parser, Expr};
use crate::parser::{Decl, FuncBody, FuncDecl, Param, Program, TypeExpr};

fn print_type_expr(ty: &TypeExpr, indent: usize) {
	let pad = " ".repeat(indent);
	match ty {
		TypeExpr::Named(name) => println!("{}Type::Named({})", pad, name),
		TypeExpr::Iterable(inner) => {
			println!("{}Type::Iterable", pad);
			print_type_expr(inner, indent + 2);
		}
		TypeExpr::Vector(inner) => {
			println!("{}Type::Vector", pad);
			print_type_expr(inner, indent + 2);
		}
		TypeExpr::Functor { params, returns } => {
			println!("{}Type::Functor", pad);
			println!("{}  params", pad);
			for param in params {
				print_type_expr(param, indent + 4);
			}
			println!("{}  returns", pad);
			print_type_expr(returns, indent + 4);
		}
	}
}

fn print_param(param: &Param, indent: usize) {
	let pad = " ".repeat(indent);
	println!("{}Param", pad);
	println!("{}  name: {}", pad, param.name);
	match &param.ty {
		Some(ty) => {
			println!("{}  ty:", pad);
			print_type_expr(ty, indent + 2);
		}
		None => println!("{}  ty: None", pad),
	}
	println!("{}  span: {}", pad, param.span);
}

fn print_func_body(body: &FuncBody, indent: usize) {
	let pad = " ".repeat(indent);
	match body {
		FuncBody::Inline(expr) => {
			println!("{}FuncBody::Inline", pad);
			print_expr(expr, indent + 2);
		}
		FuncBody::Block(expr) => {
			println!("{}FuncBody::Block", pad);
			print_expr(expr, indent + 2);
		}
	}
}

fn print_func_decl(func: &FuncDecl, indent: usize) {
	let pad = " ".repeat(indent);
	println!("{}FuncDecl", pad);
	println!("{}  name: {}", pad, func.name);
	println!("{}  params:", pad);
	for param in &func.params {
		print_param(param, indent + 4);
	}
	match &func.return_type {
		Some(ty) => {
			println!("{}  return_type:", pad);
			print_type_expr(ty, indent + 4);
		}
		None => println!("{}  return_type: None", pad),
	}
	println!("{}  body:", pad);
	print_func_body(&func.body, indent + 4);
	println!("{}  span: {}", pad, func.span);
}

fn print_decl(decl: &Decl, indent: usize) {
	let pad = " ".repeat(indent);
	match decl {
		Decl::Function(func) => {
			println!("{}Decl::Function", pad);
			print_func_decl(func, indent + 2);
		}
		Decl::Type(_) => println!("{}Decl::Type(...)", pad),
		Decl::Protocol(_) => println!("{}Decl::Protocol(...)", pad),
		Decl::Macro(_) => println!("{}Decl::Macro(...)", pad),
	}
}

fn print_program(program: &Program) {
	println!("Program");
	println!("  decls:");
	for decl in &program.decls {
		print_decl(decl, 4);
	}
	println!("  expr:");
	print_expr(&program.expr, 4);
	println!("  span: {}", program.span);
}

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

fn test_program(src: &str) {
	println!("\n=== Test: Program ===");
	println!("Source: {}", src);

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

	let ts = TokenStream::new(src);
	let mut parser = Parser::new(ts);
	match parser.parse_program() {
		Some(program) => {
			println!("\nAST:");
			print_program(&program);
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

	// Función con bloque y while loop
	// Función con bloque y while loop
	test_program(r#"
	function sum_until(max : Number): Number => {
		let result = 0, i = 0 in
		while (i < max) {
			result := result + i;
			i := i + 1;
		};
		result
	};"#);

	// Suma con for e if/elif/else (multi-línea)
	test_program(r#"
	function sum_vec(v): Number => {
		let total = 0 in
		for (i in v) {
			if (i < 0) {
				total := total + (0 - i);
			} elif (i == 0) {
				total := total + 0;
			} else {
				total := total + i;
			};
		};
		total
	};"#);

	// Factorial con while y bloque anidado
	test_program(r#"
	function factorial(n: Number): Number => {
		let result = 1, i = 1 in {
			while (i <= n) {
				result := result * i;
				i := i + 1;
			};
			result
		}
	};"#);

	// Comprensión de vectores (multi-línea corta)
	test_program(r#"
	let evens = [ x * 2 | x in [1, 2, 3, 4, 5] ];
	evens;"#);

	// If / elif / else multi-línea
	test_program(r#"
	if (true) {
		1
	} elif (false) {
		2
	} else {
		3
	};"#);

	// Asignaciones dentro de bloques anidados
	test_program(r#"
	let a = 10 in {
		let b = 20 in {
			a := a + b;
			a
		}
	};"#);

	// Función que devuelve otra función (functor) y llamada encadenada
	test_program(r#"
	function make_adder(n): Function => {
		function (x): Number => { x + n }
	};
	make_adder(5)(3);"#);

	// Vector literal e indexación
	test_program(r#"let v = [1, 2, 3, 4] in v[2];"#);

	// Varias declaraciones y llamada final
	test_program(r#"
	function f(a, b): Number => { if (a > b) { a } else { b } };
	function g(): Number => {
		let r = f(10, 20) in
		r
	};
	g();"#);

	// Bloque con expresión final y asignación
	test_program(r#"
	{ let x = 1 in { x := x + 1; x } };"#);

	// Cadena y let multi-línea
	test_program(r#"let s = "hello" in {
		s
	};"#);

	// Doble for anidado con if (multi-línea más complejo)
	test_program(r#"
	function nested(a) : Number => {
		let sum = 0 in
		for (i in a) {
			for (j in i) {
				if (j % 2 == 0) { sum := sum + j } else { sum := sum + 0 };
			};
		};
		sum
	};"#);
}

