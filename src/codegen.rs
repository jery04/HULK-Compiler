//! Backend: lower the type-checked HULK AST to C source, then let the system C
//! compiler turn it into the native `./output` binary required by the contract.
//!
//! Runtime model: every HULK value is a tagged `Value` (number/string/bool/object).
//! Objects carry a `type_id` and a flat array of attribute slots indexed by a
//! *global* attribute-name → slot map; method dispatch goes through a per-type
//! vtable indexed by a global method-name → slot map. This keeps codegen fully
//! decoupled from static types: operators and dispatch resolve on the runtime tag.

use std::collections::HashMap;
use std::fmt::Write as _;

use crate::parser::{
    BinOp, Decl, Expr, FuncBody, FuncDecl, MethodDef, Program, TypeDecl, TypeMember, UnaryOp,
};

/// Generate a complete C translation unit for `program`.
pub fn emit_c(program: &Program) -> String {
    let mut cg = Codegen::new(program);
    cg.run()
}

struct TypeInfo<'a> {
    decl: &'a TypeDecl,
    id: usize,
    parent: Option<String>,
}

struct Codegen<'a> {
    program: &'a Program,
    funcs: HashMap<String, &'a FuncDecl>,
    types: HashMap<String, TypeInfo<'a>>,
    type_order: Vec<String>,
    attr_slots: HashMap<String, usize>,
    method_slots: HashMap<String, usize>,
    // mutable per-function state
    body: String,
    temp: usize,
    var: usize,
    scopes: Vec<HashMap<String, String>>,
    // method context for `base`
    cur_type: Option<String>,
    cur_method: Option<String>,
}

impl<'a> Codegen<'a> {
    fn new(program: &'a Program) -> Self {
        let mut funcs = HashMap::new();
        for d in &program.decls {
            if let Decl::Function(f) = d {
                funcs.insert(f.name.clone(), f);
            }
        }

        let mut types = HashMap::new();
        let mut type_order = Vec::new();
        let mut attr_slots: HashMap<String, usize> = HashMap::new();
        let mut method_slots: HashMap<String, usize> = HashMap::new();
        for d in &program.decls {
            if let Decl::Type(t) = d {
                let id = type_order.len();
                let parent = t.inherits.as_ref().map(|i| i.parent.clone());
                types.insert(t.name.clone(), TypeInfo { decl: t, id, parent });
                type_order.push(t.name.clone());
                for m in &t.members {
                    match m {
                        TypeMember::Attribute(a) => {
                            let n = attr_slots.len();
                            attr_slots.entry(a.name.clone()).or_insert(n);
                        }
                        TypeMember::Method(me) => {
                            let n = method_slots.len();
                            method_slots.entry(me.name.clone()).or_insert(n);
                        }
                    }
                }
            }
        }

        Codegen {
            program,
            funcs,
            types,
            type_order,
            attr_slots,
            method_slots,
            body: String::new(),
            temp: 0,
            var: 0,
            scopes: Vec::new(),
            cur_type: None,
            cur_method: None,
        }
    }

    fn run(&mut self) -> String {
        let mut out = String::new();
        out.push_str(RUNTIME_PREAMBLE);

        let n_attr = self.attr_slots.len().max(1);
        let n_meth = self.method_slots.len().max(1);
        let n_types = self.type_order.len().max(1);
        let _ = writeln!(out, "#define NUM_ATTR_SLOTS {}", n_attr);
        let _ = writeln!(out, "#define NUM_METHOD_SLOTS {}", n_meth);
        let _ = writeln!(out, "#define NUM_TYPES {}", n_types);
        out.push_str("typedef Value (*Method)(Value, Value*);\n");
        out.push_str("static Method vtables[NUM_TYPES][NUM_METHOD_SLOTS];\n");
        out.push_str("static int parent_id[NUM_TYPES];\n");
        out.push_str(
            "static Value hulk_is(Value v, int target){\n\
            \x20   if (v.tag != TAG_OBJ) return mk_bool(0);\n\
            \x20   int id = v.obj->type_id;\n\
            \x20   while (id >= 0) { if (id == target) return mk_bool(1); id = parent_id[id]; }\n\
            \x20   return mk_bool(0);\n\
            }\n\n",
        );

        // Forward declarations.
        for f in self.funcs.values() {
            let _ = writeln!(out, "static Value hulk_fn_{}(Value* args);", f.name);
        }
        for tn in &self.type_order {
            let _ = writeln!(out, "static Value hulk_new_{}(Value* args);", tn);
            let _ = writeln!(out, "static void hulk_initall_{}(Obj* self_o, Value* args);", tn);
            let info = &self.types[tn];
            for m in &info.decl.members {
                if let TypeMember::Method(me) = m {
                    let _ = writeln!(out, "static Value hulk_m_{}_{}(Value self, Value* args);", tn, me.name);
                }
            }
        }
        out.push('\n');

        // Function definitions.
        let func_names: Vec<String> = self.funcs.keys().cloned().collect();
        for name in func_names {
            let f = self.funcs[&name];
            let code = self.gen_function(f);
            out.push_str(&code);
        }

        // Type constructors + methods.
        let type_names = self.type_order.clone();
        for tn in &type_names {
            let code = self.gen_type(tn);
            out.push_str(&code);
        }

        // vtable / parent initialization.
        out.push_str("static void init_tables(void) {\n");
        for tn in &type_names {
            let info = &self.types[tn];
            let tid = info.id;
            // parent id
            let pid = match &info.parent {
                Some(p) => self.types.get(p).map(|t| t.id as i64).unwrap_or(-1),
                None => -1,
            };
            let _ = writeln!(out, "    parent_id[{}] = {};", tid, pid);
            // for each method slot, resolve most-derived implementation in chain
            for (mname, slot) in &self.method_slots {
                if let Some(impl_ty) = self.resolve_method(tn, mname) {
                    let _ = writeln!(
                        out,
                        "    vtables[{}][{}] = hulk_m_{}_{};",
                        tid, slot, impl_ty, mname
                    );
                }
            }
        }
        out.push_str("}\n\n");

        // main()
        out.push_str("int main(void) {\n    init_tables();\n");
        self.begin_function();
        let expr_code_var = self.gen_expr(&self.program.expr.clone());
        let body = std::mem::take(&mut self.body);
        out.push_str(&body);
        let _ = writeln!(out, "    (void){};", expr_code_var);
        out.push_str("    return 0;\n}\n");
        out
    }

    /// Most-derived type in [start, ancestors...] that declares `method`.
    fn resolve_method(&self, start: &str, method: &str) -> Option<String> {
        let mut cur = Some(start.to_string());
        while let Some(tn) = cur {
            let info = self.types.get(&tn)?;
            for m in &info.decl.members {
                if let TypeMember::Method(me) = m {
                    if me.name == method {
                        return Some(tn.clone());
                    }
                }
            }
            cur = info.parent.clone();
        }
        None
    }

    // ── per-function machinery ──────────────────────────────────────────────

    fn begin_function(&mut self) {
        self.body.clear();
        self.temp = 0;
        self.var = 0;
        self.scopes.clear();
        self.scopes.push(HashMap::new());
    }

    fn fresh(&mut self) -> String {
        let t = format!("t{}", self.temp);
        self.temp += 1;
        t
    }

    fn new_var(&mut self, hulk_name: &str) -> String {
        let v = format!("v{}_{}", self.var, sanitize(hulk_name));
        self.var += 1;
        self.scopes.last_mut().unwrap().insert(hulk_name.to_string(), v.clone());
        v
    }

    fn lookup(&self, name: &str) -> Option<String> {
        for s in self.scopes.iter().rev() {
            if let Some(v) = s.get(name) {
                return Some(v.clone());
            }
        }
        None
    }

    fn emit(&mut self, line: &str) {
        self.body.push_str("    ");
        self.body.push_str(line);
        self.body.push('\n');
    }

    fn gen_function(&mut self, f: &FuncDecl) -> String {
        self.begin_function();
        for (i, p) in f.params.iter().enumerate() {
            let v = self.new_var(&p.name);
            self.emit(&format!("Value {} = args[{}];", v, i));
        }
        let body_expr = match &f.body {
            FuncBody::Inline(e) | FuncBody::Block(e) => e.clone(),
        };
        let r = self.gen_expr(&body_expr);
        let mut out = String::new();
        let _ = writeln!(out, "static Value hulk_fn_{}(Value* args) {{", f.name);
        out.push_str("    (void)args;\n");
        out.push_str(&std::mem::take(&mut self.body));
        let _ = writeln!(out, "    return {};", r);
        out.push_str("}\n\n");
        out
    }

    fn gen_type(&mut self, tn: &str) -> String {
        let info = &self.types[tn];
        let tid = info.id;
        let type_params: Vec<(String, ())> = info.decl.type_params.iter().map(|p| (p.name.clone(), ())).collect();
        let parent = info.parent.clone();
        let inherits_args: Vec<Expr> = info
            .decl
            .inherits
            .as_ref()
            .map(|i| i.args.clone())
            .unwrap_or_default();
        let attrs: Vec<(String, Expr)> = info
            .decl
            .members
            .iter()
            .filter_map(|m| match m {
                TypeMember::Attribute(a) => Some((a.name.clone(), (*a.init).clone())),
                _ => None,
            })
            .collect();
        let methods: Vec<MethodDef> = info
            .decl
            .members
            .iter()
            .filter_map(|m| match m {
                TypeMember::Method(me) => Some(me.clone()),
                _ => None,
            })
            .collect();

        let mut out = String::new();

        // initall
        self.begin_function();
        self.emit("Value self_v; self_v.tag = TAG_OBJ; self_v.obj = self_o;");
        self.scopes.last_mut().unwrap().insert("self".to_string(), "self_v".to_string());
        for (i, (pname, _)) in type_params.iter().enumerate() {
            let v = self.new_var(pname);
            self.emit(&format!("Value {} = args[{}];", v, i));
        }
        if let Some(p) = &parent {
            // compute parent args
            let mut arg_vars = Vec::new();
            for a in &inherits_args {
                let r = self.gen_expr(a);
                arg_vars.push(r);
            }
            let pa = self.fresh();
            if arg_vars.is_empty() {
                self.emit(&format!("Value* {} = NULL;", pa));
            } else {
                self.emit(&format!("Value {}[{}];", pa, arg_vars.len()));
                for (i, av) in arg_vars.iter().enumerate() {
                    self.emit(&format!("{}[{}] = {};", pa, i, av));
                }
            }
            self.emit(&format!("hulk_initall_{}(self_o, {});", p, pa));
        }
        for (aname, init) in &attrs {
            let r = self.gen_expr(init);
            let slot = self.attr_slots[aname];
            self.emit(&format!("self_o->fields[{}] = {};", slot, r));
        }
        let _ = writeln!(out, "static void hulk_initall_{}(Obj* self_o, Value* args) {{", tn);
        out.push_str("    (void)args;\n");
        out.push_str(&std::mem::take(&mut self.body));
        out.push_str("}\n\n");

        // constructor
        let _ = writeln!(out, "static Value hulk_new_{}(Value* args) {{", tn);
        out.push_str("    Obj* o = (Obj*)malloc(sizeof(Obj));\n");
        let _ = writeln!(out, "    o->type_id = {};", tid);
        out.push_str("    o->fields = (Value*)calloc(NUM_ATTR_SLOTS, sizeof(Value));\n");
        let _ = writeln!(out, "    hulk_initall_{}(o, args);", tn);
        out.push_str("    Value v; v.tag = TAG_OBJ; v.obj = o; return v;\n}\n\n");

        // methods
        for me in &methods {
            self.begin_function();
            self.cur_type = Some(tn.to_string());
            self.cur_method = Some(me.name.clone());
            self.scopes.last_mut().unwrap().insert("self".to_string(), "self".to_string());
            for (i, p) in me.params.iter().enumerate() {
                let v = self.new_var(&p.name);
                self.emit(&format!("Value {} = args[{}];", v, i));
            }
            let body_expr = match &me.body {
                FuncBody::Inline(e) | FuncBody::Block(e) => e.clone(),
            };
            let r = self.gen_expr(&body_expr);
            self.cur_type = None;
            self.cur_method = None;
            let _ = writeln!(out, "static Value hulk_m_{}_{}(Value self, Value* args) {{", tn, me.name);
            out.push_str("    (void)args; (void)self;\n");
            out.push_str(&std::mem::take(&mut self.body));
            let _ = writeln!(out, "    return {};", r);
            out.push_str("}\n\n");
        }

        out
    }

    // ── expression lowering: returns the name of a C var holding the Value ──

    fn gen_expr(&mut self, e: &Expr) -> String {
        match e {
            Expr::Number { value, .. } => {
                let t = self.fresh();
                self.emit(&format!("Value {} = mk_num({});", t, value));
                t
            }
            Expr::StringLit { value, .. } => {
                let t = self.fresh();
                self.emit(&format!("Value {} = mk_str(\"{}\");", t, escape_c(value)));
                t
            }
            Expr::Bool { value, .. } => {
                let t = self.fresh();
                self.emit(&format!("Value {} = mk_bool({});", t, if *value { 1 } else { 0 }));
                t
            }
            Expr::Ident { name, .. } => {
                if let Some(v) = self.lookup(name) {
                    v
                } else {
                    // Unknown identifier as a value: should not occur in checked programs.
                    let t = self.fresh();
                    self.emit(&format!("Value {} = mk_num(0); /* unresolved {} */", t, name));
                    t
                }
            }
            Expr::Block { exprs, .. } => {
                let mut last = String::new();
                if exprs.is_empty() {
                    let t = self.fresh();
                    self.emit(&format!("Value {} = mk_num(0);", t));
                    return t;
                }
                for e in exprs {
                    last = self.gen_expr(e);
                }
                last
            }
            Expr::Let { bindings, body, .. } => {
                self.scopes.push(HashMap::new());
                for b in bindings {
                    let r = self.gen_expr(&b.init);
                    let v = self.new_var(&b.name);
                    self.emit(&format!("Value {} = {};", v, r));
                }
                let res = self.gen_expr(body);
                self.scopes.pop();
                res
            }
            Expr::BinaryOp { op, left, right, .. } => self.gen_binop(*op, left, right),
            Expr::UnaryOp { op, operand, .. } => {
                let a = self.gen_expr(operand);
                let t = self.fresh();
                match op {
                    UnaryOp::Neg => self.emit(&format!("Value {} = hulk_neg({});", t, a)),
                    UnaryOp::Not => self.emit(&format!("Value {} = hulk_not({});", t, a)),
                }
                t
            }
            Expr::If { condition, then_expr, elif_branches, else_expr, .. } => {
                let r = self.fresh();
                self.emit(&format!("Value {}; {}.tag = TAG_NUM; {}.num = 0;", r, r, r));
                let c = self.gen_expr(condition);
                self.emit(&format!("if ({}.b) {{", c));
                let tv = self.gen_expr(then_expr);
                self.emit(&format!("{} = {};", r, tv));
                self.emit("} else {");
                // chain elifs
                let mut open = 1;
                for eb in elif_branches {
                    let cc = self.gen_expr(&eb.condition);
                    self.emit(&format!("if ({}.b) {{", cc));
                    let bv = self.gen_expr(&eb.body);
                    self.emit(&format!("{} = {};", r, bv));
                    self.emit("} else {");
                    open += 1;
                }
                let ev = self.gen_expr(else_expr);
                self.emit(&format!("{} = {};", r, ev));
                for _ in 0..open {
                    self.emit("}");
                }
                r
            }
            Expr::While { condition, body, .. } => {
                let r = self.fresh();
                self.emit(&format!("Value {} = mk_num(0);", r));
                self.emit("while (1) {");
                let c = self.gen_expr(condition);
                self.emit(&format!("if (!{}.b) break;", c));
                let bv = self.gen_expr(body);
                self.emit(&format!("{} = {};", r, bv));
                self.emit("}");
                r
            }
            Expr::Assign { target, value, .. } => {
                let val = self.gen_expr(value);
                match &**target {
                    Expr::Ident { name, .. } => {
                        if let Some(v) = self.lookup(name) {
                            self.emit(&format!("{} = {};", v, val));
                        }
                        val
                    }
                    Expr::FieldAccess { object, field, .. } => {
                        let o = self.gen_expr(object);
                        let slot = *self.attr_slots.get(field).unwrap_or(&0);
                        self.emit(&format!("{}.obj->fields[{}] = {};", o, slot, val));
                        val
                    }
                    _ => val,
                }
            }
            Expr::FieldAccess { object, field, .. } => {
                let o = self.gen_expr(object);
                let slot = *self.attr_slots.get(field).unwrap_or(&0);
                let t = self.fresh();
                self.emit(&format!("Value {} = {}.obj->fields[{}];", t, o, slot));
                t
            }
            Expr::New { type_name, args, .. } => {
                let argv = self.gen_args(args);
                let t = self.fresh();
                self.emit(&format!("Value {} = hulk_new_{}({});", t, type_name, argv));
                t
            }
            Expr::MethodCall { object, method, args, .. } => {
                let o = self.gen_expr(object);
                let argv = self.gen_args(args);
                let slot = *self.method_slots.get(method).unwrap_or(&0);
                let t = self.fresh();
                self.emit(&format!(
                    "Value {} = vtables[{}.obj->type_id][{}]({}, {});",
                    t, o, slot, o, argv
                ));
                t
            }
            Expr::Call { callee, args, .. } => self.gen_call(callee, args),
            Expr::Base { args, .. } => {
                // call parent implementation of the current method
                let argv = self.gen_args(args);
                let t = self.fresh();
                if let (Some(ct), Some(cm)) = (self.cur_type.clone(), self.cur_method.clone()) {
                    let parent = self.types.get(&ct).and_then(|i| i.parent.clone());
                    if let Some(p) = parent {
                        if let Some(impl_ty) = self.resolve_method(&p, &cm) {
                            self.emit(&format!("Value {} = hulk_m_{}_{}(self, {});", t, impl_ty, cm, argv));
                            return t;
                        }
                    }
                }
                self.emit(&format!("Value {} = mk_num(0);", t));
                t
            }
            Expr::IsType { expr, ty, .. } => {
                let v = self.gen_expr(expr);
                let t = self.fresh();
                let tyname = type_expr_name(ty);
                if let Some(info) = self.types.get(&tyname) {
                    self.emit(&format!("Value {} = hulk_is({}, {});", t, v, info.id));
                } else {
                    // builtin type test on tag
                    let tag = match tyname.as_str() {
                        "Number" => "TAG_NUM",
                        "String" => "TAG_STR",
                        "Boolean" => "TAG_BOOL",
                        _ => "TAG_OBJ",
                    };
                    self.emit(&format!("Value {} = mk_bool({}.tag == {});", t, v, tag));
                }
                t
            }
            Expr::AsType { expr, .. } => self.gen_expr(expr),
            Expr::For { var, iterable, body, .. } => {
                let result = self.fresh();
                self.emit(&format!("Value {} = mk_num(0);", result));

                // `for (x in range(lo, hi))` lowers to a counted loop.
                if let Expr::Call { callee, args, .. } = &**iterable {
                    if let Expr::Ident { name, .. } = &**callee {
                        if name == "range" && args.len() == 2 {
                            let lo = self.gen_expr(&args[0]);
                            let hi = self.gen_expr(&args[1]);
                            let idx = self.fresh();
                            self.emit(&format!(
                                "for (double {i} = {lo}.num; {i} < {hi}.num; {i} += 1.0) {{",
                                i = idx, lo = lo, hi = hi
                            ));
                            self.scopes.push(HashMap::new());
                            let xv = self.new_var(var);
                            self.emit(&format!("Value {} = mk_num({});", xv, idx));
                            let _ = self.gen_expr(body);
                            self.scopes.pop();
                            self.emit("}");
                            return result;
                        }
                    }
                }

                // General iterable object: drive the HULK iterator protocol
                // `next(): Boolean` / `current(): T` through dynamic dispatch.
                let it = self.gen_expr(iterable);
                let snext = *self.method_slots.get("next").unwrap_or(&0);
                let scur = *self.method_slots.get("current").unwrap_or(&0);
                self.emit(&format!(
                    "while (vtables[{it}.obj->type_id][{sn}]({it}, NULL).b) {{",
                    it = it, sn = snext
                ));
                self.scopes.push(HashMap::new());
                let xv = self.new_var(var);
                self.emit(&format!(
                    "Value {} = vtables[{it}.obj->type_id][{sc}]({it}, NULL);",
                    xv, it = it, sc = scur
                ));
                let _ = self.gen_expr(body);
                self.scopes.pop();
                self.emit("}");
                result
            }
            Expr::Error { .. } => {
                let t = self.fresh();
                self.emit(&format!("Value {} = mk_num(0);", t));
                t
            }
        }
    }

    fn gen_args(&mut self, args: &[Expr]) -> String {
        if args.is_empty() {
            return "NULL".to_string();
        }
        let mut vals = Vec::new();
        for a in args {
            vals.push(self.gen_expr(a));
        }
        let arr = self.fresh();
        self.emit(&format!("Value {}[{}];", arr, vals.len()));
        for (i, v) in vals.iter().enumerate() {
            self.emit(&format!("{}[{}] = {};", arr, i, v));
        }
        arr
    }

    fn gen_call(&mut self, callee: &Expr, args: &[Expr]) -> String {
        if let Expr::Ident { name, .. } = callee {
            match name.as_str() {
                "print" => {
                    let a = self.gen_expr(&args[0]);
                    let t = self.fresh();
                    self.emit(&format!("Value {} = hulk_print({});", t, a));
                    return t;
                }
                "sqrt" | "sin" | "cos" | "exp" | "log" => {
                    let a = self.gen_expr(&args[0]);
                    let t = self.fresh();
                    self.emit(&format!("Value {} = mk_num({}({}.num));", t, name, a));
                    return t;
                }
                "rand" => {
                    let t = self.fresh();
                    self.emit(&format!("Value {} = mk_num((double)rand() / (double)RAND_MAX);", t));
                    return t;
                }
                _ => {
                    if self.funcs.contains_key(name) {
                        let argv = self.gen_args(args);
                        let t = self.fresh();
                        self.emit(&format!("Value {} = hulk_fn_{}({});", t, name, argv));
                        return t;
                    }
                }
            }
        }
        // Fallback: evaluate args, return 0.
        let _ = self.gen_args(args);
        let t = self.fresh();
        self.emit(&format!("Value {} = mk_num(0);", t));
        t
    }

    fn gen_binop(&mut self, op: BinOp, left: &Expr, right: &Expr) -> String {
        let a = self.gen_expr(left);
        let b = self.gen_expr(right);
        let t = self.fresh();
        let call = match op {
            BinOp::Add => format!("hulk_add({}, {})", a, b),
            BinOp::Sub => format!("hulk_sub({}, {})", a, b),
            BinOp::Mul => format!("hulk_mul({}, {})", a, b),
            BinOp::Div => format!("hulk_div({}, {})", a, b),
            BinOp::Mod => format!("hulk_mod({}, {})", a, b),
            BinOp::Pow => format!("hulk_pow({}, {})", a, b),
            BinOp::Eq => format!("hulk_eq({}, {})", a, b),
            BinOp::NotEq => format!("hulk_neq({}, {})", a, b),
            BinOp::Lt => format!("hulk_lt({}, {})", a, b),
            BinOp::Gt => format!("hulk_gt({}, {})", a, b),
            BinOp::LtEq => format!("hulk_le({}, {})", a, b),
            BinOp::GtEq => format!("hulk_ge({}, {})", a, b),
            BinOp::And => format!("hulk_and({}, {})", a, b),
            BinOp::Or => format!("hulk_or({}, {})", a, b),
            BinOp::Concat => format!("hulk_concat({}, {}, 0)", a, b),
            BinOp::ConcatSpace => format!("hulk_concat({}, {}, 1)", a, b),
        };
        self.emit(&format!("Value {} = {};", t, call));
        t
    }
}

fn sanitize(name: &str) -> String {
    name.chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '_' { c } else { '_' })
        .collect()
}

fn escape_c(s: &str) -> String {
    let mut out = String::new();
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            '\r' => out.push_str("\\r"),
            _ => out.push(c),
        }
    }
    out
}

fn type_expr_name(ty: &crate::parser::TypeExpr) -> String {
    match ty {
        crate::parser::TypeExpr::Named(n) => n.clone(),
        crate::parser::TypeExpr::Iterable(t) | crate::parser::TypeExpr::Vector(t) => type_expr_name(t),
    }
}

const RUNTIME_PREAMBLE: &str = r#"#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <math.h>

#define TAG_NUM 0
#define TAG_STR 1
#define TAG_BOOL 2
#define TAG_OBJ 3

typedef struct Obj Obj;
typedef struct { int tag; double num; char* str; int b; Obj* obj; } Value;
struct Obj { int type_id; Value* fields; };

static Value mk_num(double n){ Value v; v.tag=TAG_NUM; v.num=n; v.str=0; v.b=0; v.obj=0; return v; }
static Value mk_bool(int b){ Value v; v.tag=TAG_BOOL; v.b=b; v.num=0; v.str=0; v.obj=0; return v; }
static Value mk_str(const char* s){ Value v; v.tag=TAG_STR; v.str=(char*)s; v.num=0; v.b=0; v.obj=0; return v; }

static char* num_to_str(double n){
    char* buf = (char*)malloc(32);
    if (n == (long long)n && n < 1e15 && n > -1e15) snprintf(buf, 32, "%lld", (long long)n);
    else snprintf(buf, 32, "%g", n);
    return buf;
}
static char* val_to_str(Value v){
    if (v.tag == TAG_STR) return v.str ? v.str : (char*)"";
    if (v.tag == TAG_NUM) return num_to_str(v.num);
    if (v.tag == TAG_BOOL) return v.b ? (char*)"true" : (char*)"false";
    return (char*)"<object>";
}

static Value hulk_add(Value a, Value b){ return mk_num(a.num + b.num); }
static Value hulk_sub(Value a, Value b){ return mk_num(a.num - b.num); }
static Value hulk_mul(Value a, Value b){ return mk_num(a.num * b.num); }
static Value hulk_div(Value a, Value b){ return mk_num(a.num / b.num); }
static Value hulk_mod(Value a, Value b){ return mk_num(fmod(a.num, b.num)); }
static Value hulk_pow(Value a, Value b){ return mk_num(pow(a.num, b.num)); }
static Value hulk_neg(Value a){ return mk_num(-a.num); }
static Value hulk_lt(Value a, Value b){ return mk_bool(a.num <  b.num); }
static Value hulk_gt(Value a, Value b){ return mk_bool(a.num >  b.num); }
static Value hulk_le(Value a, Value b){ return mk_bool(a.num <= b.num); }
static Value hulk_ge(Value a, Value b){ return mk_bool(a.num >= b.num); }
static Value hulk_and(Value a, Value b){ return mk_bool(a.b && b.b); }
static Value hulk_or(Value a, Value b){ return mk_bool(a.b || b.b); }
static Value hulk_not(Value a){ return mk_bool(!a.b); }

static Value hulk_eq(Value a, Value b){
    if (a.tag != b.tag) return mk_bool(0);
    if (a.tag == TAG_NUM) return mk_bool(a.num == b.num);
    if (a.tag == TAG_STR) return mk_bool(strcmp(a.str?a.str:"", b.str?b.str:"") == 0);
    if (a.tag == TAG_BOOL) return mk_bool(a.b == b.b);
    if (a.tag == TAG_OBJ) return mk_bool(a.obj == b.obj);
    return mk_bool(0);
}
static Value hulk_neq(Value a, Value b){ return mk_bool(!hulk_eq(a,b).b); }

static Value hulk_concat(Value a, Value b, int space){
    char* sa = val_to_str(a);
    char* sb = val_to_str(b);
    size_t la = strlen(sa), lb = strlen(sb);
    char* r = (char*)malloc(la + lb + 2);
    memcpy(r, sa, la);
    size_t off = la;
    if (space) { r[off++] = ' '; }
    memcpy(r + off, sb, lb);
    r[off + lb] = 0;
    return mk_str(r);
}

static Value hulk_print(Value v){
    printf("%s\n", val_to_str(v));
    return v;
}
"#;
