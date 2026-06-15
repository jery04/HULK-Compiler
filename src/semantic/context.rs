use std::collections::{HashMap, HashSet};

use super::checker::SimpleType;

/// Signature for a callable value (function, builtin or method).
#[derive(Clone)]
pub(super) struct CallableSignature {
    pub(super) params: Vec<Option<SimpleType>>,
    pub(super) return_type: Option<SimpleType>,
}

/// Context holds scoped variables, type/function registries and builtins.
#[derive(Clone)]
pub struct Context {
    var_scopes: Vec<HashSet<String>>,
    var_types: Vec<HashMap<String, SimpleType>>,
    functions: HashMap<String, HashSet<usize>>,
    function_signatures: HashMap<String, CallableSignature>,
    types: HashMap<String, TypeInfo>,
    protocols: HashMap<String, ProtocolInfo>,
    builtin_functions: HashMap<String, HashSet<usize>>,
    builtin_function_signatures: HashMap<String, HashMap<usize, CallableSignature>>,
    builtin_types: HashSet<String>,
    builtin_consts: HashSet<String>,
    pub(super) current_type: Option<CurrentTypeInfo>,
    pub(super) current_method: Option<(String, usize)>,
    pub(super) in_method: bool,
}

/// Type metadata recorded in the context.
#[derive(Clone)]
struct TypeInfo {
    param_count: usize,
    parent: Option<String>,
    attrs: HashSet<String>,
    attr_types: HashMap<String, SimpleType>,
    methods: HashMap<String, HashMap<usize, CallableSignature>>,
}

/// Protocol metadata recorded in the context.
#[derive(Clone)]
struct ProtocolInfo {
    extends: Option<String>,
    methods: HashMap<String, HashMap<usize, CallableSignature>>,
}

/// Information about the currently checked type (attributes and methods).
#[derive(Clone)]
pub(super) struct CurrentTypeInfo {
    pub(super) parent: Option<String>,
    pub(super) attrs: HashSet<String>,
    pub(super) attr_types: HashMap<String, SimpleType>,
    pub(super) methods: HashMap<String, HashMap<usize, CallableSignature>>,
}

impl Context {
    /// Create a new context populated with builtin functions/types/constants.
    pub(super) fn new() -> Self {
        Self {
            var_scopes: vec![HashSet::new()],
            var_types: vec![HashMap::new()],
            functions: HashMap::new(),
            function_signatures: HashMap::new(),
            types: HashMap::new(),
            protocols: HashMap::new(),
            builtin_functions: builtin_functions(),
            builtin_function_signatures: builtin_function_signatures(),
            builtin_types: builtin_types(),
            builtin_consts: builtin_consts(),
            current_type: None,
            current_method: None,
            in_method: false,
        }
    }

    /// Push a new variable scope.
    pub(super) fn push_scope(&mut self) {
        self.var_scopes.push(HashSet::new());
        self.var_types.push(HashMap::new());
    }

    /// Pop the current variable scope.
    pub(super) fn pop_scope(&mut self) {
        self.var_scopes.pop();
        self.var_types.pop();
    }

    /// Whether `type_name` (or any ancestor) declares an attribute named `field`.
    pub(super) fn type_has_attr(&self, type_name: &str, field: &str) -> bool {
        let mut cur = Some(type_name.to_string());
        while let Some(tn) = cur {
            match self.types.get(&tn) {
                Some(info) => {
                    if info.attrs.contains(field) {
                        return true;
                    }
                    cur = info.parent.clone();
                }
                None => return false,
            }
        }
        false
    }

    /// Define a variable in the current scope. Returns false on redefinition.
    pub(super) fn define_var(&mut self, name: &str) -> bool {
        if let Some(scope) = self.var_scopes.last_mut() {
            return scope.insert(name.to_string());
        }
        false
    }

    /// Set an inferred simple type for a variable in the current scope.
    pub(super) fn set_var_type(&mut self, name: &str, ty: SimpleType) {
        if let Some(scope) = self.var_types.last_mut() {
            scope.insert(name.to_string(), ty);
        }
    }

    /// Set an inferred simple type for the nearest scope that defines the variable.
    pub(super) fn set_var_type_in_scope(&mut self, name: &str, ty: SimpleType) -> bool {
        for index in (0..self.var_scopes.len()).rev() {
            if self.var_scopes[index].contains(name) {
                self.var_types[index].insert(name.to_string(), ty);
                return true;
            }
        }
        false
    }

    /// Check if a variable is defined in any active scope.
    pub(super) fn is_var_defined(&self, name: &str) -> bool {
        self.var_scopes.iter().rev().any(|s| s.contains(name))
    }

    /// Get the inferred simple type for a variable if available.
    pub(super) fn var_type(&self, name: &str) -> Option<SimpleType> {
        for scope in self.var_types.iter().rev() {
            if let Some(ty) = scope.get(name) {
                return Some(ty.clone());
            }
        }
        None
    }

    /// Register a function name with its arity.
    pub(super) fn insert_function(&mut self, name: &str, arity: usize) -> bool {
        insert_arity(&mut self.functions, name, arity)
    }

    /// Register the signature of a function.
    pub(super) fn insert_function_signature(
        &mut self,
        name: &str,
        params: Vec<Option<SimpleType>>,
        return_type: Option<SimpleType>,
    ) {
        self.function_signatures.insert(
            name.to_string(),
            CallableSignature { params, return_type },
        );
    }

    /// Register a user-defined type with the number of type parameters.
    pub(super) fn insert_type(
        &mut self,
        name: &str,
        param_count: usize,
        parent: Option<String>,
    ) {
        self.types
            .entry(name.to_string())
            .or_insert(TypeInfo {
                param_count,
                parent,
                attrs: HashSet::new(),
                attr_types: HashMap::new(),
                methods: HashMap::new(),
            });
    }

    /// Get the direct parent of a type if it exists.
    pub(super) fn type_parent(&self, name: &str) -> Option<&str> {
        self.types.get(name)?.parent.as_deref()
    }

    /// Set recorded attributes and methods for a previously registered type.
    pub(super) fn set_type_members(
        &mut self,
        name: &str,
        parent: Option<String>,
        attrs: HashSet<String>,
        attr_types: HashMap<String, SimpleType>,
        methods: HashMap<String, HashMap<usize, CallableSignature>>,
    ) {
        if let Some(t) = self.types.get_mut(name) {
            t.parent = parent;
            t.attrs = attrs;
            t.attr_types = attr_types;
            t.methods = methods;
        }
    }

    /// Get the signature of a method for a given type name and arity.
    pub(super) fn type_method_signature(
        &self,
        type_name: &str,
        method: &str,
        arity: usize,
    ) -> Option<&CallableSignature> {
        let mut current = Some(type_name);

        while let Some(name) = current {
            let type_info = self.types.get(name)?;
            if let Some(signature) = type_info
                .methods
                .get(method)
                .and_then(|by_arity| by_arity.get(&arity))
            {
                return Some(signature);
            }
            current = type_info.parent.as_deref();
        }

        None
    }

    /// Check whether a method with the given name exists on a type or one of its ancestors.
    pub(super) fn type_has_method_name(&self, type_name: &str, method: &str) -> bool {
        let mut current = Some(type_name);
        let mut seen = HashSet::new();

        while let Some(name) = current {
            if !seen.insert(name.to_string()) {
                break;
            }

            let Some(type_info) = self.types.get(name) else {
                return false;
            };

            if type_info.methods.contains_key(method) {
                return true;
            }

            current = type_info.parent.as_deref();
        }

        false
    }

    /// Get the signature of a method for the current type, following inheritance.
    pub(super) fn current_type_method_signature(
        &self,
        method: &str,
        arity: usize,
    ) -> Option<&CallableSignature> {
        let current = self.current_type.as_ref()?;

        if let Some(signature) = current
            .methods
            .get(method)
            .and_then(|by_arity| by_arity.get(&arity))
        {
            return Some(signature);
        }

        current
            .parent
            .as_deref()
            .and_then(|parent| self.type_method_signature(parent, method, arity))
    }

    /// Get the recorded type of an attribute on the current type or one of its ancestors.
    pub(super) fn current_type_attr_type(&self, name: &str) -> Option<&SimpleType> {
        let current = self.current_type.as_ref()?;
        if let Some(ty) = current.attr_types.get(name) {
            return Some(ty);
        }

        current
            .parent
            .as_deref()
            .and_then(|parent| self.type_attr_type(parent, name))
    }

    /// Get the recorded type of an attribute on a concrete type, following inheritance.
    pub(super) fn type_attr_type(&self, type_name: &str, name: &str) -> Option<&SimpleType> {
        let mut current = Some(type_name);

        while let Some(type_name) = current {
            let type_info = self.types.get(type_name)?;
            if let Some(ty) = type_info.attr_types.get(name) {
                return Some(ty);
            }
            current = type_info.parent.as_deref();
        }

        None
    }

    /// Register a protocol name.
    pub(super) fn insert_protocol(&mut self, name: &str, extends: Option<String>) {
        self.protocols.entry(name.to_string()).or_insert(ProtocolInfo {
            extends,
            methods: HashMap::new(),
        });
    }

    /// Get the direct parent protocol if it exists.
    pub(super) fn protocol_parent(&self, name: &str) -> Option<&str> {
        self.protocols.get(name)?.extends.as_deref()
    }

    /// Store the methods and parent of a protocol.
    pub(super) fn set_protocol_members(
        &mut self,
        name: &str,
        extends: Option<String>,
        methods: HashMap<String, HashMap<usize, CallableSignature>>,
    ) {
        if let Some(protocol) = self.protocols.get_mut(name) {
            protocol.extends = extends;
            protocol.methods = methods;
        }
    }

    /// Get a protocol method signature, following protocol inheritance.
    pub(super) fn protocol_method_signature(
        &self,
        protocol_name: &str,
        method: &str,
        arity: usize,
    ) -> Option<&CallableSignature> {
        let mut current = Some(protocol_name);
        let mut seen = HashSet::new();

        while let Some(name) = current {
            if !seen.insert(name.to_string()) {
                break;
            }

            let protocol = self.protocols.get(name)?;
            if let Some(signature) = protocol
                .methods
                .get(method)
                .and_then(|by_arity| by_arity.get(&arity))
            {
                return Some(signature);
            }
            current = protocol.extends.as_deref();
        }

        None
    }

    /// Check whether a function with a given arity exists.
    pub(super) fn has_function(&self, name: &str, arity: usize) -> bool {
        has_arity(&self.functions, name, arity)
    }

    /// Check whether a builtin function with a given arity exists.
    pub(super) fn has_builtin_function(&self, name: &str, arity: usize) -> bool {
        has_arity(&self.builtin_functions, name, arity)
    }

    /// Get the signature of a user-defined function if available.
    pub(super) fn function_signature(&self, name: &str) -> Option<&CallableSignature> {
        self.function_signatures.get(name)
    }

    /// Get the signature of a builtin function for a given arity if available.
    pub(super) fn builtin_function_signature(
        &self,
        name: &str,
        arity: usize,
    ) -> Option<&CallableSignature> {
        self.builtin_function_signatures
            .get(name)
            .and_then(|sigs| sigs.get(&arity))
    }

    /// Check whether any function with the name exists (any arity).
    pub(super) fn has_function_name(&self, name: &str) -> bool {
        self.functions.contains_key(name)
    }

    /// Is the given name a builtin function?
    pub(super) fn is_builtin_function(&self, name: &str) -> bool {
        self.builtin_functions.contains_key(name)
    }

    /// Is the given name a builtin type?
    pub(super) fn is_builtin_type(&self, name: &str) -> bool {
        self.builtin_types.contains(name)
    }

    /// Is the given name a builtin constant?
    pub(super) fn is_builtin_const(&self, name: &str) -> bool {
        self.builtin_consts.contains(name)
    }

    /// Is a protocol defined with this name?
    pub(super) fn is_protocol_defined(&self, name: &str) -> bool {
        self.protocols.contains_key(name)
    }

    /// Is a type or protocol defined with this name?
    pub(super) fn is_type_or_protocol_defined(&self, name: &str) -> bool {
        self.types.contains_key(name) || self.protocols.contains_key(name)
    }

    /// Is this a known type (builtin, user type or protocol)?
    pub(super) fn is_known_type(&self, name: &str) -> bool {
        self.builtin_types.contains(name)
            || self.types.contains_key(name)
            || self.protocols.contains_key(name)
    }

    /// Is this a constructible type (builtin or user-defined)?
    pub(super) fn is_constructible_type(&self, name: &str) -> bool {
        self.builtin_types.contains(name) || self.types.contains_key(name)
    }

    /// Check whether a concrete type inherits from another concrete type.
    pub(super) fn type_is_subtype_of(&self, actual: &str, expected: &str) -> bool {
        if actual == expected || expected == "Object" {
            return true;
        }

        let mut current = Some(actual);
        let mut seen = HashSet::new();

        while let Some(name) = current {
            if !seen.insert(name.to_string()) {
                break;
            }

            let Some(type_info) = self.types.get(name) else {
                return false;
            };

            if type_info.parent.as_deref() == Some(expected) {
                return true;
            }

            current = type_info.parent.as_deref();
        }

        false
    }

    /// Check whether a concrete type implements a protocol.
    pub(super) fn type_implements_protocol(&self, type_name: &str, protocol_name: &str) -> bool {
        let Some(protocol) = self.protocols.get(protocol_name) else {
            return false;
        };

        if let Some(parent) = protocol.extends.as_deref() {
            if !self.type_implements_protocol(type_name, parent) {
                return false;
            }
        }

        for (method_name, by_arity) in &protocol.methods {
            for (arity, expected_signature) in by_arity {
                let Some(actual_signature) = self.type_method_signature(type_name, method_name, *arity) else {
                    return false;
                };
                if !self.callable_signature_compatible(actual_signature, expected_signature) {
                    return false;
                }
            }
        }

        true
    }

    /// Check whether a protocol conforms to another protocol.
    pub(super) fn protocol_conforms_to_protocol(
        &self,
        actual: &str,
        expected: &str,
    ) -> bool {
        if actual == expected {
            return true;
        }

        let Some(actual_info) = self.protocols.get(actual) else {
            return false;
        };

        let mut current = actual_info.extends.as_deref();
        let mut seen = HashSet::new();
        while let Some(name) = current {
            if !seen.insert(name.to_string()) {
                break;
            }
            if name == expected {
                return true;
            }
            current = self.protocols.get(name).and_then(|p| p.extends.as_deref());
        }

        let Some(expected_info) = self.protocols.get(expected) else {
            return false;
        };

        for (method_name, by_arity) in &expected_info.methods {
            for (arity, expected_signature) in by_arity {
                let Some(actual_signature) = self.protocol_method_signature(actual, method_name, *arity) else {
                    return false;
                };
                if !self.callable_signature_compatible(actual_signature, expected_signature) {
                    return false;
                }
            }
        }

        true
    }

    /// Check whether one callable signature is compatible with another.
    pub(super) fn callable_signature_compatible(
        &self,
        actual: &CallableSignature,
        expected: &CallableSignature,
    ) -> bool {
        if actual.params.len() != expected.params.len() {
            return false;
        }

        for (actual_param, expected_param) in actual.params.iter().zip(expected.params.iter()) {
            let (Some(actual_param), Some(expected_param)) = (actual_param, expected_param) else {
                return false;
            };
            if !self.simple_type_conforms_to(expected_param, actual_param) {
                return false;
            }
        }

        match (&actual.return_type, &expected.return_type) {
            (Some(actual_return), Some(expected_return)) => {
                self.simple_type_conforms_to(actual_return, expected_return)
            }
            _ => false,
        }
    }

    /// Check whether one simple type conforms to another.
    pub(super) fn simple_type_conforms_to(&self, actual: &SimpleType, expected: &SimpleType) -> bool {
        if actual == expected {
            return true;
        }

        match (actual, expected) {
            (_, SimpleType::Named(name)) if name == "Object" => true,
            (SimpleType::Named(actual_name), SimpleType::Named(expected_name)) => {
                if self.is_protocol_defined(expected_name) {
                    if self.is_protocol_defined(actual_name) {
                        self.protocol_conforms_to_protocol(actual_name, expected_name)
                    } else {
                        self.type_implements_protocol(actual_name, expected_name)
                    }
                } else if self.is_protocol_defined(actual_name) {
                    false
                } else {
                    self.type_is_subtype_of(actual_name, expected_name)
                }
            }
            (SimpleType::Vector(actual_inner), SimpleType::Vector(expected_inner)) => {
                self.simple_type_conforms_to(actual_inner, expected_inner)
            }
            _ => false,
        }
    }

    /// Return number of type parameters for a type if known.
    pub(super) fn type_param_count(&self, name: &str) -> Option<usize> {
        if self.builtin_types.contains(name) {
            return Some(0);
        }

        let mut current = Some(name);
        let mut seen = HashSet::new();

        while let Some(type_name) = current {
            if !seen.insert(type_name.to_string()) {
                break;
            }

            let type_info = self.types.get(type_name)?;
            if type_info.param_count > 0 {
                return Some(type_info.param_count);
            }

            if let Some(parent) = type_info.parent.as_deref() {
                current = Some(parent);
            } else {
                return Some(0);
            }
        }

        Some(0)
    }
}

/// Builtin functions and allowed arities.
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

/// Builtin function signatures used for type checking.
fn builtin_function_signatures() -> HashMap<String, HashMap<usize, CallableSignature>> {
    let mut map = HashMap::new();

    map.insert(
        "sin".to_string(),
        signature_map(vec![(1, vec![Some(SimpleType::Number)], Some(SimpleType::Number))]),
    );
    map.insert(
        "cos".to_string(),
        signature_map(vec![(1, vec![Some(SimpleType::Number)], Some(SimpleType::Number))]),
    );
    map.insert(
        "sqrt".to_string(),
        signature_map(vec![(1, vec![Some(SimpleType::Number)], Some(SimpleType::Number))]),
    );
    map.insert(
        "exp".to_string(),
        signature_map(vec![(1, vec![Some(SimpleType::Number)], Some(SimpleType::Number))]),
    );
    map.insert(
        "log".to_string(),
        signature_map(vec![
            (1, vec![Some(SimpleType::Number)], Some(SimpleType::Number)),
            (
                2,
                vec![Some(SimpleType::Number), Some(SimpleType::Number)],
                Some(SimpleType::Number),
            ),
        ]),
    );
    map.insert(
        "rand".to_string(),
        signature_map(vec![(0, vec![], Some(SimpleType::Number))]),
    );
    map.insert(
        "print".to_string(),
        signature_map(vec![(1, vec![None], None)]),
    );
    map.insert(
        "range".to_string(),
        signature_map(vec![(
            2,
            vec![Some(SimpleType::Number), Some(SimpleType::Number)],
            Some(SimpleType::Vector(Box::new(SimpleType::Number))),
        )]),
    );

    map
}

/// Builtin type names.
fn builtin_types() -> HashSet<String> {
    ["Number", "String", "Boolean", "Object"]
        .iter()
        .map(|s| s.to_string())
        .collect()
}

/// Builtin constant names.
fn builtin_consts() -> HashSet<String> {
    ["PI", "E", "()"].iter().map(|s| s.to_string()).collect()
}

/// Create a HashSet of arities from an array.
fn arity_set(values: &[usize]) -> HashSet<usize> {
    values.iter().copied().collect()
}

/// Build a builtin signature map keyed by arity.
fn signature_map(
    values: Vec<(usize, Vec<Option<SimpleType>>, Option<SimpleType>)>,
) -> HashMap<usize, CallableSignature> {
    values
        .into_iter()
        .map(|(arity, params, return_type)| {
            (arity, CallableSignature { params, return_type })
        })
        .collect()
}

/// Insert an arity into the map for a given name.
fn insert_arity(map: &mut HashMap<String, HashSet<usize>>, name: &str, arity: usize) -> bool {
    let entry = map.entry(name.to_string()).or_insert_with(HashSet::new);
    entry.insert(arity)
}

/// Check if a name has a given arity in the map.
fn has_arity(map: &HashMap<String, HashSet<usize>>, name: &str, arity: usize) -> bool {
    map.get(name).map_or(false, |set| set.contains(&arity))
}