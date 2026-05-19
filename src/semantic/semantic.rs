use std::collections::{HashMap, HashSet};

use crate::lexer::lexer::Span;
use crate::parser::ast::*;

#[derive(Debug, Clone)]
pub struct SemanticError {
    pub span: Span,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum SimpleType {
    Number,
    String,
    Boolean,
    Vector(Box<SimpleType>),
}

impl SimpleType {
    fn display_name(&self) -> String {
        match self {
            SimpleType::Number => "Number".to_string(),
            SimpleType::String => "String".to_string(),
            SimpleType::Boolean => "Boolean".to_string(),
            SimpleType::Vector(inner) => format!("Vector<{}>", inner.display_name()),
        }
    }
}

impl std::fmt::Display for SemanticError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[SemanticError {}] {}", self.span, self.message)
    }
}

pub fn check_program(program: &Program) -> Vec<SemanticError> {
    let mut checker = SemanticChecker::new();
    checker.check_program(program);
    checker.errors
}

pub struct SemanticChecker {
    ctx: Context,
    errors: Vec<SemanticError>,
}

impl SemanticChecker {
    pub fn new() -> Self {
        Self {
            ctx: Context::new(),
            errors: Vec::new(),
        }
    }

    pub fn check_program(&mut self, program: &Program) {
        for decl in &program.decls {
            self.predeclare_decl(decl);
        }
        for decl in &program.decls {
            self.check_decl(decl);
        }
        self.check_expr(&program.expr);
    }

    fn predeclare_decl(&mut self, decl: &Decl) {
        match decl {
            Decl::Function(d) => self.predeclare_func_decl(d),
            Decl::Type(d) => self.predeclare_type_decl(d),
            Decl::Protocol(d) => self.predeclare_protocol_decl(d),
            Decl::Macro(d) => self.predeclare_macro_decl(d),
        }
    }

    fn check_decl(&mut self, decl: &Decl) {
        match decl {
            Decl::Function(d) => self.check_func_decl(d),
            Decl::Type(d) => self.check_type_decl(d),
            Decl::Protocol(d) => self.check_protocol_decl(d),
            Decl::Macro(d) => self.check_macro_decl(d),
        }
    }

    fn predeclare_func_decl(&mut self, decl: &FuncDecl) {
        if is_placeholder(&decl.name) {
            return;
        }

        if self.ctx.is_builtin_function(&decl.name) {
            self.report(decl.span, format!(
                "funcion predefinida '{}' no se puede redefinir",
                decl.name
            ));
            return;
        }

        if self.ctx.has_function_name(&decl.name) {
            self.report(decl.span, format!(
                "funcion '{}' ya definida",
                decl.name
            ));
        }

        self.ctx.insert_function(&decl.name, decl.params.len());
    }

    fn predeclare_type_decl(&mut self, decl: &TypeDecl) {
        if is_placeholder(&decl.name) {
            return;
        }

        if self.ctx.is_builtin_type(&decl.name) {
            self.report(decl.span, format!(
                "tipo predefinido '{}' no se puede redefinir",
                decl.name
            ));
            return;
        }

        if self.ctx.is_type_or_protocol_defined(&decl.name) {
            self.report(decl.span, format!(
                "tipo o protocolo '{}' ya definido",
                decl.name
            ));
            return;
        }

        self.ctx.insert_type(&decl.name, decl.type_params.len());
    }

    fn predeclare_protocol_decl(&mut self, decl: &ProtocolDecl) {
        if is_placeholder(&decl.name) {
            return;
        }

        if self.ctx.is_builtin_type(&decl.name) {
            self.report(decl.span, format!(
                "protocolo '{}' entra en conflicto con tipo predefinido",
                decl.name
            ));
            return;
        }

        if self.ctx.is_type_or_protocol_defined(&decl.name) {
            self.report(decl.span, format!(
                "tipo o protocolo '{}' ya definido",
                decl.name
            ));
            return;
        }

        self.ctx.insert_protocol(&decl.name);
    }

    fn predeclare_macro_decl(&mut self, decl: &MacroDecl) {
        if is_placeholder(&decl.name) {
            return;
        }

        if self.ctx.is_builtin_function(&decl.name) {
            self.report(decl.span, format!(
                "macro '{}' entra en conflicto con funcion predefinida",
                decl.name
            ));
            return;
        }

        if self.ctx.has_macro_name(&decl.name) {
            self.report(decl.span, format!(
                "macro '{}' ya definida",
                decl.name
            ));
        }

        self.ctx.insert_macro(&decl.name, decl.params.len());
    }

    fn check_func_decl(&mut self, decl: &FuncDecl) {
        if is_placeholder(&decl.name) {
            return;
        }

        let mut seen = HashSet::new();
        for param in &decl.params {
            self.check_param(param, &mut seen);
        }
        if let Some(ty) = &decl.return_type {
            self.check_type_expr(ty, decl.span);
        }

        self.ctx.push_scope();
        for param in &decl.params {
            self.define_var(&param.name, param.span);
        }
        self.check_func_body(&decl.body);
        self.ctx.pop_scope();
    }

    fn check_type_decl(&mut self, decl: &TypeDecl) {
        if is_placeholder(&decl.name) {
            return;
        }

        let mut type_param_names = HashSet::new();
        for param in &decl.type_params {
            self.check_param(param, &mut type_param_names);
        }

        self.ctx.push_scope();
        for param in &decl.type_params {
            self.define_var(&param.name, param.span);
        }

        if let Some(inherits) = &decl.inherits {
            self.check_inherits_clause(inherits);
        }

        let mut attrs = HashSet::new();
        let mut methods = HashMap::new();
        for member in &decl.members {
            match member {
                TypeMember::Attribute(attr) => {
                    if !attrs.insert(attr.name.clone()) {
                        self.report(attr.span, format!(
                            "atributo '{}' ya definido en el tipo",
                            attr.name
                        ));
                    }
                    if methods.contains_key(&attr.name) {
                        self.report(attr.span, format!(
                            "atributo '{}' entra en conflicto con metodo",
                            attr.name
                        ));
                    }
                }
                TypeMember::Method(method) => {
                    let entry = methods
                        .entry(method.name.clone())
                        .or_insert_with(HashSet::new);
                    if !entry.insert(method.params.len()) {
                        self.report(method.span, format!(
                            "metodo '{}' con aridad {} ya definido en el tipo",
                            method.name,
                            method.params.len()
                        ));
                    }
                    if attrs.contains(&method.name) {
                        self.report(method.span, format!(
                            "metodo '{}' entra en conflicto con atributo",
                            method.name
                        ));
                    }
                }
            }
        }

        let current_type = CurrentTypeInfo {
            parent: decl.inherits.as_ref().map(|i| i.parent.clone()),
            attrs,
            methods,
        };
        let prev_type = self.ctx.current_type.take();
        let prev_in_method = self.ctx.in_method;
        self.ctx.current_type = Some(current_type);
        self.ctx.in_method = false;

        for member in &decl.members {
            match member {
                TypeMember::Attribute(attr) => self.check_attr_def(attr),
                TypeMember::Method(method) => self.check_method_def(method),
            }
        }

        self.ctx.current_type = prev_type;
        self.ctx.in_method = prev_in_method;
        self.ctx.pop_scope();
    }

    fn check_protocol_decl(&mut self, decl: &ProtocolDecl) {
        if is_placeholder(&decl.name) {
            return;
        }

        if let Some(parent) = &decl.extends {
            if !self.ctx.is_protocol_defined(parent) {
                self.report(decl.span, format!(
                    "protocolo padre '{}' no definido",
                    parent
                ));
            }
        }

        let mut method_set = HashMap::new();
        for method in &decl.methods {
            let entry = method_set
                .entry(method.name.clone())
                .or_insert_with(HashSet::new);
            if !entry.insert(method.params.len()) {
                self.report(method.span, format!(
                    "metodo '{}' con aridad {} ya definido en el protocolo",
                    method.name,
                    method.params.len()
                ));
            }
            self.check_method_sig(method);
        }
    }

    fn check_macro_decl(&mut self, decl: &MacroDecl) {
        if is_placeholder(&decl.name) {
            return;
        }

        let mut seen = HashSet::new();
        for param in &decl.params {
            self.check_macro_param(param, &mut seen);
        }

        self.ctx.push_scope();
        for param in &decl.params {
            if let Some(name) = macro_param_name(param) {
                self.define_var(&name, macro_param_span(param));
            }
        }
        self.check_func_body(&decl.body);
        self.ctx.pop_scope();
    }

    fn check_method_sig(&mut self, sig: &MethodSig) {
        let mut seen = HashSet::new();
        for param in &sig.params {
            if is_placeholder(&param.name) {
                continue;
            }
            if !seen.insert(param.name.clone()) {
                self.report(param.span, format!(
                    "parametro '{}' duplicado",
                    param.name
                ));
            }
            if let Some(ty) = &param.ty {
                self.check_type_expr(ty, param.span);
            }
        }
        self.check_type_expr(&sig.return_type, sig.span);
    }

    fn check_method_def(&mut self, method: &MethodDef) {
        let mut seen = HashSet::new();
        for param in &method.params {
            self.check_param(param, &mut seen);
        }
        if let Some(ty) = &method.return_type {
            self.check_type_expr(ty, method.span);
        }

        let prev_in_method = self.ctx.in_method;
        self.ctx.in_method = true;
        self.ctx.push_scope();
        for param in &method.params {
            self.define_var(&param.name, param.span);
        }
        self.check_func_body(&method.body);
        self.ctx.pop_scope();
        self.ctx.in_method = prev_in_method;
    }

    fn check_attr_def(&mut self, attr: &AttrDef) {
        if let Some(ty) = &attr.ty {
            self.check_type_expr(ty, attr.span);
        }
        self.check_expr(&attr.init);
    }

    fn check_inherits_clause(&mut self, inherits: &InheritsClause) {
        if is_placeholder(&inherits.parent) {
            return;
        }

        if !self.ctx.is_constructible_type(&inherits.parent) {
            self.report(inherits.span, format!(
                "tipo padre '{}' no definido",
                inherits.parent
            ));
        } else if let Some(expected) = self.ctx.type_param_count(&inherits.parent) {
            if expected != inherits.args.len() {
                self.report(inherits.span, format!(
                    "tipo padre '{}' requiere {} argumentos",
                    inherits.parent,
                    expected
                ));
            }
        }

        for arg in &inherits.args {
            self.check_expr(arg);
        }
    }

    fn check_param(&mut self, param: &Param, seen: &mut HashSet<String>) {
        if is_placeholder(&param.name) {
            return;
        }
        if !seen.insert(param.name.clone()) {
            self.report(param.span, format!(
                "parametro '{}' duplicado",
                param.name
            ));
        }
        if let Some(ty) = &param.ty {
            self.check_type_expr(ty, param.span);
        }
    }

    fn check_macro_param(&mut self, param: &MacroParam, seen: &mut HashSet<String>) {
        match param {
            MacroParam::Regular(p) => self.check_param(p, seen),
            MacroParam::Block { name, ty, span }
            | MacroParam::Symbolic { name, ty, span }
            | MacroParam::Placeholder { name, ty, span } => {
                if is_placeholder(name) {
                    return;
                }
                if !seen.insert(name.clone()) {
                    self.report(*span, format!(
                        "parametro '{}' duplicado",
                        name
                    ));
                }
                self.check_type_expr(ty, *span);
            }
        }
    }

    fn check_func_body(&mut self, body: &FuncBody) {
        match body {
            FuncBody::Inline(expr) => self.check_expr(expr),
            FuncBody::Block(expr) => self.check_expr(expr),
        }
    }

    fn check_expr(&mut self, expr: &Expr) {
        match expr {
            Expr::Number { .. } => {}
            Expr::StringLit { .. } => {}
            Expr::Bool { .. } => {}
            Expr::Ident { name, span } => {
                if is_placeholder(name) {
                    return;
                }
                if self.ctx.is_var_defined(name)
                    || self.ctx.is_builtin_const(name)
                    || self.ctx.has_function_name(name)
                    || self.ctx.has_macro_name(name)
                    || self.ctx.is_builtin_function(name)
                {
                    return;
                }
                self.report(*span, format!(
                    "identificador '{}' no definido",
                    name
                ));
            }
            Expr::Call { callee, args, .. } => {
                for arg in args {
                    self.check_expr(arg);
                }

                match &**callee {
                    Expr::Ident { name, span } => {
                        self.check_call_ident(name, args.len(), *span)
                    }
                    _ => self.check_expr(callee),
                }
            }
            Expr::New { type_name, args, span } => {
                if !is_placeholder(type_name) {
                    if !self.ctx.is_constructible_type(type_name) {
                        self.report(*span, format!(
                            "tipo '{}' no definido",
                            type_name
                        ));
                    } else if let Some(expected) = self.ctx.type_param_count(type_name) {
                        if expected != args.len() {
                            self.report(*span, format!(
                                "tipo '{}' requiere {} argumentos",
                                type_name,
                                expected
                            ));
                        }
                    }
                }
                for arg in args {
                    self.check_expr(arg);
                }
            }
            Expr::FieldAccess { object, field, span } => {
                self.check_expr(object);
                if self.ctx.in_method && is_self_ref(object) {
                    if let Some(current) = &self.ctx.current_type {
                        if !current.attrs.contains(field) {
                            self.report(*span, format!(
                                "atributo '{}' no definido en el tipo actual",
                                field
                            ));
                        }
                    }
                }
            }
            Expr::MethodCall { object, method, args, span } => {
                self.check_expr(object);
                for arg in args {
                    self.check_expr(arg);
                }
                if self.ctx.in_method && is_self_ref(object) {
                    if let Some(current) = &self.ctx.current_type {
                        let ok = current
                            .methods
                            .get(method)
                            .map_or(false, |set| set.contains(&args.len()));
                        if !ok {
                            self.report(*span, format!(
                                "metodo '{}' con aridad {} no definido en el tipo actual",
                                method,
                                args.len()
                            ));
                        }
                    }
                }
            }
            Expr::SelfRef { span } => {
                if !self.ctx.in_method {
                    self.report(*span, "uso de self fuera de un metodo".to_string());
                }
            }
            Expr::Base { args, span } => {
                if !self.ctx.in_method {
                    self.report(*span, "uso de base fuera de un metodo".to_string());
                } else if let Some(current) = &self.ctx.current_type {
                    if let Some(parent) = &current.parent {
                        if let Some(expected) = self.ctx.type_param_count(parent) {
                            if expected != args.len() {
                                self.report(*span, format!(
                                    "base requiere {} argumentos",
                                    expected
                                ));
                            }
                        }
                    } else {
                        self.report(*span, "base requiere herencia".to_string());
                    }
                }
                for arg in args {
                    self.check_expr(arg);
                }
            }
            Expr::BinaryOp { left, right, .. } => {
                self.check_expr(left);
                self.check_expr(right);
            }
            Expr::UnaryOp { operand, .. } => {
                self.check_expr(operand);
            }
            Expr::IsType { expr, ty, span } => {
                self.check_expr(expr);
                self.check_type_expr(ty, *span);
            }
            Expr::AsType { expr, ty, span } => {
                self.check_expr(expr);
                self.check_type_expr(ty, *span);
            }
            Expr::If { condition, then_expr, elif_branches, else_expr, .. } => {
                self.check_expr(condition);
                self.with_scope(|this| this.check_expr(then_expr));
                for branch in elif_branches {
                    self.check_expr(&branch.condition);
                    self.with_scope(|this| this.check_expr(&branch.body));
                }
                self.with_scope(|this| this.check_expr(else_expr));
            }
            Expr::While { condition, body, .. } => {
                self.check_expr(condition);
                self.with_scope(|this| this.check_expr(body));
            }
            Expr::For { var, iterable, body, .. } => {
                self.check_expr(iterable);
                self.with_scope(|this| {
                    this.define_var(var, expr_span(expr));
                    this.check_expr(body);
                });
            }
            Expr::Let { bindings, body, span } => {
                if bindings.is_empty() {
                    self.report(*span, "let sin bindings".to_string());
                    return;
                }
                let mut scopes_pushed = 0usize;
                for binding in bindings {
                    self.check_expr(&binding.init);
                    if let Some(ty) = &binding.ty {
                        self.check_type_expr(ty, binding.span);
                    }
                    self.ctx.push_scope();
                    scopes_pushed += 1;
                    self.define_var(&binding.name, binding.span);
                }
                self.check_expr(body);
                for _ in 0..scopes_pushed {
                    self.ctx.pop_scope();
                }
            }
            Expr::Assign { target, value, span } => {
                self.check_assign_target(target, *span);
                self.check_expr(value);
            }
            Expr::Block { exprs, span } => {
                if exprs.is_empty() {
                    self.report(*span, "bloque vacio".to_string());
                    return;
                }
                self.ctx.push_scope();
                for e in exprs {
                    self.check_expr(e);
                }
                self.ctx.pop_scope();
            }
            Expr::VectorLit { elements, .. } => {
                for e in elements {
                    self.check_expr(e);
                }
                self.check_vector_literal_types(elements);
            }
            Expr::VectorGen { element, var, iterable, .. } => {
                self.check_expr(iterable);
                self.with_scope(|this| {
                    this.define_var(var, expr_span(expr));
                    this.check_expr(element);
                });
            }
            Expr::Index { object, index, .. } => {
                self.check_expr(object);
                self.check_expr(index);
            }
            Expr::Lambda { params, return_type, body, .. } => {
                let mut seen = HashSet::new();
                for param in params {
                    self.check_param(param, &mut seen);
                }
                if let Some(ty) = return_type {
                    self.check_type_expr(ty, expr_span(expr));
                }
                self.ctx.push_scope();
                for param in params {
                    self.define_var(&param.name, param.span);
                }
                self.check_func_body(body);
                self.ctx.pop_scope();
            }
            Expr::Error { .. } => {}
        }
    }

    fn check_assign_target(&mut self, target: &Expr, span: Span) {
        match target {
            Expr::Ident { name, span } => {
                if is_placeholder(name) {
                    return;
                }
                if !self.ctx.is_var_defined(name) {
                    self.report(*span, format!(
                        "asignacion a variable no definida '{}'",
                        name
                    ));
                }
            }
            Expr::FieldAccess { object, .. } => {
                self.check_expr(target);
            }
            Expr::Index { object, index, .. } => {
                self.check_expr(object);
                self.check_expr(index);
            }
            Expr::Error { .. } => {}
            _ => {
                self.report(span, "objetivo de asignacion invalido".to_string());
            }
        }
    }

    fn check_call_ident(&mut self, name: &str, arity: usize, span: Span) {
        if is_placeholder(name) {
            return;
        }
        if self.ctx.is_var_defined(name) {
            return;
        }
        if self.ctx.has_function(name, arity)
            || self.ctx.has_macro(name, arity)
            || self.ctx.has_builtin_function(name, arity)
        {
            return;
        }
        if self.ctx.has_function_name(name)
            || self.ctx.has_macro_name(name)
            || self.ctx.is_builtin_function(name)
        {
            self.report(span, format!(
                "llamada a '{}' con aridad invalida ({})",
                name,
                arity
            ));
        } else {
            self.report(span, format!(
                "funcion '{}' no definida",
                name
            ));
        }
    }

    fn check_type_expr(&mut self, ty: &TypeExpr, span: Span) {
        match ty {
            TypeExpr::Named(name) => {
                if is_placeholder(name) {
                    return;
                }
                if !self.ctx.is_known_type(name) {
                    self.report(span, format!(
                        "tipo '{}' no definido",
                        name
                    ));
                }
            }
            TypeExpr::Iterable(inner) => self.check_type_expr(inner, span),
            TypeExpr::Vector(inner) => self.check_type_expr(inner, span),
            TypeExpr::Functor { params, returns } => {
                for p in params {
                    self.check_type_expr(p, span);
                }
                self.check_type_expr(returns, span);
            }
        }
    }

    fn define_var(&mut self, name: &str, span: Span) {
        if is_placeholder(name) {
            return;
        }
        if !self.ctx.define_var(name) {
            self.report(span, format!(
                "variable '{}' ya definida en este ambito",
                name
            ));
        }
    }

    fn report(&mut self, span: Span, message: String) {
        self.errors.push(SemanticError { span, message });
    }

    // Conservative type inference for vector literal element checks.
    fn infer_simple_type(&self, expr: &Expr) -> Option<SimpleType> {
        match expr {
            Expr::Number { .. } => Some(SimpleType::Number),
            Expr::StringLit { .. } => Some(SimpleType::String),
            Expr::Bool { .. } => Some(SimpleType::Boolean),
            Expr::UnaryOp { op, operand, .. } => {
                let inner = self.infer_simple_type(operand)?;
                match (op, inner) {
                    (UnaryOp::Neg, SimpleType::Number) => Some(SimpleType::Number),
                    (UnaryOp::Not, SimpleType::Boolean) => Some(SimpleType::Boolean),
                    _ => None,
                }
            }
            Expr::BinaryOp { op, left, right, .. } => {
                let left_ty = self.infer_simple_type(left)?;
                let right_ty = self.infer_simple_type(right)?;
                match op {
                    BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div | BinOp::Mod | BinOp::Pow => {
                        if left_ty == SimpleType::Number && right_ty == SimpleType::Number {
                            Some(SimpleType::Number)
                        } else {
                            None
                        }
                    }
                    BinOp::And | BinOp::Or => {
                        if left_ty == SimpleType::Boolean && right_ty == SimpleType::Boolean {
                            Some(SimpleType::Boolean)
                        } else {
                            None
                        }
                    }
                    BinOp::Eq | BinOp::NotEq => {
                        if left_ty == right_ty {
                            Some(SimpleType::Boolean)
                        } else {
                            None
                        }
                    }
                    BinOp::Lt | BinOp::Gt | BinOp::LtEq | BinOp::GtEq => {
                        if left_ty == SimpleType::Number && right_ty == SimpleType::Number {
                            Some(SimpleType::Boolean)
                        } else {
                            None
                        }
                    }
                    BinOp::Concat | BinOp::ConcatSpace => {
                        if left_ty == SimpleType::String || right_ty == SimpleType::String {
                            Some(SimpleType::String)
                        } else {
                            None
                        }
                    }
                }
            }
            Expr::VectorLit { elements, .. } => {
                let mut expected: Option<SimpleType> = None;
                for element in elements {
                    let Some(element_ty) = self.infer_simple_type(element) else {
                        continue;
                    };
                    match &expected {
                        Some(expected_ty) if expected_ty != &element_ty => return None,
                        None => expected = Some(element_ty),
                        _ => {}
                    }
                }
                expected.map(|ty| SimpleType::Vector(Box::new(ty)))
            }
            _ => None,
        }
    }

    fn check_vector_literal_types(&mut self, elements: &[Expr]) {
        let mut expected: Option<SimpleType> = None;
        for element in elements {
            let Some(element_ty) = self.infer_simple_type(element) else {
                continue;
            };
            match &expected {
                Some(expected_ty) if expected_ty != &element_ty => {
                    self.report(
                        expr_span(element),
                        format!(
                            "vector con elementos de distinto tipo (se esperaba {}, se encontro {})",
                            expected_ty.display_name(),
                            element_ty.display_name()
                        ),
                    );
                    break;
                }
                None => expected = Some(element_ty),
                _ => {}
            }
        }
    }

    fn with_scope<F>(&mut self, f: F)
    where
        F: FnOnce(&mut SemanticChecker),
    {
        self.ctx.push_scope();
        f(self);
        self.ctx.pop_scope();
    }
}

#[derive(Clone)]
struct Context {
    var_scopes: Vec<HashSet<String>>,
    functions: HashMap<String, HashSet<usize>>,
    macros: HashMap<String, HashSet<usize>>,
    types: HashMap<String, TypeInfo>,
    protocols: HashSet<String>,
    builtin_functions: HashMap<String, HashSet<usize>>,
    builtin_types: HashSet<String>,
    builtin_consts: HashSet<String>,
    current_type: Option<CurrentTypeInfo>,
    in_method: bool,
}

#[derive(Clone)]
struct TypeInfo {
    param_count: usize,
}

#[derive(Clone)]
struct CurrentTypeInfo {
    parent: Option<String>,
    attrs: HashSet<String>,
    methods: HashMap<String, HashSet<usize>>,
}

impl Context {
    fn new() -> Self {
        Self {
            var_scopes: vec![HashSet::new()],
            functions: HashMap::new(),
            macros: HashMap::new(),
            types: HashMap::new(),
            protocols: HashSet::new(),
            builtin_functions: builtin_functions(),
            builtin_types: builtin_types(),
            builtin_consts: builtin_consts(),
            current_type: None,
            in_method: false,
        }
    }

    fn push_scope(&mut self) {
        self.var_scopes.push(HashSet::new());
    }

    fn pop_scope(&mut self) {
        self.var_scopes.pop();
    }

    fn define_var(&mut self, name: &str) -> bool {
        if let Some(scope) = self.var_scopes.last_mut() {
            return scope.insert(name.to_string());
        }
        false
    }

    fn is_var_defined(&self, name: &str) -> bool {
        self.var_scopes.iter().rev().any(|s| s.contains(name))
    }

    fn insert_function(&mut self, name: &str, arity: usize) -> bool {
        insert_arity(&mut self.functions, name, arity)
    }

    fn insert_macro(&mut self, name: &str, arity: usize) -> bool {
        insert_arity(&mut self.macros, name, arity)
    }

    fn insert_type(&mut self, name: &str, param_count: usize) {
        self.types
            .entry(name.to_string())
            .or_insert(TypeInfo { param_count });
    }

    fn insert_protocol(&mut self, name: &str) {
        self.protocols.insert(name.to_string());
    }

    fn has_function(&self, name: &str, arity: usize) -> bool {
        has_arity(&self.functions, name, arity)
    }

    fn has_macro(&self, name: &str, arity: usize) -> bool {
        has_arity(&self.macros, name, arity)
    }

    fn has_builtin_function(&self, name: &str, arity: usize) -> bool {
        has_arity(&self.builtin_functions, name, arity)
    }

    fn has_function_name(&self, name: &str) -> bool {
        self.functions.contains_key(name)
    }

    fn has_macro_name(&self, name: &str) -> bool {
        self.macros.contains_key(name)
    }

    fn is_builtin_function(&self, name: &str) -> bool {
        self.builtin_functions.contains_key(name)
    }

    fn is_builtin_type(&self, name: &str) -> bool {
        self.builtin_types.contains(name)
    }

    fn is_builtin_const(&self, name: &str) -> bool {
        self.builtin_consts.contains(name)
    }

    fn is_protocol_defined(&self, name: &str) -> bool {
        self.protocols.contains(name)
    }

    fn is_type_or_protocol_defined(&self, name: &str) -> bool {
        self.types.contains_key(name) || self.protocols.contains(name)
    }

    fn is_known_type(&self, name: &str) -> bool {
        self.builtin_types.contains(name)
            || self.types.contains_key(name)
            || self.protocols.contains(name)
    }

    fn is_constructible_type(&self, name: &str) -> bool {
        self.builtin_types.contains(name) || self.types.contains_key(name)
    }

    fn type_param_count(&self, name: &str) -> Option<usize> {
        if self.builtin_types.contains(name) {
            return Some(0);
        }
        self.types.get(name).map(|t| t.param_count)
    }
}

fn builtin_functions() -> HashMap<String, HashSet<usize>> {
    let mut map = HashMap::new();
    map.insert("sin".to_string(), arity_set(&[1]));
    map.insert("cos".to_string(), arity_set(&[1]));
    map.insert("sqrt".to_string(), arity_set(&[1]));
    map.insert("exp".to_string(), arity_set(&[1]));
    map.insert("log".to_string(), arity_set(&[1, 2]));
    map.insert("rand".to_string(), arity_set(&[0]));
    map.insert("print".to_string(), arity_set(&[1]));
    map.insert("range".to_string(), arity_set(&[2]));
    map
}

fn builtin_types() -> HashSet<String> {
    ["Number", "String", "Boolean", "Object"]
        .iter()
        .map(|s| s.to_string())
        .collect()
}

fn builtin_consts() -> HashSet<String> {
    ["PI", "E", "()"].iter().map(|s| s.to_string()).collect()
}

fn arity_set(values: &[usize]) -> HashSet<usize> {
    values.iter().copied().collect()
}

fn insert_arity(map: &mut HashMap<String, HashSet<usize>>, name: &str, arity: usize) -> bool {
    let entry = map.entry(name.to_string()).or_insert_with(HashSet::new);
    entry.insert(arity)
}

fn has_arity(map: &HashMap<String, HashSet<usize>>, name: &str, arity: usize) -> bool {
    map.get(name).map_or(false, |set| set.contains(&arity))
}

fn is_placeholder(name: &str) -> bool {
    name.is_empty() || name == "__parse_error__"
}

fn is_self_ref(expr: &Expr) -> bool {
    matches!(expr, Expr::SelfRef { .. })
}

fn expr_span(expr: &Expr) -> Span {
    match expr {
        Expr::Number { span, .. } => *span,
        Expr::StringLit { span, .. } => *span,
        Expr::Bool { span, .. } => *span,
        Expr::Ident { span, .. } => *span,
        Expr::Call { span, .. } => *span,
        Expr::New { span, .. } => *span,
        Expr::FieldAccess { span, .. } => *span,
        Expr::MethodCall { span, .. } => *span,
        Expr::SelfRef { span } => *span,
        Expr::Base { span, .. } => *span,
        Expr::BinaryOp { span, .. } => *span,
        Expr::UnaryOp { span, .. } => *span,
        Expr::IsType { span, .. } => *span,
        Expr::AsType { span, .. } => *span,
        Expr::If { span, .. } => *span,
        Expr::While { span, .. } => *span,
        Expr::For { span, .. } => *span,
        Expr::Let { span, .. } => *span,
        Expr::Assign { span, .. } => *span,
        Expr::Block { span, .. } => *span,
        Expr::VectorLit { span, .. } => *span,
        Expr::VectorGen { span, .. } => *span,
        Expr::Index { span, .. } => *span,
        Expr::Lambda { span, .. } => *span,
        Expr::Error { span } => *span,
    }
}

fn macro_param_name(param: &MacroParam) -> Option<String> {
    match param {
        MacroParam::Regular(p) => Some(p.name.clone()),
        MacroParam::Block { name, .. }
        | MacroParam::Symbolic { name, .. }
        | MacroParam::Placeholder { name, .. } => Some(name.clone()),
    }
}

fn macro_param_span(param: &MacroParam) -> Span {
    match param {
        MacroParam::Regular(p) => p.span,
        MacroParam::Block { span, .. }
        | MacroParam::Symbolic { span, .. }
        | MacroParam::Placeholder { span, .. } => *span,
    }
}

