use std::collections::{HashMap, HashSet};

use crate::lexer::lexer::Span;
use crate::parser::ast::*;

use super::context::{CallableSignature, Context, CurrentTypeInfo};

/// A semantic error produced during checking, with a source span and message.
#[derive(Debug, Clone)]
pub struct SemanticError {
    /// Source span where the error occurred.
    pub span: Span,
    /// Human readable error message.
    pub message: String,
}

/// A simplified set of types used for conservative inference during checks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SimpleType {
    Number,
    String,
    Boolean,
    Named(String),
    Vector(Box<SimpleType>),
}

impl SimpleType {
    /// Return a human friendly name for the simple type.
    fn display_name(&self) -> String {
        match self {
            SimpleType::Number => "Number".to_string(),
            SimpleType::String => "String".to_string(),
            SimpleType::Boolean => "Boolean".to_string(),
            SimpleType::Named(name) => name.clone(),
            SimpleType::Vector(inner) => format!("Vector<{}>", inner.display_name()),
        }
    }
}

impl std::fmt::Display for SemanticError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[SemanticError {}] {}", self.span, self.message)
    }
}

/// Run semantic checks over a program and return a list of errors.
pub fn check_program(program: &Program) -> Vec<SemanticError> {
    let mut checker = SemanticChecker::new();
    checker.check_program(program);
    checker.errors
}

/// Top-level semantic checker that holds context and collected errors.
pub struct SemanticChecker {
    ctx: Context,
    errors: Vec<SemanticError>,
}

impl SemanticChecker {
    /// Create a new semantic checker with a fresh context.
    pub fn new() -> Self {
        Self {
            ctx: Context::new(),
            errors: Vec::new(),
        }
    }

    /// Perform full semantic checking of a program (predeclare then check).
    pub fn check_program(&mut self, program: &Program) {
        for decl in &program.decls {
            self.predeclare_decl(decl);
        }
        for decl in &program.decls {
            self.check_decl(decl);
        }
        self.check_expr(&program.expr);
    }

    /// Pre-declare a top-level declaration to allow forward references.
    fn predeclare_decl(&mut self, decl: &Decl) {
        match decl {
            Decl::Function(d) => self.predeclare_func_decl(d),
            Decl::Type(d) => self.predeclare_type_decl(d),
            Decl::Protocol(d) => self.predeclare_protocol_decl(d),
            Decl::Macro(d) => self.predeclare_macro_decl(d),
        }
    }

    /// Check a top-level declaration in detail.
    fn check_decl(&mut self, decl: &Decl) {
        match decl {
            Decl::Function(d) => self.check_func_decl(d),
            Decl::Type(d) => self.check_type_decl(d),
            Decl::Protocol(d) => self.check_protocol_decl(d),
            Decl::Macro(d) => self.check_macro_decl(d),
        }
    }

    /// Predeclare a function name and arity; report redefinitions/conflicts.
    fn predeclare_func_decl(&mut self, decl: &FuncDecl) {
        if is_placeholder(&decl.name) {
            return;
        }

        if self.ctx.is_builtin_function(&decl.name) {
            self.report(decl.span, format!(
                "builtin function '{}' cannot be redefined",
                decl.name
            ));
            return;
        }

        if self.ctx.has_function_name(&decl.name) {
            self.report(decl.span, format!(
                "function '{}' already defined",
                decl.name
            ));
        }

        self.ctx.insert_function(&decl.name, decl.params.len());
        self.ctx.insert_function_signature(
            &decl.name,
            callable_params_from_params(&decl.params),
            decl.return_type.as_ref().and_then(simple_type_from_type_expr),
        );
    }

    /// Predeclare a user-defined type and its type parameter count.
    fn predeclare_type_decl(&mut self, decl: &TypeDecl) {
        if is_placeholder(&decl.name) {
            return;
        }

        if self.ctx.is_builtin_type(&decl.name) {
            self.report(decl.span, format!(
                "builtin type '{}' cannot be redefined",
                decl.name
            ));
            return;
        }

        if self.ctx.is_type_or_protocol_defined(&decl.name) {
            self.report(decl.span, format!(
                "type or protocol '{}' already defined",
                decl.name
            ));
            return;
        }

        self.ctx.insert_type(&decl.name, decl.type_params.len());
    }

    /// Predeclare a protocol, reporting conflicts with builtin types.
    fn predeclare_protocol_decl(&mut self, decl: &ProtocolDecl) {
        if is_placeholder(&decl.name) {
            return;
        }

        if self.ctx.is_builtin_type(&decl.name) {
            self.report(decl.span, format!(
                "protocol '{}' conflicts with builtin type",
                decl.name
            ));
            return;
        }

        if self.ctx.is_type_or_protocol_defined(&decl.name) {
            self.report(decl.span, format!(
                "type or protocol '{}' already defined",
                decl.name
            ));
            return;
        }

        self.ctx.insert_protocol(&decl.name);
    }

    /// Predeclare a macro name and arity; report conflicts.
    fn predeclare_macro_decl(&mut self, decl: &MacroDecl) {
        if is_placeholder(&decl.name) {
            return;
        }

        if self.ctx.is_builtin_function(&decl.name) {
            self.report(decl.span, format!(
                "macro '{}' conflicts with builtin function",
                decl.name
            ));
            return;
        }

        if self.ctx.has_macro_name(&decl.name) {
            self.report(decl.span, format!(
                "macro '{}' already defined",
                decl.name
            ));
        }

        self.ctx.insert_macro(&decl.name, decl.params.len());
    }

    /// Check a function declaration: parameters, return type, and body.
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

        let mut inferred_param_types = callable_params_from_params(&decl.params);
        for (index, param) in decl.params.iter().enumerate() {
            if inferred_param_types[index].is_none() {
                inferred_param_types[index] = self.ctx.var_type(&param.name);
            }
        }

        let inferred_return_type = self.infer_callable_body_type(&decl.body);
        self.check_return_type(
            "function",
            &decl.name,
            decl.return_type.as_ref(),
            inferred_return_type.clone(),
            decl.span,
        );
        if decl.return_type.as_ref().and_then(simple_type_from_type_expr).is_none() {
            if let Some(inferred_return_type) = inferred_return_type {
                self.ctx.insert_function_signature(&decl.name, inferred_param_types, Some(inferred_return_type));
            } else {
                self.ctx.insert_function_signature(
                    &decl.name,
                    inferred_param_types,
                    self.ctx
                        .function_signature(&decl.name)
                        .and_then(|signature| signature.return_type.clone()),
                );
            }
        } else if inferred_param_types != callable_params_from_params(&decl.params) {
            self.ctx.insert_function_signature(
                &decl.name,
                inferred_param_types,
                decl.return_type.as_ref().and_then(simple_type_from_type_expr),
            );
        }
        self.ctx.pop_scope();
    }

    /// Check a type declaration: type parameters, inheritance and members.
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
        let mut methods: HashMap<String, HashMap<usize, CallableSignature>> = HashMap::new();
        for member in &decl.members {
            match member {
                TypeMember::Attribute(attr) => {
                    if !attrs.insert(attr.name.clone()) {
                        self.report(attr.span, format!(
                            "attribute '{}' already defined in the type",
                            attr.name
                        ));
                    }
                    if methods.contains_key(&attr.name) {
                        self.report(attr.span, format!(
                            "attribute '{}' conflicts with method",
                            attr.name
                        ));
                    }
                }
                TypeMember::Method(method) => {
                    let entry = methods
                        .entry(method.name.clone())
                        .or_insert_with(HashMap::new);
                    if entry.contains_key(&method.params.len()) {
                        self.report(method.span, format!(
                            "method '{}' with arity {} already defined in the type",
                            method.name,
                            method.params.len()
                        ));
                    } else {
                        entry.insert(
                            method.params.len(),
                            CallableSignature {
                                params: callable_params_from_params(&method.params),
                                return_type: method
                                    .return_type
                                    .as_ref()
                                    .and_then(simple_type_from_type_expr),
                            },
                        );
                    }
                    if attrs.contains(&method.name) {
                        self.report(method.span, format!(
                            "method '{}' conflicts with attribute",
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
        // Record the discovered attributes and methods in the global context
        self.ctx.set_type_members(&decl.name, current_type.attrs.clone(), current_type.methods.clone());
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

    /// Check a protocol declaration: verifies parent and method signatures.
    fn check_protocol_decl(&mut self, decl: &ProtocolDecl) {
        if is_placeholder(&decl.name) {
            return;
        }

        if let Some(parent) = &decl.extends {
            if !self.ctx.is_protocol_defined(parent) {
                self.report(decl.span, format!(
                    "parent protocol '{}' not defined",
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
                    "method '{}' with arity {} already defined in the protocol",
                    method.name,
                    method.params.len()
                ));
            }
            self.check_method_sig(method);
        }
    }

    /// Check a macro declaration and its parameters/body.
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

    /// Check a method signature for duplicate parameters and valid types.
    fn check_method_sig(&mut self, sig: &MethodSig) {
        let mut seen = HashSet::new();
        for param in &sig.params {
            if is_placeholder(&param.name) {
                continue;
            }
            if !seen.insert(param.name.clone()) {
                self.report(param.span, format!(
                    "duplicate parameter '{}'",
                    param.name
                ));
            }
            if let Some(ty) = &param.ty {
                self.check_type_expr(ty, param.span);
            }
        }
        self.check_type_expr(&sig.return_type, sig.span);
    }

    /// Check a method definition including its body and parameter types.
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

        let mut inferred_param_types = callable_params_from_params(&method.params);
        for (index, param) in method.params.iter().enumerate() {
            if inferred_param_types[index].is_none() {
                inferred_param_types[index] = self.ctx.var_type(&param.name);
            }
        }

        let inferred_return_type = self.infer_callable_body_type(&method.body);
        self.check_return_type(
            "method",
            &method.name,
            method.return_type.as_ref(),
            inferred_return_type.clone(),
            method.span,
        );
        if method.return_type.as_ref().and_then(simple_type_from_type_expr).is_none() {
            if let Some(inferred_return_type) = inferred_return_type {
                if let Some(current_type) = self.ctx.current_type.as_mut() {
                    if let Some(methods) = current_type.methods.get_mut(&method.name) {
                        if let Some(signature) = methods.get_mut(&method.params.len()) {
                            signature.return_type = Some(inferred_return_type);
                            signature.params = inferred_param_types;
                        }
                    }
                }
            } else if let Some(current_type) = self.ctx.current_type.as_mut() {
                if let Some(methods) = current_type.methods.get_mut(&method.name) {
                    if let Some(signature) = methods.get_mut(&method.params.len()) {
                        signature.params = inferred_param_types;
                    }
                }
            }
        } else if let Some(current_type) = self.ctx.current_type.as_mut() {
            if let Some(methods) = current_type.methods.get_mut(&method.name) {
                if let Some(signature) = methods.get_mut(&method.params.len()) {
                    signature.params = inferred_param_types;
                }
            }
        }
        self.ctx.pop_scope();
        self.ctx.in_method = prev_in_method;
    }

    /// Check an attribute definition: its declared type and initializer.
    fn check_attr_def(&mut self, attr: &AttrDef) {
        if let Some(ty) = &attr.ty {
            self.check_type_expr(ty, attr.span);
        }
        self.check_expr(&attr.init);
    }

    /// Validate an inheritance clause: parent exists and args match.
    fn check_inherits_clause(&mut self, inherits: &InheritsClause) {
        if is_placeholder(&inherits.parent) {
            return;
        }

        if !self.ctx.is_constructible_type(&inherits.parent) {
            self.report(inherits.span, format!(
                "parent type '{}' not defined",
                inherits.parent
            ));
        } else if let Some(expected) = self.ctx.type_param_count(&inherits.parent) {
            if expected != inherits.args.len() {
                self.report(inherits.span, format!(
                    "parent type '{}' requires {} arguments",
                    inherits.parent,
                    expected
                ));
            }
        }

        for arg in &inherits.args {
            self.check_expr(arg);
        }
    }

    /// Validate a parameter: uniqueness and type correctness.
    fn check_param(&mut self, param: &Param, seen: &mut HashSet<String>) {
        if is_placeholder(&param.name) {
            return;
        }
        if !seen.insert(param.name.clone()) {
            self.report(param.span, format!(
                "duplicate parameter '{}'",
                param.name
            ));
        }
        if let Some(ty) = &param.ty {
            self.check_type_expr(ty, param.span);
        }
    }

    /// Validate macro parameter variants and uniqueness.
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
                        "duplicate parameter '{}'",
                        name
                    ));
                }
                self.check_type_expr(ty, *span);
            }
        }
    }

    /// Check the body of a function (inline or block).
    fn check_func_body(&mut self, body: &FuncBody) {
        match body {
            FuncBody::Inline(expr) => self.check_expr(expr),
            FuncBody::Block(expr) => self.check_expr(expr),
        }
    }

    /// Recursively perform semantic checks on an expression tree.
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
                    "identifier '{}' not defined",
                    name
                ));
            }
            Expr::Call { callee, args, .. } => {
                for arg in args {
                    self.check_expr(arg);
                }

                match &**callee {
                    Expr::Ident { name, span } => {
                        self.check_call_ident(name, args, *span)
                    }
                    _ => self.check_expr(callee),
                }
            }
            Expr::New { type_name, args, span } => {
                if !is_placeholder(type_name) {
                    if !self.ctx.is_constructible_type(type_name) {
                        self.report(*span, format!(
                            "type '{}' not defined",
                            type_name
                        ));
                    } else if let Some(expected) = self.ctx.type_param_count(type_name) {
                        if expected != args.len() {
                            self.report(*span, format!(
                                "type '{}' requires {} arguments",
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
                                "attribute '{}' not defined on current type",
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
                    if let Some(signature) = self
                        .ctx
                        .current_type
                        .as_ref()
                        .and_then(|current| current.methods.get(method))
                        .and_then(|set| set.get(&args.len()))
                        .cloned()
                    {
                        self.check_callable_args(method, args, &signature.params, "method", *span);
                    } else {
                        self.report(*span, format!(
                            "method '{}' with arity {} not defined on current type",
                            method,
                            args.len()
                        ));
                    }
                } else {
                    // Try to resolve method from the static type of the object
                    if let Some(obj_ty) = self.infer_simple_type(object) {
                        if let SimpleType::Named(type_name) = obj_ty {
                            if let Some(signature) = self
                                .ctx
                                .type_method_signature(&type_name, method, args.len())
                                .cloned()
                            {
                                self.check_callable_args(method, args, &signature.params, "method", *span);
                            } else {
                                self.report(*span, format!(
                                    "method '{}' with arity {} not defined on type '{}'",
                                    method,
                                    args.len(),
                                    type_name
                                ));
                            }
                        }
                    }
                }
            }
            Expr::SelfRef { span } => {
                if !self.ctx.in_method {
                    self.report(*span, "use of self outside of a method".to_string());
                }
            }
            Expr::Base { args, span } => {
                if !self.ctx.in_method {
                    self.report(*span, "use of base outside of a method".to_string());
                } else if let Some(current) = &self.ctx.current_type {
                    if let Some(parent) = &current.parent {
                        if let Some(expected) = self.ctx.type_param_count(parent) {
                            if expected != args.len() {
                                self.report(*span, format!(
                                    "base requires {} arguments",
                                    expected
                                ));
                            }
                        }
                    } else {
                        self.report(*span, "base requires inheritance".to_string());
                    }
                }
                for arg in args {
                    self.check_expr(arg);
                }
            }
            Expr::BinaryOp { op, left, right, span } => {
                self.check_expr(left);
                self.check_expr(right);

                let left_ty = self.infer_simple_type(left);
                let right_ty = self.infer_simple_type(right);

                match op {
                    BinOp::And | BinOp::Or => {
                        if let Some(lt) = &left_ty {
                            if *lt != SimpleType::Boolean {
                                self.report(*span, format!(
                                    "logical operator requires Boolean (left side: {})",
                                    lt.display_name()
                                ));
                            }
                        }
                        if let Some(rt) = &right_ty {
                            if *rt != SimpleType::Boolean {
                                self.report(*span, format!(
                                    "logical operator requires Boolean (right side: {})",
                                    rt.display_name()
                                ));
                            }
                        }
                    }
                    BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div | BinOp::Mod | BinOp::Pow => {
                        if let Some(lt) = &left_ty {
                            if *lt != SimpleType::Number {
                                self.report(*span, format!(
                                    "arithmetic operator requires Number (left side: {})",
                                    lt.display_name()
                                ));
                            }
                        }
                        if let Some(rt) = &right_ty {
                            if *rt != SimpleType::Number {
                                self.report(*span, format!(
                                    "arithmetic operator requires Number (right side: {})",
                                    rt.display_name()
                                ));
                            }
                        }
                        self.constrain_number_operand(left, right_ty.as_ref());
                        self.constrain_number_operand(right, left_ty.as_ref());
                    }
                    BinOp::Lt | BinOp::Gt | BinOp::LtEq | BinOp::GtEq => {
                        if let Some(lt) = &left_ty {
                            if *lt != SimpleType::Number {
                                self.report(*span, format!(
                                    "relational operator requires Number (left side: {})",
                                    lt.display_name()
                                ));
                            }
                        }
                        if let Some(rt) = &right_ty {
                            if *rt != SimpleType::Number {
                                self.report(*span, format!(
                                    "relational operator requires Number (right side: {})",
                                    rt.display_name()
                                ));
                            }
                        }
                        self.constrain_number_operand(left, right_ty.as_ref());
                        self.constrain_number_operand(right, left_ty.as_ref());
                    }
                    BinOp::Eq | BinOp::NotEq => {
                        if let (Some(lt), Some(rt)) = (&left_ty, &right_ty) {
                            if lt != rt {
                                self.report(*span, format!(
                                    "equality operator requires operands of the same type ({} vs {})",
                                    lt.display_name(),
                                    rt.display_name()
                                ));
                            }
                        }
                        self.constrain_operand_type(left, right_ty.as_ref());
                        self.constrain_operand_type(right, left_ty.as_ref());
                    }
                    BinOp::Concat => {
                        if let (Some(lt), Some(rt)) = (&left_ty, &right_ty) {
                            if lt != &SimpleType::String && rt != &SimpleType::String {
                                self.report(*span, format!(
                                    "concatenation operator requires String in at least one operand ({} vs {})",
                                    lt.display_name(),
                                    rt.display_name()
                                ));
                            }
                        }
                    }
                    BinOp::ConcatSpace => {
                        if let (Some(lt), Some(rt)) = (&left_ty, &right_ty) {
                            let is_string_concat = lt == &SimpleType::String || rt == &SimpleType::String;
                            let is_vector_concat = matches!(lt, SimpleType::Vector(_))
                                && matches!(rt, SimpleType::Vector(_))
                                && lt == rt;

                            if !is_string_concat && !is_vector_concat {
                                self.report(*span, format!(
                                    "concatenation operator requires String or vectors of the same type ({} vs {})",
                                    lt.display_name(),
                                    rt.display_name()
                                ));
                            }
                        }
                    }
                }
            }
            Expr::UnaryOp { op, operand, span } => {
                self.check_expr(operand);
                if let Some(ot) = self.infer_simple_type(operand) {
                    match op {
                        UnaryOp::Neg => {
                            if ot != SimpleType::Number {
                                self.report(*span, format!(
                                    "unary operator '-' requires Number (found {})",
                                    ot.display_name()
                                ));
                            }
                        }
                            UnaryOp::Not => {
                            if ot != SimpleType::Boolean {
                                self.report(*span, format!(
                                    "unary operator '!' requires Boolean (found {})",
                                    ot.display_name()
                                ));
                            }
                        }
                    }
                } else if matches!(op, UnaryOp::Neg) {
                    self.constrain_number_operand(operand, Some(&SimpleType::Number));
                }
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
                if let Some(ct) = self.infer_simple_type(condition) {
                    if ct != SimpleType::Boolean {
                        self.report(expr_span(condition), format!(
                            "if condition must be Boolean (found {})",
                            ct.display_name()
                        ));
                    }
                }
                self.with_scope(|this| this.check_expr(then_expr));
                for branch in elif_branches {
                    self.check_expr(&branch.condition);
                    if let Some(ct) = self.infer_simple_type(&branch.condition) {
                        if ct != SimpleType::Boolean {
                            self.report(expr_span(&branch.condition), format!(
                                "elif condition must be Boolean (found {})",
                                ct.display_name()
                            ));
                        }
                    }
                    self.with_scope(|this| this.check_expr(&branch.body));
                }
                self.with_scope(|this| this.check_expr(else_expr));
            }
            Expr::While { condition, body, .. } => {
                self.check_expr(condition);
                if let Some(ct) = self.infer_simple_type(condition) {
                    if ct != SimpleType::Boolean {
                        self.report(expr_span(condition), format!(
                            "while condition must be Boolean (found {})",
                            ct.display_name()
                        ));
                    }
                }
                self.with_scope(|this| this.check_expr(body));
            }
            Expr::For { var, iterable, body, .. } => {
                self.check_expr(iterable);
                self.mark_iterable_usage(iterable);
                self.with_scope(|this| {
                    this.define_var(var, expr_span(expr));
                    if let Some(element_ty) = this.infer_iterable_element_type(iterable) {
                        this.ctx.set_var_type(var, element_ty);
                    }
                    this.check_expr(body);
                    if let Some(loop_item_ty) = this.ctx.var_type(var) {
                        this.record_iterable_element_type(iterable, loop_item_ty);
                    }
                });
            }
            Expr::Let { bindings, body, span } => {
                if bindings.is_empty() {
                    self.report(*span, "let with no bindings".to_string());
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
                    if let Some(simple_ty) = binding
                        .ty
                        .as_ref()
                        .and_then(simple_type_from_type_expr)
                        .or_else(|| self.infer_simple_type(&binding.init))
                    {
                        self.ctx.set_var_type(&binding.name, simple_ty);
                    }
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
                    self.report(*span, "empty block".to_string());
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
                    if let Some(element_ty) = this.infer_iterable_element_type(iterable) {
                        this.ctx.set_var_type(var, element_ty);
                    }
                    this.check_expr(element);
                    if let Some(loop_item_ty) = this.ctx.var_type(var) {
                        this.record_iterable_element_type(iterable, loop_item_ty);
                    }
                });
            }
            Expr::Index { object, index, .. } => {
                self.check_expr(object);
                self.check_expr(index);
                self.mark_iterable_usage(object);
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

    /// Validate the target of an assignment expression.
    fn check_assign_target(&mut self, target: &Expr, span: Span) {
        match target {
            Expr::Ident { name, span } => {
                if is_placeholder(name) {
                    return;
                }
                if !self.ctx.is_var_defined(name) {
                    self.report(*span, format!(
                        "assignment to undefined variable '{}'",
                        name
                    ));
                }
            }
            Expr::FieldAccess { .. } => {
                self.check_expr(target);
            }
            Expr::Index { object, index, .. } => {
                self.check_expr(object);
                self.check_expr(index);
            }
            Expr::Error { .. } => {}
            _ => {
                self.report(span, "invalid assignment target".to_string());
            }
        }
    }

    /// Validate a call to an identifier: variables, functions, macros or builtins.
    fn check_call_ident(&mut self, name: &str, args: &[Expr], span: Span) {
        if is_placeholder(name) {
            return;
        }

        let starts_with_uppercase = name
            .chars()
            .next()
            .map(char::is_uppercase)
            .unwrap_or(false);

        if starts_with_uppercase {
            if !self.ctx.is_constructible_type(name) {
                self.report(span, format!(
                    "type '{}' not defined",
                    name
                ));
                return;
            }

            if let Some(expected) = self.ctx.type_param_count(name) {
                if expected != args.len() {
                    self.report(span, format!(
                        "type '{}' requires {} arguments",
                        name,
                        expected
                    ));
                }
            }
            return;
        }

        if self.ctx.is_var_defined(name) {
            return;
        }
        if self.ctx.has_function(name, args.len()) {
            if let Some(signature) = self.ctx.function_signature(name).cloned() {
                self.check_callable_args(name, args, &signature.params, "call to", span);
            }
            return;
        }
        if self.ctx.has_macro(name, args.len()) {
            return;
        }
        if self.ctx.has_builtin_function(name, args.len()) {
            if let Some(signature) = self
                .ctx
                .builtin_function_signature(name, args.len())
                .cloned()
            {
                self.check_callable_args(name, args, &signature.params, "call to", span);
            }
            return;
        }
        if self.ctx.has_function_name(name)
            || self.ctx.has_macro_name(name)
            || self.ctx.is_builtin_function(name)
        {
            self.report(span, format!(
                "call to '{}' with invalid arity ({})",
                name,
                args.len()
            ));
        } else {
            self.report(span, format!(
                "function '{}' not defined",
                name
            ));
        }
    }

    /// Ensure a type expression refers to a known type.
    fn check_type_expr(&mut self, ty: &TypeExpr, span: Span) {
        match ty {
            TypeExpr::Named(name) => {
                if is_placeholder(name) {
                    return;
                }
                if !self.ctx.is_known_type(name) {
                    self.report(span, format!(
                        "type '{}' not defined",
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

    /// Define a variable in the current scope, reporting redefinitions.
    fn define_var(&mut self, name: &str, span: Span) {
        if is_placeholder(name) {
            return;
        }
        if !self.ctx.define_var(name) {
            self.report(span, format!(
                "variable '{}' already defined in this scope",
                name
            ));
        }
    }

    /// Push a semantic error into the collected errors.
    fn report(&mut self, span: Span, message: String) {
        self.errors.push(SemanticError { span, message });
    }

    /// Conservative type inference for use in checks (vectors, numbers, strings).
    fn infer_simple_type(&self, expr: &Expr) -> Option<SimpleType> {
        match expr {
            Expr::Number { .. } => Some(SimpleType::Number),
            Expr::StringLit { .. } => Some(SimpleType::String),
            Expr::Bool { .. } => Some(SimpleType::Boolean),
            Expr::Ident { name, .. } => self
                .ctx
                .var_type(name)
                .or_else(|| builtin_const_simple_type(name)),
            Expr::New { type_name, .. } => {
                if is_placeholder(type_name) {
                    None
                } else {
                    Some(SimpleType::Named(type_name.clone()))
                }
            }
            Expr::AsType { ty, .. } => simple_type_from_type_expr(ty),
            Expr::Call { callee, args, .. } => {
                if let Expr::Ident { name, .. } = &**callee {
                    return self
                        .ctx
                        .function_signature(name)
                        .and_then(|signature| signature.return_type.clone())
                        .or_else(|| {
                            self.ctx
                                .builtin_function_signature(name, args.len())
                                .and_then(|signature| signature.return_type.clone())
                        })
                        .or_else(|| {
                            if self.ctx.is_constructible_type(name) {
                                Some(SimpleType::Named(name.clone()))
                            } else {
                                None
                            }
                        });
                }
                None
            }
                Expr::If { then_expr, elif_branches, else_expr, .. } => {
                    let then_ty = self.infer_simple_type(then_expr)?;
                    for branch in elif_branches {
                        let branch_ty = self.infer_simple_type(&branch.body)?;
                        if branch_ty != then_ty {
                            return None;
                        }
                    }
                    let else_ty = self.infer_simple_type(else_expr)?;
                    if else_ty == then_ty {
                        Some(then_ty)
                    } else {
                        None
                    }
                }
                Expr::While { body, .. } => self.infer_simple_type(body),
                Expr::For { body, .. } => self.infer_simple_type(body),
                Expr::Let { body, .. } => self.infer_simple_type(body),
                Expr::Assign { value, .. } => self.infer_simple_type(value),
                Expr::Block { exprs, .. } => exprs.last().and_then(|expr| self.infer_simple_type(expr)),
            Expr::MethodCall { object, method, args, .. } => {
                if is_self_ref(object) {
                    return self
                        .ctx
                        .current_type
                        .as_ref()
                        .and_then(|current| current.methods.get(method))
                        .and_then(|by_arity| by_arity.get(&args.len()))
                        .and_then(|signature| signature.return_type.clone());
                }
                // If the object has a named static type, try to resolve the method signature
                if let Some(obj_ty) = self.infer_simple_type(object) {
                    if let SimpleType::Named(type_name) = obj_ty {
                        return self
                            .ctx
                            .type_method_signature(&type_name, method, args.len())
                            .and_then(|s| s.return_type.clone());
                    }
                }
                None
            }
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
                    BinOp::Concat => {
                        if left_ty == SimpleType::String || right_ty == SimpleType::String {
                            Some(SimpleType::String)
                        } else {
                            None
                        }
                    }
                    BinOp::ConcatSpace => {
                        if left_ty == SimpleType::String || right_ty == SimpleType::String {
                            Some(SimpleType::String)
                        } else if matches!((&left_ty, &right_ty), (SimpleType::Vector(_), SimpleType::Vector(_)))
                            && left_ty == right_ty
                        {
                            Some(left_ty)
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

    /// Check that vector literal elements share the same inferred simple type.
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
                            "vector with elements of different types (expected {}, found {})",
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

    /// Execute a closure inside a new variable scope (push/pop).
    fn with_scope<F>(&mut self, f: F)
    where
        F: FnOnce(&mut SemanticChecker),
    {
        self.ctx.push_scope();
        f(self);
        self.ctx.pop_scope();
    }

    /// Validate a callable invocation against the expected parameter types.
    fn check_callable_args(
        &mut self,
        name: &str,
        args: &[Expr],
        expected_params: &[Option<SimpleType>],
        call_kind: &str,
        _span: Span,
    ) {
        for (index, (arg, expected)) in args.iter().zip(expected_params.iter()).enumerate() {
            let Some(expected_ty) = expected else {
                continue;
            };
            let Some(actual_ty) = self.infer_simple_type(arg) else {
                continue;
            };
            if &actual_ty != expected_ty {
                self.report(
                    expr_span(arg),
                    format!(
                        "{} '{}' argument {} expects {}, found {}",
                        call_kind,
                        name,
                        index + 1,
                        expected_ty.display_name(),
                        actual_ty.display_name()
                    ),
                );
            }
        }
    }

    /// Record that an expression is being used as an iterable/vector when possible.
    fn mark_iterable_usage(&mut self, expr: &Expr) {
        if let Expr::Ident { name, span } = expr {
            if let Some(existing_ty) = self.ctx.var_type(name) {
                if !matches!(existing_ty, SimpleType::Vector(_)) {
                    self.report(
                        *span,
                        format!(
                            "iterable expression expects Vector, found {}",
                            existing_ty.display_name()
                        ),
                    );
                }
            }
        }
    }

    /// Constrain an identifier operand to Number when the surrounding operation requires it.
    fn constrain_number_operand(&mut self, expr: &Expr, other_ty: Option<&SimpleType>) {
        if !matches!(other_ty, Some(SimpleType::Number)) {
            return;
        }

        self.constrain_operand_type(expr, other_ty);
    }

    /// Constrain an identifier operand to the type required by the surrounding expression.
    fn constrain_operand_type(&mut self, expr: &Expr, other_ty: Option<&SimpleType>) {
        let Some(other_ty) = other_ty else {
            return;
        };

        if let Expr::Ident { name, .. } = expr {
            if self.ctx.var_type(name).is_none() && self.ctx.is_var_defined(name) {
                self.ctx.set_var_type_in_scope(name, other_ty.clone());
            }
        }
    }

    /// Update the iterable expression to a Vector of the inferred loop element type.
    fn record_iterable_element_type(&mut self, expr: &Expr, element_ty: SimpleType) {
        if let Expr::Ident { name, span } = expr {
            match self.ctx.var_type(name) {
                Some(SimpleType::Vector(existing_inner)) => {
                    if *existing_inner != element_ty {
                        self.report(
                            *span,
                            format!(
                                "iterable expression expects Vector<{}>, found Vector<{}>",
                                element_ty.display_name(),
                                existing_inner.display_name()
                            ),
                        );
                    }
                }
                Some(existing_ty) => {
                    self.report(
                        *span,
                        format!(
                            "iterable expression expects Vector<{}>, found {}",
                            element_ty.display_name(),
                            existing_ty.display_name()
                        ),
                    );
                }
                None => {
                    if self.ctx.is_var_defined(name) {
                        self.ctx.set_var_type_in_scope(
                            name,
                            SimpleType::Vector(Box::new(element_ty)),
                        );
                    }
                }
            }
        }
    }

    /// Infer the type of a single element yielded by an iterable expression.
    fn infer_iterable_element_type(&self, expr: &Expr) -> Option<SimpleType> {
        match self.infer_simple_type(expr)? {
            SimpleType::Vector(inner) => Some(*inner),
            _ => None,
        }
    }

    /// Infer the return type of a callable body if the expression type is known.
    fn infer_callable_body_type(&self, body: &FuncBody) -> Option<SimpleType> {
        let mut ctx = self.ctx.clone();
        match body {
            FuncBody::Inline(expr) | FuncBody::Block(expr) => {
                self.infer_expr_with_scopes(expr, &mut ctx)
            }
        }
    }

    /// Conservative type inference that preserves local scopes created inside expressions.
    fn infer_expr_with_scopes(&self, expr: &Expr, ctx: &mut Context) -> Option<SimpleType> {
        match expr {
            Expr::Number { .. } => Some(SimpleType::Number),
            Expr::StringLit { .. } => Some(SimpleType::String),
            Expr::Bool { .. } => Some(SimpleType::Boolean),
            Expr::Ident { name, .. } => ctx.var_type(name).or_else(|| builtin_const_simple_type(name)),
            Expr::New { type_name, .. } => {
                if is_placeholder(type_name) {
                    None
                } else {
                    Some(SimpleType::Named(type_name.clone()))
                }
            }
            Expr::AsType { ty, .. } => simple_type_from_type_expr(ty),
            Expr::Call { callee, args, .. } => {
                if let Expr::Ident { name, .. } = &**callee {
                    return ctx
                        .function_signature(name)
                        .and_then(|signature| signature.return_type.clone())
                        .or_else(|| {
                            ctx.builtin_function_signature(name, args.len())
                                .and_then(|signature| signature.return_type.clone())
                        });
                }
                None
            }
            Expr::MethodCall { object, method, args, .. } => {
                if is_self_ref(object) {
                    return ctx
                        .current_type
                        .as_ref()
                        .and_then(|current| current.methods.get(method))
                        .and_then(|by_arity| by_arity.get(&args.len()))
                        .and_then(|signature| signature.return_type.clone());
                }
                if let Some(obj_ty) = self.infer_expr_with_scopes(object, ctx) {
                    if let SimpleType::Named(type_name) = obj_ty {
                        return ctx
                            .type_method_signature(&type_name, method, args.len())
                            .and_then(|signature| signature.return_type.clone());
                    }
                }
                None
            }
            Expr::UnaryOp { op, operand, .. } => {
                let inner = self.infer_expr_with_scopes(operand, ctx)?;
                match (op, inner) {
                    (UnaryOp::Neg, SimpleType::Number) => Some(SimpleType::Number),
                    (UnaryOp::Not, SimpleType::Boolean) => Some(SimpleType::Boolean),
                    _ => None,
                }
            }
            Expr::BinaryOp { op, left, right, .. } => {
                let left_ty = self.infer_expr_with_scopes(left, ctx)?;
                let right_ty = self.infer_expr_with_scopes(right, ctx)?;
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
                    BinOp::Concat => {
                        if left_ty == SimpleType::String || right_ty == SimpleType::String {
                            Some(SimpleType::String)
                        } else {
                            None
                        }
                    }
                    BinOp::ConcatSpace => {
                        if left_ty == SimpleType::String || right_ty == SimpleType::String {
                            Some(SimpleType::String)
                        } else if matches!((&left_ty, &right_ty), (SimpleType::Vector(_), SimpleType::Vector(_)))
                            && left_ty == right_ty
                        {
                            Some(left_ty)
                        } else {
                            None
                        }
                    }
                }
            }
            Expr::If { then_expr, elif_branches, else_expr, .. } => {
                let then_ty = {
                    let mut branch_ctx = ctx.clone();
                    branch_ctx.push_scope();
                    self.infer_expr_with_scopes(then_expr, &mut branch_ctx)?
                };
                for branch in elif_branches {
                    let branch_ty = {
                        let mut branch_ctx = ctx.clone();
                        branch_ctx.push_scope();
                        self.infer_expr_with_scopes(&branch.body, &mut branch_ctx)?
                    };
                    if branch_ty != then_ty {
                        return None;
                    }
                }
                let else_ty = {
                    let mut branch_ctx = ctx.clone();
                    branch_ctx.push_scope();
                    self.infer_expr_with_scopes(else_expr, &mut branch_ctx)?
                };
                if else_ty == then_ty {
                    Some(then_ty)
                } else {
                    None
                }
            }
            Expr::While { body, .. } => {
                let mut loop_ctx = ctx.clone();
                loop_ctx.push_scope();
                self.infer_expr_with_scopes(body, &mut loop_ctx)
            }
            Expr::For { var, body, .. } => {
                let mut loop_ctx = ctx.clone();
                loop_ctx.push_scope();
                loop_ctx.define_var(var);
                self.infer_expr_with_scopes(body, &mut loop_ctx)
            }
            Expr::Let { bindings, body, .. } => {
                let mut local_ctx = ctx.clone();
                for binding in bindings {
                    let init_ty = self.infer_expr_with_scopes(&binding.init, &mut local_ctx);
                    local_ctx.push_scope();
                    local_ctx.define_var(&binding.name);
                    if let Some(ty) = binding
                        .ty
                        .as_ref()
                        .and_then(simple_type_from_type_expr)
                        .or(init_ty)
                    {
                        local_ctx.set_var_type(&binding.name, ty);
                    }
                }
                self.infer_expr_with_scopes(body, &mut local_ctx)
            }
            Expr::Assign { value, .. } => self.infer_expr_with_scopes(value, ctx),
            Expr::Block { exprs, .. } => {
                if exprs.is_empty() {
                    return None;
                }
                let mut local_ctx = ctx.clone();
                local_ctx.push_scope();
                let mut result = None;
                for expr in exprs {
                    result = self.infer_expr_with_scopes(expr, &mut local_ctx);
                }
                result
            }
            Expr::VectorLit { elements, .. } => {
                let mut expected: Option<SimpleType> = None;
                for element in elements {
                    let Some(element_ty) = self.infer_expr_with_scopes(element, ctx) else {
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
            Expr::VectorGen { element, iterable, .. } => {
                self.infer_expr_with_scopes(iterable, ctx)?;
                self.infer_expr_with_scopes(element, ctx)
            }
            Expr::Index { object, .. } => self.infer_expr_with_scopes(object, ctx),
            Expr::Lambda { params, return_type, body, .. } => {
                let mut lambda_ctx = ctx.clone();
                lambda_ctx.push_scope();
                for param in params {
                    lambda_ctx.define_var(&param.name);
                    if let Some(ty) = &param.ty {
                        if let Some(simple_ty) = simple_type_from_type_expr(ty) {
                            lambda_ctx.set_var_type(&param.name, simple_ty);
                        }
                    }
                }
                if let Some(ty) = return_type {
                    simple_type_from_type_expr(ty)
                } else {
                    match body {
                        FuncBody::Inline(expr) | FuncBody::Block(expr) => {
                            self.infer_expr_with_scopes(expr, &mut lambda_ctx)
                        }
                    }
                }
            }
            Expr::Error { .. } => None,
            Expr::IsType { .. } => Some(SimpleType::Boolean),
            Expr::SelfRef { .. } | Expr::Base { .. } => self.infer_simple_type(expr),
            Expr::FieldAccess { .. } => self.infer_simple_type(expr),
        }
    }

    /// Compare an inferred return type against an optional declared return type.
    fn check_return_type(
        &mut self,
        callable_kind: &str,
        callable_name: &str,
        declared_return_type: Option<&TypeExpr>,
        inferred_return_type: Option<SimpleType>,
        span: Span,
    ) {
        let declared_return_type = declared_return_type.and_then(simple_type_from_type_expr);
        if let (Some(expected), Some(actual)) = (
            declared_return_type.as_ref(),
            inferred_return_type.as_ref(),
        ) {
            if expected != actual {
                self.report(
                    span,
                    format!(
                        "{} '{}' return type expects {}, found {}",
                        callable_kind,
                        callable_name,
                        expected.display_name(),
                        actual.display_name()
                    ),
                );
            }
        }
    }
}

/// Map certain builtin constants to simple types.
fn builtin_const_simple_type(name: &str) -> Option<SimpleType> {
    match name {
        "PI" | "E" => Some(SimpleType::Number),
        _ => None,
    }
}

/// Convert a TypeExpr to a SimpleType when possible.
fn simple_type_from_type_expr(ty: &TypeExpr) -> Option<SimpleType> {
    match ty {
        TypeExpr::Named(name) => match name.as_str() {
            "Number" => Some(SimpleType::Number),
            "String" => Some(SimpleType::String),
            "Boolean" => Some(SimpleType::Boolean),
            _ => Some(SimpleType::Named(name.clone())),
        },
        TypeExpr::Iterable(inner) | TypeExpr::Vector(inner) => {
            simple_type_from_type_expr(inner).map(|inner_ty| SimpleType::Vector(Box::new(inner_ty)))
        }
        TypeExpr::Functor { .. } => None,
    }
}

/// Convert typed parameters into simplified types for call checking.
fn callable_params_from_params(params: &[Param]) -> Vec<Option<SimpleType>> {
    params
        .iter()
        .map(|param| param.ty.as_ref().and_then(simple_type_from_type_expr))
        .collect()
}

/// Is this a placeholder name produced by the parser?
fn is_placeholder(name: &str) -> bool {
    name.is_empty() || name == "__parse_error__"
}

/// Is the expression a `self` reference?
fn is_self_ref(expr: &Expr) -> bool {
    matches!(expr, Expr::SelfRef { .. })
}

/// Extract the span from an expression.
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

/// Extract the parameter name from a MacroParam if present.
fn macro_param_name(param: &MacroParam) -> Option<String> {
    match param {
        MacroParam::Regular(p) => Some(p.name.clone()),
        MacroParam::Block { name, .. }
        | MacroParam::Symbolic { name, .. }
        | MacroParam::Placeholder { name, .. } => Some(name.clone()),
    }
}

/// Extract the span from a MacroParam.
fn macro_param_span(param: &MacroParam) -> Span {
    match param {
        MacroParam::Regular(p) => p.span,
        MacroParam::Block { span, .. }
        | MacroParam::Symbolic { span, .. }
        | MacroParam::Placeholder { span, .. } => *span,
    }
}