use std::collections::{HashMap, HashSet};

use crate::lexer::lexer::{Span, TokenStream};
use crate::parser::{Decl, Expr, FuncBody, FuncDecl, Param, Parser, Program, TypeExpr};
use crate::semantic;

#[derive(Clone)]
struct TreePrinter {
    prefix: String,
    depth: usize,
    is_last: bool,
}

impl TreePrinter {
    fn root() -> Self {
        Self {
            prefix: String::new(),
            depth: 0,
            is_last: true,
        }
    }

    fn child(&self, is_last: bool) -> Self {
        let mut prefix = self.prefix.clone();
        if self.depth > 0 {
            if self.is_last {
                prefix.push_str("    ");
            } else {
                prefix.push_str("|   ");
            }
        }
        Self {
            prefix,
            depth: self.depth + 1,
            is_last,
        }
    }

    fn line(&self, label: &str) {
        if self.depth == 0 {
            println!("{}", label);
        } else {
            let connector = if self.is_last { "`-- " } else { "|-- " };
            println!("{}{}{}", self.prefix, connector, label);
        }
    }
}

fn print_type_expr(ty: &TypeExpr, printer: &TreePrinter) {
    match ty {
        TypeExpr::Named(name) => printer.line(&format!("Type::Named({})", name)),
        TypeExpr::Iterable(inner) => {
            printer.line("Type::Iterable");
            let child = printer.child(true);
            print_type_expr(inner, &child);
        }
        TypeExpr::Vector(inner) => {
            printer.line("Type::Vector");
            let child = printer.child(true);
            print_type_expr(inner, &child);
        }
        TypeExpr::Functor { params, returns } => {
            printer.line("Type::Functor");

            let params_printer = printer.child(false);
            if params.is_empty() {
                params_printer.line("params: []");
            } else {
                params_printer.line("params");
                for (idx, param) in params.iter().enumerate() {
                    let child = params_printer.child(idx + 1 == params.len());
                    print_type_expr(param, &child);
                }
            }

            let returns_printer = printer.child(true);
            returns_printer.line("returns");
            let child = returns_printer.child(true);
            print_type_expr(returns, &child);
        }
    }
}

fn print_span(span: Span, printer: &TreePrinter) {
    printer.line(&format!("span: {}", span));
}

fn build_type_decl_map<'a>(program: &'a Program) -> HashMap<String, &'a crate::parser::TypeDecl> {
    let mut type_decl_map = HashMap::new();

    for decl in &program.decls {
        if let Decl::Type(ty) = decl {
            type_decl_map.insert(ty.name.clone(), ty);
        }
    }

    type_decl_map
}

fn type_member_method_names(ty: &crate::parser::TypeDecl) -> Vec<String> {
    ty.members
        .iter()
        .filter_map(|member| match member {
            crate::parser::TypeMember::Method(method) => Some(method.name.clone()),
            _ => None,
        })
        .collect()
}

fn collect_inherited_methods<'a>(
    ty: &'a crate::parser::TypeDecl,
    type_decl_map: &HashMap<String, &'a crate::parser::TypeDecl>,
    visited: &mut HashSet<String>,
    inherited_methods: &mut Vec<(String, Vec<String>)>,
) {
    let Some(inherits) = &ty.inherits else {
        return;
    };

    let parent_name = inherits.parent.clone();
    if !visited.insert(parent_name.clone()) {
        return;
    }

    if let Some(parent_ty) = type_decl_map.get(&parent_name) {
        inherited_methods.push((parent_name.clone(), type_member_method_names(parent_ty)));
        collect_inherited_methods(parent_ty, type_decl_map, visited, inherited_methods);
    }
}

fn print_type_decl(
    ty: &crate::parser::TypeDecl,
    type_decl_map: &HashMap<String, &crate::parser::TypeDecl>,
    printer: &TreePrinter,
) {
    printer.line("TypeDecl");

    let name_printer = printer.child(false);
    name_printer.line(&format!("name: {}", ty.name));

    let type_params_printer = printer.child(false);
    if ty.type_params.is_empty() {
        type_params_printer.line("type_params: []");
    } else {
        type_params_printer.line("type_params");
        for (idx, param) in ty.type_params.iter().enumerate() {
            let child = type_params_printer.child(idx + 1 == ty.type_params.len());
            print_param(param, &child);
        }
    }

    let inherits_printer = printer.child(false);
    match &ty.inherits {
        Some(ic) => {
            inherits_printer.line("inherits");

            let parent_printer = inherits_printer.child(false);
            parent_printer.line(&format!("parent: {}", ic.parent));

            let args_printer = inherits_printer.child(false);
            if ic.args.is_empty() {
                args_printer.line("args: []");
            } else {
                args_printer.line("args");
                for (idx, arg) in ic.args.iter().enumerate() {
                    let child = args_printer.child(idx + 1 == ic.args.len());
                    print_expr(arg, &child);
                }
            }

            let span_printer = inherits_printer.child(true);
            print_span(ic.span, &span_printer);
        }
        None => inherits_printer.line("inherits: None"),
    }

    let members_printer = printer.child(false);
    if ty.members.is_empty() {
        members_printer.line("members: []");
    } else {
        members_printer.line("members");
        for (idx, member) in ty.members.iter().enumerate() {
            let child = members_printer.child(idx + 1 == ty.members.len());
            match member {
                crate::parser::TypeMember::Attribute(attr) => {
                    child.line("Attribute");

                    let name_printer = child.child(false);
                    name_printer.line(&format!("name: {}", attr.name));

                    let ty_printer = child.child(false);
                    match &attr.ty {
                        Some(attr_ty) => {
                            ty_printer.line("ty");
                            let inner = ty_printer.child(true);
                            print_type_expr(attr_ty, &inner);
                        }
                        None => ty_printer.line("ty: None"),
                    }

                    let init_printer = child.child(false);
                    init_printer.line("init");
                    let init_child = init_printer.child(true);
                    print_expr(&attr.init, &init_child);

                    let span_printer = child.child(true);
                    print_span(attr.span, &span_printer);
                }
                crate::parser::TypeMember::Method(method) => {
                    child.line("Method");
                    let method_printer = child.child(true);
                    print_method_def(method, &method_printer);
                }
            }
        }
    }

    let inherited_methods_printer = printer.child(false);
    let mut visited = HashSet::new();
    let mut inherited_methods = Vec::new();
    collect_inherited_methods(ty, type_decl_map, &mut visited, &mut inherited_methods);
    if inherited_methods.is_empty() {
        inherited_methods_printer.line("inherited_methods: []");
    } else {
        inherited_methods_printer.line("inherited_methods");
        for (idx, (ancestor_name, method_names)) in inherited_methods.iter().enumerate() {
            let ancestor_printer = inherited_methods_printer.child(idx + 1 == inherited_methods.len());
            ancestor_printer.line(&format!("ancestor: {}", ancestor_name));

            let methods_printer = ancestor_printer.child(true);
            if method_names.is_empty() {
                methods_printer.line("methods: []");
            } else {
                methods_printer.line("methods");
                for (method_idx, method_name) in method_names.iter().enumerate() {
                    let method_printer = methods_printer.child(method_idx + 1 == method_names.len());
                    method_printer.line(method_name);
                }
            }
        }
    }

    let span_printer = printer.child(true);
    print_span(ty.span, &span_printer);
}

fn print_method_def(m: &crate::parser::MethodDef, printer: &TreePrinter) {
    printer.line("MethodDef");

    let name_printer = printer.child(false);
    name_printer.line(&format!("name: {}", m.name));

    let params_printer = printer.child(false);
    if m.params.is_empty() {
        params_printer.line("params: []");
    } else {
        params_printer.line("params");
        for (idx, param) in m.params.iter().enumerate() {
            let child = params_printer.child(idx + 1 == m.params.len());
            print_param(param, &child);
        }
    }

    let return_printer = printer.child(false);
    match &m.return_type {
        Some(ty) => {
            return_printer.line("return_type");
            let child = return_printer.child(true);
            print_type_expr(ty, &child);
        }
        None => return_printer.line("return_type: None"),
    }

    let body_printer = printer.child(false);
    body_printer.line("body");
    let body_child = body_printer.child(true);
    print_func_body(&m.body, &body_child);

    let span_printer = printer.child(true);
    print_span(m.span, &span_printer);
}

fn print_protocol_decl(p: &crate::parser::ProtocolDecl, printer: &TreePrinter) {
    printer.line("ProtocolDecl");

    let name_printer = printer.child(false);
    name_printer.line(&format!("name: {}", p.name));

    let extends_printer = printer.child(false);
    extends_printer.line(&format!(
        "extends: {}",
        p.extends.as_deref().unwrap_or("None")
    ));

    let methods_printer = printer.child(false);
    if p.methods.is_empty() {
        methods_printer.line("methods: []");
    } else {
        methods_printer.line("methods");
        for (idx, method) in p.methods.iter().enumerate() {
            let child = methods_printer.child(idx + 1 == p.methods.len());
            child.line("MethodSig");

            let name_printer = child.child(false);
            name_printer.line(&format!("name: {}", method.name));

            let params_printer = child.child(false);
            if method.params.is_empty() {
                params_printer.line("params: []");
            } else {
                params_printer.line("params");
                for (sp_idx, sp) in method.params.iter().enumerate() {
                    let sp_child = params_printer.child(sp_idx + 1 == method.params.len());
                    sp_child.line("SigParam");

                    let sp_name_printer = sp_child.child(false);
                    sp_name_printer.line(&format!("name: {}", sp.name));

                    let sp_ty_printer = sp_child.child(false);
                    match &sp.ty {
                        Some(ty) => {
                            sp_ty_printer.line("ty");
                            let inner = sp_ty_printer.child(true);
                            print_type_expr(ty, &inner);
                        }
                        None => sp_ty_printer.line("ty: None"),
                    }

                    let sp_span_printer = sp_child.child(true);
                    print_span(sp.span, &sp_span_printer);
                }
            }

            let return_printer = child.child(false);
            return_printer.line("return_type");
            let return_child = return_printer.child(true);
            print_type_expr(&method.return_type, &return_child);

            let span_printer = child.child(true);
            print_span(method.span, &span_printer);
        }
    }

    let span_printer = printer.child(true);
    print_span(p.span, &span_printer);
}

fn print_macro_decl(m: &crate::parser::MacroDecl, printer: &TreePrinter) {
    printer.line("MacroDecl");

    let name_printer = printer.child(false);
    name_printer.line(&format!("name: {}", m.name));

    let params_printer = printer.child(false);
    if m.params.is_empty() {
        params_printer.line("params: []");
    } else {
        params_printer.line("params");
        for (idx, mp) in m.params.iter().enumerate() {
            let child = params_printer.child(idx + 1 == m.params.len());
            match mp {
                crate::parser::MacroParam::Regular(param) => {
                    child.line("Regular");
                    let inner = child.child(true);
                    print_param(param, &inner);
                }
                crate::parser::MacroParam::Block { name, ty, span } => {
                    child.line("Block");

                    let name_printer = child.child(false);
                    name_printer.line(&format!("name: {}", name));

                    let ty_printer = child.child(false);
                    ty_printer.line("ty");
                    let ty_child = ty_printer.child(true);
                    print_type_expr(ty, &ty_child);

                    let span_printer = child.child(true);
                    print_span(*span, &span_printer);
                }
                crate::parser::MacroParam::Symbolic { name, ty, span } => {
                    child.line("Symbolic");

                    let name_printer = child.child(false);
                    name_printer.line(&format!("name: {}", name));

                    let ty_printer = child.child(false);
                    ty_printer.line("ty");
                    let ty_child = ty_printer.child(true);
                    print_type_expr(ty, &ty_child);

                    let span_printer = child.child(true);
                    print_span(*span, &span_printer);
                }
                crate::parser::MacroParam::Placeholder { name, ty, span } => {
                    child.line("Placeholder");

                    let name_printer = child.child(false);
                    name_printer.line(&format!("name: {}", name));

                    let ty_printer = child.child(false);
                    ty_printer.line("ty");
                    let ty_child = ty_printer.child(true);
                    print_type_expr(ty, &ty_child);

                    let span_printer = child.child(true);
                    print_span(*span, &span_printer);
                }
            }
        }
    }

    let body_printer = printer.child(false);
    body_printer.line("body");
    let body_child = body_printer.child(true);
    print_func_body(&m.body, &body_child);

    let span_printer = printer.child(true);
    print_span(m.span, &span_printer);
}

fn print_param(param: &Param, printer: &TreePrinter) {
    printer.line("Param");

    let name_printer = printer.child(false);
    name_printer.line(&format!("name: {}", param.name));

    let ty_printer = printer.child(false);
    match &param.ty {
        Some(ty) => {
            ty_printer.line("ty");
            let inner = ty_printer.child(true);
            print_type_expr(ty, &inner);
        }
        None => ty_printer.line("ty: None"),
    }

    let span_printer = printer.child(true);
    print_span(param.span, &span_printer);
}

fn print_func_body(body: &FuncBody, printer: &TreePrinter) {
    match body {
        FuncBody::Inline(expr) => {
            printer.line("FuncBody::Inline");
            let child = printer.child(true);
            print_expr(expr, &child);
        }
        FuncBody::Block(expr) => {
            printer.line("FuncBody::Block");
            let child = printer.child(true);
            print_expr(expr, &child);
        }
    }
}

fn print_func_decl(func: &FuncDecl, printer: &TreePrinter) {
    printer.line("FuncDecl");

    let name_printer = printer.child(false);
    name_printer.line(&format!("name: {}", func.name));

    let params_printer = printer.child(false);
    if func.params.is_empty() {
        params_printer.line("params: []");
    } else {
        params_printer.line("params");
        for (idx, param) in func.params.iter().enumerate() {
            let child = params_printer.child(idx + 1 == func.params.len());
            print_param(param, &child);
        }
    }

    let return_printer = printer.child(false);
    match &func.return_type {
        Some(ty) => {
            return_printer.line("return_type");
            let child = return_printer.child(true);
            print_type_expr(ty, &child);
        }
        None => return_printer.line("return_type: None"),
    }

    let body_printer = printer.child(false);
    body_printer.line("body");
    let body_child = body_printer.child(true);
    print_func_body(&func.body, &body_child);

    let span_printer = printer.child(true);
    print_span(func.span, &span_printer);
}

fn print_decl(
    decl: &Decl,
    type_decl_map: &HashMap<String, &crate::parser::TypeDecl>,
    printer: &TreePrinter,
) {
    match decl {
        Decl::Function(func) => {
            printer.line("Decl::Function");
            let child = printer.child(true);
            print_func_decl(func, &child);
        }
        Decl::Type(ty) => {
            printer.line("Decl::Type");
            let child = printer.child(true);
            print_type_decl(ty, type_decl_map, &child);
        }
        Decl::Protocol(protocol) => {
            printer.line("Decl::Protocol");
            let child = printer.child(true);
            print_protocol_decl(protocol, &child);
        }
        Decl::Macro(mac) => {
            printer.line("Decl::Macro");
            let child = printer.child(true);
            print_macro_decl(mac, &child);
        }
    }
}

fn print_program(program: &Program) {
    let printer = TreePrinter::root();
    printer.line("Program");
    let type_decl_map = build_type_decl_map(program);

    let decls_printer = printer.child(false);
    if program.decls.is_empty() {
        decls_printer.line("decls: []");
    } else {
        decls_printer.line("decls");
        for (idx, decl) in program.decls.iter().enumerate() {
            let child = decls_printer.child(idx + 1 == program.decls.len());
            print_decl(decl, &type_decl_map, &child);
        }
    }

    let expr_printer = printer.child(false);
    expr_printer.line("expr");
    let expr_child = expr_printer.child(true);
    print_expr(&program.expr, &expr_child);

    let span_printer = printer.child(true);
    print_span(program.span, &span_printer);
}

fn print_expr(expr: &Expr, printer: &TreePrinter) {
    match expr {
        Expr::Number { value, span } => {
            printer.line(&format!("Number({})", value));
            let span_printer = printer.child(true);
            print_span(*span, &span_printer);
        }
        Expr::StringLit { value, span } => {
            printer.line(&format!("String(\"{}\")", value));
            let span_printer = printer.child(true);
            print_span(*span, &span_printer);
        }
        Expr::Bool { value, span } => {
            printer.line(&format!("Bool({})", value));
            let span_printer = printer.child(true);
            print_span(*span, &span_printer);
        }
        Expr::Ident { name, span } => {
            printer.line(&format!("Ident({})", name));
            let span_printer = printer.child(true);
            print_span(*span, &span_printer);
        }
        Expr::Call { callee, args, span } => {
            printer.line("Call");

            let callee_printer = printer.child(false);
            callee_printer.line("callee");
            let callee_child = callee_printer.child(true);
            print_expr(callee, &callee_child);

            let args_printer = printer.child(false);
            if args.is_empty() {
                args_printer.line("args: []");
            } else {
                args_printer.line("args");
                for (idx, arg) in args.iter().enumerate() {
                    let child = args_printer.child(idx + 1 == args.len());
                    print_expr(arg, &child);
                }
            }

            let span_printer = printer.child(true);
            print_span(*span, &span_printer);
        }
        Expr::MethodCall {
            object,
            method,
            args,
            span,
        } => {
            printer.line(&format!("MethodCall({})", method));

            let object_printer = printer.child(false);
            object_printer.line("object");
            let object_child = object_printer.child(true);
            print_expr(object, &object_child);

            let args_printer = printer.child(false);
            if args.is_empty() {
                args_printer.line("args: []");
            } else {
                args_printer.line("args");
                for (idx, arg) in args.iter().enumerate() {
                    let child = args_printer.child(idx + 1 == args.len());
                    print_expr(arg, &child);
                }
            }

            let span_printer = printer.child(true);
            print_span(*span, &span_printer);
        }
        Expr::FieldAccess { object, field, span } => {
            printer.line(&format!("FieldAccess({})", field));

            let object_printer = printer.child(false);
            object_printer.line("object");
            let object_child = object_printer.child(true);
            print_expr(object, &object_child);

            let span_printer = printer.child(true);
            print_span(*span, &span_printer);
        }
        Expr::New {
            type_name,
            args,
            span,
        } => {
            printer.line(&format!("New({})", type_name));

            let args_printer = printer.child(false);
            if args.is_empty() {
                args_printer.line("args: []");
            } else {
                args_printer.line("args");
                for (idx, arg) in args.iter().enumerate() {
                    let child = args_printer.child(idx + 1 == args.len());
                    print_expr(arg, &child);
                }
            }

            let span_printer = printer.child(true);
            print_span(*span, &span_printer);
        }
        
        Expr::Base { args, span } => {
            printer.line("Base");

            let args_printer = printer.child(false);
            if args.is_empty() {
                args_printer.line("args: []");
            } else {
                args_printer.line("args");
                for (idx, arg) in args.iter().enumerate() {
                    let child = args_printer.child(idx + 1 == args.len());
                    print_expr(arg, &child);
                }
            }

            let span_printer = printer.child(true);
            print_span(*span, &span_printer);
        }
        Expr::BinaryOp {
            op,
            left,
            right,
            span,
        } => {
            printer.line(&format!("BinaryOp({:?})", op));

            let left_printer = printer.child(false);
            left_printer.line("left");
            let left_child = left_printer.child(true);
            print_expr(left, &left_child);

            let right_printer = printer.child(false);
            right_printer.line("right");
            let right_child = right_printer.child(true);
            print_expr(right, &right_child);

            let span_printer = printer.child(true);
            print_span(*span, &span_printer);
        }
        Expr::UnaryOp { op, operand, span } => {
            printer.line(&format!("UnaryOp({:?})", op));

            let operand_printer = printer.child(false);
            operand_printer.line("operand");
            let operand_child = operand_printer.child(true);
            print_expr(operand, &operand_child);

            let span_printer = printer.child(true);
            print_span(*span, &span_printer);
        }
        Expr::IsType { expr: e, ty, span } => {
            printer.line("IsType");

            let expr_printer = printer.child(false);
            expr_printer.line("expr");
            let expr_child = expr_printer.child(true);
            print_expr(e, &expr_child);

            let ty_printer = printer.child(false);
            ty_printer.line("ty");
            let ty_child = ty_printer.child(true);
            print_type_expr(ty, &ty_child);

            let span_printer = printer.child(true);
            print_span(*span, &span_printer);
        }
        Expr::AsType { expr: e, ty, span } => {
            printer.line("AsType");

            let expr_printer = printer.child(false);
            expr_printer.line("expr");
            let expr_child = expr_printer.child(true);
            print_expr(e, &expr_child);

            let ty_printer = printer.child(false);
            ty_printer.line("ty");
            let ty_child = ty_printer.child(true);
            print_type_expr(ty, &ty_child);

            let span_printer = printer.child(true);
            print_span(*span, &span_printer);
        }
        Expr::If {
            condition,
            then_expr,
            elif_branches,
            else_expr,
            span,
        } => {
            printer.line("If");

            let condition_printer = printer.child(false);
            condition_printer.line("condition");
            let condition_child = condition_printer.child(true);
            print_expr(condition, &condition_child);

            let then_printer = printer.child(false);
            then_printer.line("then");
            let then_child = then_printer.child(true);
            print_expr(then_expr, &then_child);

            let elif_printer = printer.child(false);
            if elif_branches.is_empty() {
                elif_printer.line("elif_branches: []");
            } else {
                elif_printer.line("elif_branches");
                for (idx, eb) in elif_branches.iter().enumerate() {
                    let eb_printer = elif_printer.child(idx + 1 == elif_branches.len());
                    eb_printer.line("ElifBranch");

                    let eb_condition = eb_printer.child(false);
                    eb_condition.line("condition");
                    let eb_condition_child = eb_condition.child(true);
                    print_expr(&eb.condition, &eb_condition_child);

                    let eb_body = eb_printer.child(false);
                    eb_body.line("body");
                    let eb_body_child = eb_body.child(true);
                    print_expr(&eb.body, &eb_body_child);

                    let eb_span = eb_printer.child(true);
                    print_span(eb.span, &eb_span);
                }
            }

            let else_printer = printer.child(false);
            else_printer.line("else");
            let else_child = else_printer.child(true);
            print_expr(else_expr, &else_child);

            let span_printer = printer.child(true);
            print_span(*span, &span_printer);
        }
        Expr::While { condition, body, span } => {
            printer.line("While");

            let condition_printer = printer.child(false);
            condition_printer.line("condition");
            let condition_child = condition_printer.child(true);
            print_expr(condition, &condition_child);

            let body_printer = printer.child(false);
            body_printer.line("body");
            let body_child = body_printer.child(true);
            print_expr(body, &body_child);

            let span_printer = printer.child(true);
            print_span(*span, &span_printer);
        }
        Expr::For {
            var,
            iterable,
            body,
            span,
        } => {
            printer.line(&format!("For(var={})", var));

            let iterable_printer = printer.child(false);
            iterable_printer.line("iterable");
            let iterable_child = iterable_printer.child(true);
            print_expr(iterable, &iterable_child);

            let body_printer = printer.child(false);
            body_printer.line("body");
            let body_child = body_printer.child(true);
            print_expr(body, &body_child);

            let span_printer = printer.child(true);
            print_span(*span, &span_printer);
        }
        Expr::Let { bindings, body, span } => {
            printer.line("Let");

            let bindings_printer = printer.child(false);
            if bindings.is_empty() {
                bindings_printer.line("bindings: []");
            } else {
                bindings_printer.line("bindings");
                for (idx, binding) in bindings.iter().enumerate() {
                    let binding_printer = bindings_printer.child(idx + 1 == bindings.len());
                    binding_printer.line("LetBinding");

                    let name_printer = binding_printer.child(false);
                    name_printer.line(&format!("name: {}", binding.name));

                    let ty_printer = binding_printer.child(false);
                    match &binding.ty {
                        Some(ty) => {
                            ty_printer.line("ty");
                            let ty_child = ty_printer.child(true);
                            print_type_expr(ty, &ty_child);
                        }
                        None => ty_printer.line("ty: None"),
                    }

                    let init_printer = binding_printer.child(false);
                    init_printer.line("init");
                    let init_child = init_printer.child(true);
                    print_expr(&binding.init, &init_child);

                    let span_printer = binding_printer.child(true);
                    print_span(binding.span, &span_printer);
                }
            }

            let body_printer = printer.child(false);
            body_printer.line("body");
            let body_child = body_printer.child(true);
            print_expr(body, &body_child);

            let span_printer = printer.child(true);
            print_span(*span, &span_printer);
        }
        Expr::Assign { target, value, span } => {
            printer.line("Assign");

            let target_printer = printer.child(false);
            target_printer.line("target");
            let target_child = target_printer.child(true);
            print_expr(target, &target_child);

            let value_printer = printer.child(false);
            value_printer.line("value");
            let value_child = value_printer.child(true);
            print_expr(value, &value_child);

            let span_printer = printer.child(true);
            print_span(*span, &span_printer);
        }
        Expr::Block { exprs, span } => {
            printer.line("Block");

            let exprs_printer = printer.child(false);
            if exprs.is_empty() {
                exprs_printer.line("exprs: []");
            } else {
                exprs_printer.line("exprs");
                for (idx, e) in exprs.iter().enumerate() {
                    let child = exprs_printer.child(idx + 1 == exprs.len());
                    print_expr(e, &child);
                }
            }

            let span_printer = printer.child(true);
            print_span(*span, &span_printer);
        }
        Expr::VectorLit { elements, span } => {
            printer.line("VectorLit");

            let elements_printer = printer.child(false);
            if elements.is_empty() {
                elements_printer.line("elements: []");
            } else {
                elements_printer.line("elements");
                for (idx, element) in elements.iter().enumerate() {
                    let child = elements_printer.child(idx + 1 == elements.len());
                    print_expr(element, &child);
                }
            }

            let span_printer = printer.child(true);
            print_span(*span, &span_printer);
        }
        Expr::VectorGen {
            element,
            var,
            iterable,
            span,
        } => {
            printer.line(&format!("VectorGen(var={})", var));

            let element_printer = printer.child(false);
            element_printer.line("element");
            let element_child = element_printer.child(true);
            print_expr(element, &element_child);

            let iterable_printer = printer.child(false);
            iterable_printer.line("iterable");
            let iterable_child = iterable_printer.child(true);
            print_expr(iterable, &iterable_child);

            let span_printer = printer.child(true);
            print_span(*span, &span_printer);
        }
        Expr::Index { object, index, span } => {
            printer.line("Index");

            let object_printer = printer.child(false);
            object_printer.line("object");
            let object_child = object_printer.child(true);
            print_expr(object, &object_child);

            let index_printer = printer.child(false);
            index_printer.line("index");
            let index_child = index_printer.child(true);
            print_expr(index, &index_child);

            let span_printer = printer.child(true);
            print_span(*span, &span_printer);
        }
        Expr::Lambda {
            params,
            return_type,
            body,
            span,
        } => {
            printer.line("Lambda");

            let params_printer = printer.child(false);
            if params.is_empty() {
                params_printer.line("params: []");
            } else {
                params_printer.line("params");
                for (idx, param) in params.iter().enumerate() {
                    let child = params_printer.child(idx + 1 == params.len());
                    print_param(param, &child);
                }
            }

            let return_printer = printer.child(false);
            match return_type {
                Some(ty) => {
                    return_printer.line("return_type");
                    let child = return_printer.child(true);
                    print_type_expr(ty, &child);
                }
                None => return_printer.line("return_type: None"),
            }

            let body_printer = printer.child(false);
            body_printer.line("body");
            let body_child = body_printer.child(true);
            print_func_body(body, &body_child);

            let span_printer = printer.child(true);
            print_span(*span, &span_printer);
        }
        Expr::Error { span } => {
            printer.line("ErrorExpr");
            let span_printer = printer.child(true);
            print_span(*span, &span_printer);
        }
    }
}

pub fn test_expression(src: &str) {
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
    let expr_result = parser.parse_expr();
    if !parser.errors.is_empty() {
        println!("\nParser Errors:");
        for e in &parser.errors {
            println!("  {}", e);
        }
    }

    match expr_result {
        Some(expr) => {
            println!("\nAST:");
            let printer = TreePrinter::root();
            printer.line("Expr");
            let child = printer.child(true);
            print_expr(&expr, &child);
        }
        None => {
            println!("\nParser Error: no se pudo construir la expresion");
        }
    }
}

pub fn test_program(skip: bool, src: &str) {
    // Si skip es false, no hacemos nada
    if !skip {
        println!("Test skipped.");
        return;
    }

    println!("\n=== Test: Program ===");
    println!("Source: {}", src);

    let (tokens, lex_errors) = TokenStream::tokenize_all(src);
    println!("\nTokens:");
    for t in &tokens {
        println!("  {:?} -> {}", t.token, t.span);
    }

    let has_lex_errors = !lex_errors.is_empty();
    if has_lex_errors {
        println!("\nLexer Errors:");
        for e in &lex_errors {
            println!("  {}", e);
        }
    }

    let ts = TokenStream::new(src);
    let mut parser = Parser::new(ts);
    let program_result = parser.parse_program();

    let has_parse_errors = !parser.errors.is_empty();
    if has_parse_errors {
        println!("\nParser Errors:");
        for e in &parser.errors {
            println!("  {}", e);
        }
    }

    // Si hay errores, no seguimos
    if has_lex_errors || has_parse_errors {
        println!("\nParser Error: no se pudo construir el programa");
        return;
    }

    let program = program_result.unwrap();
    let semantic_errors = semantic::check_program(&program);
    if !semantic_errors.is_empty() {
        println!("\nSemantic Errors:");
        for e in &semantic_errors {
            println!("  {}", e);
        }
        println!("\nSemantic Error: no se pudo validar el programa");
        return;
    }

    // Aquí ya NO puede haber errores
    println!("\nAST:");
    print_program(&program);
}
