use super::checker::{check_program, SemanticError};
use crate::lexer::lexer::TokenStream;
use crate::parser::Parser;

fn semantic_errors(source: &str) -> Vec<SemanticError> {
    let stream = TokenStream::new(source);
    let mut parser = Parser::new(stream);
    let program = parser
        .parse_program()
        .expect("the test source should parse successfully");
    check_program(&program)
}

fn assert_has_error(errors: &[SemanticError], expected_fragment: &str) {
    assert!(
        errors
            .iter()
            .any(|error| error.message.contains(expected_fragment)),
        "expected an error containing '{}', got: {:?}",
        expected_fragment,
        errors
    );
}

#[test]
fn equality_rejects_number_and_string() {
    let errors = semantic_errors(r#"
        1 == "hola";
    "#);

    assert!(
        errors.iter().any(|error| error.message.contains("equality operator")),
        "expected an equality-type mismatch error, got: {:?}",
        errors
    );
}

#[test]
fn equality_rejects_boolean_and_number() {
    let errors = semantic_errors(r#"
        true == 42;
    "#);

    assert!(
        errors.iter().any(|error| error.message.contains("equality operator")),
        "expected an equality-type mismatch error, got: {:?}",
        errors
    );
}

#[test]
fn relational_rejects_string_and_number() {
    let errors = semantic_errors(r#"
        "hola" > 2;
    "#);

    assert!(
        errors.iter().any(|error| error.message.contains("relational operator requires Number")),
        "expected a relational-type mismatch error, got: {:?}",
        errors
    );
}

#[test]
fn relational_rejects_boolean_and_number() {
    let errors = semantic_errors(r#"
        false <= 10;
    "#);

    assert!(
        errors.iter().any(|error| error.message.contains("relational operator requires Number")),
        "expected a relational-type mismatch error, got: {:?}",
        errors
    );
}

#[test]
fn equality_rejects_mixed_types_inside_let_binding() {
    let errors = semantic_errors(r#"
        let left: String = "hola", right: Number = 42 in left != right;
    "#);

    assert!(
        errors.iter().any(|error| error.message.contains("equality operator")),
        "expected an equality-type mismatch error, got: {:?}",
        errors
    );
}

#[test]
fn relational_rejects_mixed_types_inside_let_binding() {
    let errors = semantic_errors(r#"
        let left: Boolean = true, right: Number = 2 in left >= right;
    "#);

    assert!(
        errors.iter().any(|error| error.message.contains("relational operator requires Number")),
        "expected a relational-type mismatch error, got: {:?}",
        errors
    );
}

#[test]
fn reports_call_to_nonexistent_function() {
    let errors = semantic_errors(r#"
        desconocida(1, 2);
    "#);

    assert_has_error(&errors, "function 'desconocida' not defined");
}

#[test]
fn rejects_call_syntax_for_constructible_type_names() {
    let errors = semantic_errors(r#"
        type Person(name, age) {
            name: String = name;
            age: Number = age;
        }

        let people = [Person("Ana", 20), Person("Luis", 25)] in
            people;
    "#);

    assert_has_error(&errors, "must be instantiated with 'new'");
}

#[test]
fn uppercase_call_reports_missing_type_instead_of_missing_function() {
    let errors = semantic_errors(r#"
        Fantasma(1);
    "#);

    assert_has_error(&errors, "type 'Fantasma' not defined");
}

#[test]
fn uppercase_call_reports_type_arity_errors() {
    let errors = semantic_errors(r#"
        type Person(name, age) {
            name: String = name;
            age: Number = age;
        }

        new Person("Ana");
    "#);

    assert_has_error(&errors, "type 'Person' requires 2 arguments");
}

#[test]
fn reports_invalid_arity_for_user_function() {
    let errors = semantic_errors(r#"
        function suma(a, b) => a + b;
        suma(1);
    "#);

    assert_has_error(&errors, "call to 'suma' with invalid arity");
}

#[test]
fn reports_invalid_arity_for_builtin_function() {
    let errors = semantic_errors(r#"
        sin();
    "#);

    assert_has_error(&errors, "call to 'sin' with invalid arity");
}

#[test]
fn reports_invalid_argument_types_for_user_function() {
    let errors = semantic_errors(r#"
        function mezclar(texto: String, cantidad: Number) => texto;
        mezclar(10, "hola");
    "#);

    assert_has_error(&errors, "call to 'mezclar' argument 1 expects String, found Number");
}

#[test]
fn infers_iterable_parameter_type_from_for_loop_and_rejects_number_argument() {
    let errors = semantic_errors(r#"
        function sum_vec(v): Number {
            let total = 0 in {
                for (i in v) {
                    total := total + i;
                };
                total
            }
        }
        sum_vec(1);
    "#);

    assert_has_error(&errors, "call to 'sum_vec' argument 1 expects Vector");
}

#[test]
fn infers_vector_number_argument_from_for_loop_body() {
    let errors = semantic_errors(r#"
        function sum_vec(v): Number {
            let total = 0 in {
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
            }
        }
        sum_vec([1, 2]);
    "#);

    assert!(errors.is_empty(), "expected no semantic errors, got: {:?}", errors);
}

#[test]
fn reports_all_invalid_argument_types_for_user_function() {
    let errors = semantic_errors(r#"
        function nested(a: Number, b: String) : Number {
            let sum = 0 in {
                for (i in a) {
                    for (j in i) {
                        if (j % 2 == 0) { sum := sum + j } else { sum := sum + 0 };
                    };
                };
                sum
            }
        }
        nested(true, true)
    "#);

    assert_has_error(&errors, "call to 'nested' argument 1 expects Number, found Boolean");
    assert_has_error(&errors, "call to 'nested' argument 2 expects String, found Boolean");
}

#[test]
fn reports_invalid_argument_types_for_method_call_on_self() {
    let errors = semantic_errors(r#"
        type A {
            m(texto: String, cantidad: Number) {
                0
            }

            n() {
                self.m(10, "hola");
                0
            }
        }
        0;
    "#);

    assert_has_error(&errors, "method 'm' argument 1 expects String, found Number");
}

#[test]
fn reports_invalid_argument_types_for_builtin_function() {
    let errors = semantic_errors(r#"
        sin("hola");
    "#);

    assert_has_error(&errors, "call to 'sin' argument 1 expects Number, found String");
}

#[test]
fn while_requires_boolean_condition_for_number() {
    let errors = semantic_errors(r#"
        while (1) 0;
    "#);

    assert_has_error(&errors, "while condition must be Boolean");
}

#[test]
fn while_requires_boolean_condition_for_string() {
    let errors = semantic_errors(r#"
        while ("hola") 0;
    "#);

    assert_has_error(&errors, "while condition must be Boolean");
}

#[test]
fn while_with_assignment_to_undefined_variable_reports_error() {
    let errors = semantic_errors(r#"
        while (true) {
            x := 1;
            0
        };
    "#);

    assert_has_error(&errors, "assignment to undefined variable 'x'");
}

#[test]
fn nested_while_reports_nonexistent_function_inside_body() {
    let errors = semantic_errors(r#"
        while (true) {
            while (true) inexistente(10);
            0
        };
    "#);

    assert_has_error(&errors, "function 'inexistente' not defined");
}

#[test]
fn type_inheritance_reports_undefined_parent_type() {
    let errors = semantic_errors(r#"
        type Hijo inherits Fantasma {
            valor = 1;
        }
        0;
    "#);

    assert_has_error(&errors, "parent type 'Fantasma' not defined");
}

#[test]
fn type_inheritance_reports_wrong_parent_arity() {
    let errors = semantic_errors(r#"
        type Padre(a, b) {
            valor = 1;
        }

        type Hijo inherits Padre(1) {
            otro = 2;
        }
        0;
    "#);

    assert_has_error(&errors, "parent type 'Padre' requires 2 arguments");
}

#[test]
fn type_inheritance_inherits_parent_constructor_arity_by_default() {
    let errors = semantic_errors(r#"
        type Person(name, age) {
            name = name;
            age = age;
        }

        type Knight inherits Person {
            title = "Sir";
        }

        let p = new Knight("Phil", "Collins") in p;
    "#);

    assert!(errors.is_empty(), "expected no errors, got: {:?}", errors);
}

#[test]
fn type_inheritance_reports_cyclic_inheritance() {
    let errors = semantic_errors(r#"
        type A inherits B {
            value = 1;
        }

        type B inherits A {
            value = 2;
        }

        0;
    "#);

    assert_has_error(&errors, "type 'A' has cyclic inheritance");
    assert_has_error(&errors, "type 'B' has cyclic inheritance");
}

#[test]
fn function_parameter_reports_undefined_type_annotation() {
    let errors = semantic_errors(r#"
        function f(x: TipoNoDefinido) => x;
        0;
    "#);

    assert_has_error(&errors, "type 'TipoNoDefinido' not defined");
}

#[test]
fn let_binding_reports_undefined_type_annotation() {
    let errors = semantic_errors(r#"
        let x: TipoFantasma = 1 in x;
    "#);

    assert_has_error(&errors, "type 'TipoFantasma' not defined");
}

#[test]
fn let_binding_reports_type_mismatch_for_initializer() {
    let errors = semantic_errors(r#"
        let b = 4 * 2 in
            let a: Boolean = b + 4 in {
                print(a);
            };
    "#);

    assert_has_error(&errors, "let binding 'a' expects Boolean, found Number");
}

#[test]
fn let_binding_reports_inconsistent_function_return_note() {
    let errors = semantic_errors(r#"
        function g(a): Boolean => a + 5;

        let b = 4 * 2 in
            let a: String = g(5) in {
                print(a);
            };
    "#);

    assert_has_error(
        &errors,
        "function 'g' has an inconsistent return type: it declares Boolean, but its body returns Number",
    );
    assert_has_error(&errors, "let binding 'a' expected a String, but found a value of another type; note: function 'g' has an inconsistent return type: it declares Boolean, but its body returns Number");
}

#[test]
fn arithmetic_operator_reports_source_binding_inconsistency_note() {
    let errors = semantic_errors(r#"
        function g(a): Number => a + 5;

        let b: String = 4 * 2 in
            let a: Number = g(5) + b in {
                print(a);
            };
    "#);

    assert_has_error(&errors, "let binding 'b' expects String, found Number");
    assert_has_error(
        &errors,
        "arithmetic operator requires Number (right side: String); note: let binding 'b' expects String, found Number",
    );
}

#[test]
fn protocol_extends_reports_undefined_parent_protocol() {
    let errors = semantic_errors(r#"
        protocol P extends Q {
            m(x: Number): Number;
        }
        0;
    "#);

    assert_has_error(&errors, "parent protocol 'Q' not defined");
}

#[test]
fn protocol_extends_reports_error_when_parent_is_a_type() {
    let errors = semantic_errors(r#"
        type Base {}

        protocol P extends Base {
            m(x: Number): Number;
        }
        0;
    "#);

    assert_has_error(&errors, "parent type 'Base' cannot be extended by a protocol");
}

#[test]
fn protocol_extends_reports_cyclic_inheritance() {
    let errors = semantic_errors(r#"
        protocol A extends B {
            m(): Number;
        }

        protocol B extends A {
            n(): Number;
        }

        0;
    "#);

    assert_has_error(&errors, "protocol 'A' has cyclic inheritance");
    assert_has_error(&errors, "protocol 'B' has cyclic inheritance");
}

#[test]
fn type_inherits_reports_error_when_parent_is_a_protocol() {
    let errors = semantic_errors(r#"
        protocol Greetable {
            greet(): String;
        }

        type Person(name) inherits Greetable {
            name: String = name;

            greet(): String => "Hello, I am " @ self.name;
        }

        0;
    "#);

    assert_has_error(&errors, "type 'Person' cannot inherit from protocol 'Greetable'");
}

#[test]
fn protocol_method_signature_reports_undefined_return_type() {
    let errors = semantic_errors(r#"
        protocol Serializable {
            serialize(x: Number): TipoRaro;
        }
        0;
    "#);

    assert_has_error(&errors, "type 'TipoRaro' not defined");
}

#[test]
fn protocol_is_implemented_implicitly_by_matching_methods() {
    let errors = semantic_errors(r#"
        protocol Printable {
            printSelf(): String;
        }

        type Box {
            value = 10;
            printSelf() => "Box(" @ self.value @ ")";
        }

        let p: Printable = new Box() in print(p.printSelf());
    "#);

    assert!(errors.is_empty(), "expected no semantic errors, got: {:?}", errors);
}

#[test]
fn protocol_implementation_reports_assignment_to_self_inside_method() {
    let errors = semantic_errors(r#"
        protocol Greetable {
            greet(): String;
        }

        type Person(name) {
            name: String = name;

            reset() => self := new Person("Hilko");
            greet(): String => "Hello, I am " @ self.name;
        }

        let p: Greetable = new Person("Alice") in print(p.greet());
    "#);

    assert_has_error(&errors, "cannot assign to 'self'");
}

#[test]
fn protocol_accepts_covariant_and_contravariant_overrides() {
    let errors = semantic_errors(r#"
        type Animal {}
        type Dog inherits Animal {}

        protocol Maker {
            make(input: Dog): Animal;
        }

        type AnyMaker {
            make(input: Object) => new Dog();
        }

        let maker: Maker = new AnyMaker() in maker.make(new Dog());
    "#);

    assert!(errors.is_empty(), "expected no semantic errors, got: {:?}", errors);
}

#[test]
fn protocol_let_binding_reports_why_concrete_type_does_not_match() {
    let errors = semantic_errors(r#"
        protocol Printable {
            printSelf(): String;
        }

        type Box {
            value = 10;
        }

        let p: Printable = new Box() in print(p.printSelf());
    "#);

    assert_has_error(
        &errors,
        "let binding 'p' expects Printable, found Box; Box does not satisfy the requirements of Printable",
    );
}

#[test]
fn protocol_methods_must_be_fully_typed() {
    let errors = semantic_errors(r#"
        protocol Broken {
            hash(): Number;
            equals(other): Boolean;
        }
        0;
    "#);

    assert_has_error(&errors, "protocol method 'equals' parameter 'other' must be typed");
}

#[test]
fn duplicate_function_parameters_report_error() {
    let errors = semantic_errors(r#"
        function repetir(x, x) => x;
        0;
    "#);

    assert_has_error(&errors, "duplicate parameter 'x'");
}

#[test]
fn self_outside_method_reports_error() {
    let errors = semantic_errors(r#"
        self;
    "#);

    assert_has_error(&errors, "use of self outside of a method");
}

#[test]
fn base_outside_method_reports_error() {
    let errors = semantic_errors(r#"
        base(1);
    "#);

    assert_has_error(&errors, "use of base outside of a method");
}

#[test]
fn method_call_on_self_reports_missing_method() {
    let errors = semantic_errors(r#"
        type A {
            m() {
                self.no_existe();
                0
            }
        }
        0;
    "#);

    assert_has_error(&errors, "method 'no_existe' with arity 0 not defined on current type");
}

#[test]
fn field_access_on_self_reports_missing_attribute() {
    let errors = semantic_errors(r#"
        type A {
            m() {
                self.no_existe;
                0
            }
        }
        0;
    "#);

    assert_has_error(&errors, "attribute 'no_existe' not defined on current type");
}

#[test]
fn reports_invalid_argument_types_for_method_call_on_variable() {
    let errors = semantic_errors(r#"
        type A {
            m(texto: String, cantidad: Number) {
                0
            }
        }

        let a = new A() in {
            a.m(10, "hola");
            0
        };
    "#);

    assert_has_error(&errors, "method 'm' argument 1 expects String, found Number");
}

#[test]
fn method_call_on_for_loop_variable_uses_iterable_element_type() {
    let errors = semantic_errors(r#"
        type Person(name, age) {
            name: String = name;
            age: Number = age;

            greet() => print("Hola, soy " @ self.name @ " y tengo " @ self.age @ " años");
        }

        {
            let people = [Person("Ana", 20), Person("Luis", 25)] in {
                for (p in people) {
                    p.greetol();
                }
            }
        }
    "#);

    assert_has_error(&errors, "method 'greetol' with arity 0 not defined on type 'Person'");
}

#[test]
fn inherited_method_is_found_on_subtype_instances() {
    let errors = semantic_errors(r#"
        type A {
            c = 0;

            get_c() => self.c;
        }

        type Person(name, age) inherits A {
            name: String = name;
            age: Number = age;
        }

        let jery = new Person("Jery", 21) in
            print(jery.get_c());
    "#);

    assert!(errors.is_empty(), "expected no errors, got: {:?}", errors);
}

#[test]
fn inherited_method_override_with_same_signature_is_allowed() {
    let errors = semantic_errors(r#"
        type A {
            m(x: Number): Number => x + 1;
        }

        type B inherits A {
            m(x: Number): Number => x + 2;
        }

        print(0);
    "#);

    assert!(errors.is_empty(), "expected no errors, got: {:?}", errors);
}

#[test]
fn inherited_method_override_with_different_signature_reports_error() {
    let errors = semantic_errors(r#"
        type A {
            m(x: Number): Number => x + 1;
        }

        type B inherits A {
            m(x: String): String => x;
        }

        print(0);
    "#);

    assert_has_error(&errors, "must match inherited signature");
}

#[test]
fn inherited_transitive_method_is_found_on_subtype_instances() {
    let errors = semantic_errors(r#"
        type B {
            d = 0;

            get_d() => self.d;
        }
        type A inherits B {
            c = 0;

            get_c() => self.c;
        }
        type Person(name, age) inherits A {
            name: String = name;
            age: Number = age;

            greet() => print("Hola, soy " @ self.name @ " y tengo " @ self.age @ " años");
            get_age() => self.age;
        }

        {
            let people = [new Person("Ana", 20), new Person("Luis", 25),  new Person("Jery", 22)], x = 0 in {
                for (p in people) {
                    p.greet();
                };
                let jery = new Person("Jery", 21) in
                    print(jery.get_d());
            }
        }
    "#);

    assert!(errors.is_empty(), "expected no errors, got: {:?}", errors);
}

#[test]
fn inferred_method_return_type_is_used_in_later_method_calls() {
    let errors = semantic_errors(r#"
        type A {
            value() => 1;

            twice() {
                self.value() + self.value()
            }
        }

        0;
    "#);
    assert!(errors.is_empty(), "expected no errors, got: {:?}", errors);
}

#[test]
fn method_return_type_mismatch_is_reported() {
    let errors = semantic_errors(r#"
        type A {
            m(): Number => "hola";
        }
        0;
    "#);

    assert_has_error(&errors, "method 'm' return type expects Number, found String");
}

#[test]
fn attribute_type_mismatch_is_reported() {
    let errors = semantic_errors(r#"
        type A {
        }

        type Person(name: String, age: Number) inherits A {
            name: String = age;
            age: Number = age;
        }

        0;
    "#);

    assert_has_error(&errors, "attribute 'name' expects String, found Number");
}

#[test]
fn factorial_example_return_type_mismatch_is_reported() {
    let errors = semantic_errors(r#"
        function factorial(n: Number, j: String): String {
            let result = 1, i = 1 in {
                while (i <= n) {
                    result := result * i;
                    i := i + 1;
                };
                result
            }
        }
        0;
    "#);

    assert_has_error(&errors, "function 'factorial' return type expects String, found Number");
}

#[test]
fn logical_and_requires_boolean_operands_number_left() {
    let errors = semantic_errors(r#"
        true & 1;
    "#);

    assert_has_error(&errors, "logical operator requires Boolean");
}

#[test]
fn logical_or_requires_boolean_operands_number_left() {
    let errors = semantic_errors(r#"
            1 | false;
        "#);

        assert_has_error(&errors, "logical operator requires Boolean");
}

#[test]
fn function_call_in_and_reports_nonboolean() {
    let errors = semantic_errors(r#"
            function factorial(n: Number, j: String): Number => n;

            if (factorial(1, "x") & true) { 0 } else { 0 };
        "#);

        assert_has_error(&errors, "logical operator requires Boolean");
}

#[test]
fn vector_comprehension_reports_multiple_semantic_errors() {
    let errors = semantic_errors(r#"
        let source: Number = 1 in {
            let evens = [ x * true | x in source ] in {
                print(evens);
                print(missing);
            };
        };
    "#);

    assert_has_error(&errors, "arithmetic operator requires Number (right side: Boolean)");
    assert_has_error(&errors, "identifier 'missing' not defined");
}

#[test]
fn sum_until_reports_multiple_semantic_errors() {
    let errors = semantic_errors(r#"
        function sum_until(max: Number): Number {
            let result = 0, i = "0" in (
                while (i < max) {
                    result := result + true;
                    i := i + 1;
                };
                result
            )
        }
        sum_until("10");
    "#);

    assert_has_error(&errors, "call to 'sum_until' argument 1 expects Number, found String");
    assert_has_error(&errors, "relational operator requires Number (left side: String)");
    assert_has_error(&errors, "arithmetic operator requires Number (right side: Boolean)");
    assert_has_error(&errors, "arithmetic operator requires Number (left side: String)");
}

#[test]
fn sum_vec_reports_multiple_semantic_errors() {
    let errors = semantic_errors(r#"
        function sum_vec(v): Number {
            let total = 0 in {
                for (i in v) {
                    total := total + i;
                };
                total + "x"
            }
        }
        sum_vec(1);
    "#);

    assert_has_error(&errors, "call to 'sum_vec' argument 1 expects Vector");
    assert_has_error(&errors, "arithmetic operator requires Number (right side: String)");
}

#[test]
fn sum_vec_report_semantic_errors_vector() {
    let errors = semantic_errors(r#"
        function sum_vec(v): Number {
            let total = 0 in {
                for (i in v) {
                    total := total + i;
                };
                total + "x"
            }
        }
        sum_vec(["text", "texto"]);
    "#);

    assert_has_error(&errors, "call to 'sum_vec' argument 1 expects Vector<Number>, found Vector<String>");
}

#[test]
fn sum_vec_rejects_vector_of_booleans_when_numbers_are_expected() {
    let errors = semantic_errors(r#"
        function sum_vec(v): Number {
            let total = 0 in {
                for (i in v) {
                    if (i == true) {
                        print("Found a true value, adding 1 to total");
                        total := total + 1;
                    } else {
                        total := total + 0;
                    };
                };
                total
            };
        }
        print(sum_vec(["te", "f"]));
    "#);

    assert_has_error(
        &errors,
        "call to 'sum_vec' argument 1 expects Vector<Boolean>, found Vector<String>",
    );
}

#[test]
fn sum_vec_rejects_vector_of_numbers_when_strings_are_expected() {
    let errors = semantic_errors(r#"
        function sum_vec(v): Number {
            let total = 0 in {
                for (i in v) {
                    if (i == "texto") {
                        total := total + 5;
                    } elif (i == "hola") {
                        total := total + 0;
                    } else {
                        total := total + 10;
                    };
                };
                total
            };
        }
        print(sum_vec([1, 2, 1, 3]));
    "#);

    assert_has_error(
        &errors,
        "call to 'sum_vec' argument 1 expects Vector<String>, found Vector<Number>",
    );
}

#[test]
fn vector_literals_reject_mixed_element_types() {
    let errors = semantic_errors(r#"
        function sum_vec(v): Number {
            let total = 0 in {
                for (i in v) {
                    if (i == "texto") {
                        total := total + 5;
                    } elif (i == "hola") {
                        total := total + 0;
                    } else {
                        total := total + 10;
                    };
                };
                total
            };
        }
        print(sum_vec([1, 2, "texto", 3]));
    "#);

    assert_has_error(
        &errors,
        "vector with elements of different types (expected Number, found String)",
    );
}

#[test]
fn factorial_reports_multiple_semantic_errors() {
    let errors = semantic_errors(r#"
        function factorial(n: Number, j: String): Number {
            let result = 1, i = 1 in {
                while (i <= n) {
                    i := i + true;
                };
                result := result + "x";
                result
            }
        }
        if (factorial(1, 2) > 2 & 1) {
            print("Factorial of 1 is 1");
        } else {
            print("Error in factorial function");
        };
    "#);

    assert_has_error(&errors, "call to 'factorial' argument 2 expects String, found Number");
    assert_has_error(&errors, "arithmetic operator requires Number (right side: String)");
    assert_has_error(&errors, "arithmetic operator requires Number (right side: Boolean)");
    assert_has_error(&errors, "logical operator requires Boolean (right side: Number)");
}

#[test]
fn nested_condition_reports_multiple_semantic_errors() {
    let errors = semantic_errors(r#"
        let x: Number = 2, y: Number = 4 in (
            let b: String = "text", h: Boolean = true in
            if (false & 1) {
                1
            } elif (true | x) {
                2
            } elif (y > "10") {
                3
            } else {
                4
            };
        )
    "#);

    assert_has_error(&errors, "logical operator requires Boolean (right side: Number)");
    assert_has_error(&errors, "logical operator requires Boolean (right side: Number)");
    assert_has_error(&errors, "relational operator requires Number (right side: String)");
}

#[test]
fn assignment_and_vector_literal_report_multiple_semantic_errors() {
    let errors = semantic_errors(r#"
        {
            let a = 10, c = 0 in {
                let b = "20" in {
                    a := a + b;
                    1 := 2;
                    a
                }
            };

            let v = [1, 2, (2 + 4), 3, "4"] in {
                print(v);
                print(ghost);
            };
        }
    "#);

    assert_has_error(&errors, "arithmetic operator requires Number (right side: String)");
    assert_has_error(&errors, "vector with elements of different types (expected Number, found String)");
    assert_has_error(&errors, "identifier 'ghost' not defined");
}

#[test]
fn type_and_vector_report_multiple_semantic_errors() {
    let errors = semantic_errors(r#"
        type Person(name, age) {
            name: String = name;
            age: Number = age;

            greet() => print("Hola, soy " @ self.name @ " y tengo " @ self.age @ " años");
            get_age() => self.age;
        }

        {
            let people = [new Person("Ana", 20), Person("Luis", 25), Person("Jery", 22)] in {
                for (p in people) {
                    p.greetol();
                }
                let jery = new Person("Jery", 21) in
                    print(jery.get_age());
                print(jery)
            }
        }
    "#);

    assert_has_error(&errors, "type 'Person' must be instantiated with 'new'");
    assert_has_error(&errors, "type 'Person' must be instantiated with 'new'");
    assert_has_error(&errors, "method 'greetol' with arity 0 not defined on type 'Person'");
    assert_has_error(&errors, "identifier 'jery' not defined");
}

#[test]
fn self_valid_attribute_reference() {
    let errors = semantic_errors(r#"
    type Counter(n) {
        n = n;
        get() => self.n;
    }
    new Counter(0).get()
    "#);

    assert!(errors.is_empty(), "expected no semantic errors, got: {:?}", errors);
}

#[test]
fn self_valid_method_call() {
    let errors = semantic_errors(r#"
    type Counter(n) {
        n = n;
        inc() => self.n + 1;
        double() => self.inc() * 2;
    }
    new Counter(3).double()
    "#);

    assert!(errors.is_empty(), "expected no semantic errors, got: {:?}", errors);
}

#[test]
fn self_valid_as_argument_of_function() {
    let errors = semantic_errors(r#"
    function getId(c) => 42;
    type Counter(n) {
        n = n;
        id() => getId(self);
    }
    new Counter(0).id()
    "#);

    assert!(errors.is_empty(), "expected no semantic errors, got: {:?}", errors);
}

#[test]
fn self_shadowed_by_parameter_of_method() {
    let errors = semantic_errors(r#"
    type Counter(n) {
        n = n;
        add(self) => self + 1;
    }
    new Counter(0).add(5)
    "#);

    assert!(errors.is_empty(), "expected no semantic errors, got: {:?}", errors);
}

#[test]
fn self_shadowed_by_let_inside_method() {
    let errors = semantic_errors(r#"
    type Counter(n) {
        n = n;
        compute() => let self = 42 in self * 2;
    }
    new Counter(0).compute()
    "#);

    assert!(errors.is_empty(), "expected no semantic errors, got: {:?}", errors);
}

#[test]
fn base_valid_calls_parent_method() {
    let errors = semantic_errors(r#"
    type Animal {
        name() => "Animal";
    }
    type Dog inherits Animal {
        name() => base() @ " Dog";
    }
    new Dog().name()
    "#);

    assert!(errors.is_empty(), "expected no semantic errors, got: {:?}", errors);
}

#[test]
fn base_uses_parent_method_not_parent_constructor_arity() {
    let errors = semantic_errors(r#"
    type Person(firstname, lastname) {
        firstname = firstname;
        lastname = lastname;

        name() => self.firstname @@ self.lastname;
    }

    type Knight inherits Person {
        name() => "Sir" @@ base();
    }

    let p = new Knight("Phil", "Collins") in p.name();
    "#);

    assert!(errors.is_empty(), "expected no semantic errors, got: {:?}", errors);
}

#[test]
fn base_valid_in_overridden_method_with_parameters() {
    let errors = semantic_errors(r#"
    type Person(firstname, lastname) {
        firstname = firstname;
        lastname = lastname;

        name(a: String, b: Number): String => self.firstname @@ self.lastname;
    }

    type Knight inherits Person {
        name(a: String, b: Number): String => "Sir" @@ base();
    }

    let p = new Knight("Phil", "Collins") in p.name("x", 1);
    "#);

    assert!(errors.is_empty(), "expected no semantic errors, got: {:?}", errors);
}

#[test]
fn base_valid_in_overridden_method_with_diff_parameters() {
    let errors = semantic_errors(r#"
    type Person(firstname, lastname) {
        firstname = firstname;
        lastname = lastname;

        name(a: String, b: Number): String => self.firstname @@ self.lastname;
    }

    type Knight inherits Person {
        name(a: String, b: Boolean): String => "Sir" @@ base();
    }

    let p = new Knight("Phil", "Collins") in p.name("x", 1);
    "#);

    assert_has_error(&errors, "method 'name' in type 'Knight' must match inherited signature from 'Person'");
    assert_has_error(&errors, "method 'name' argument 2 expects Boolean, found Number");
}

#[test]
fn protocols_and_colored_shapes_example() {
    let errors = semantic_errors(r#"
        protocol Shape {
            area() : Number;
            perimeter() : Number;
            describe() : String;
        }

        protocol ColoredShape extends Shape {
            color() : String;
        }

        type Rectangle(x, y) {
            width: Number = x;
            height: Number = y;

            area(): Number => self.width * self.height;
            perimeter(): Number => 2 * (self.width + self.height);
            describe(): String => "Rectángulo de " @ self.width @ " x " @ self.height;
        }

        type Square(side) {
            side: Number = side;

            area(): Number => self.side * self.side;
            perimeter(): Number => 4 * self.side;
            describe(): String => "Cuadrado de lado " @ self.side;
        }

        type Rhombus(side, d1, d2) {
            side: Number = side;
            d1: Number = d1;
            d2: Number = d2;

            area(): Number => (self.d1 * self.d2) / 2;
            perimeter(): Number => 4 * self.side;
            describe(): String => "Rombo de lado " @ self.side @ " y diagonales " @ self.d1 @ " y " @ self.d2;
        }

        type ColoredRectangle(width, height, c) {
            width: Number = width;
            height: Number = height;
            c: String = c;

            area(): Number => self.width * self.height;
            perimeter(): Number => 2 * (self.width + self.height);
            describe(): String => "Rectángulo de color " @ self.c;
            color(): String => self.c;
        }

        {
            let s1 : Shape = new Rectangle(3, 4) in {
                print(s1.describe() @ " | área = " @ s1.area() @ " | perímetro = " @ s1.perimeter());
            };

            let s2 : Shape = new Square(5) in {
                print(s2.describe() @ " | área = " @ s2.area() @ " | perímetro = " @ s2.perimeter());
            };

            let s3 : Shape = new Rhombus(4, 6, 8) in {
                print(s3.describe() @ " | área = " @ s3.area() @ " | perímetro = " @ s3.perimeter());
            };

            let cs : ColoredShape = new ColoredRectangle(2, 7, "azul") in {
                print(cs.describe() @ " | color = " @ cs.color());
            };
        }
    "#);

    assert!(errors.is_empty(), "expected no semantic errors, got: {:?}", errors);
}

