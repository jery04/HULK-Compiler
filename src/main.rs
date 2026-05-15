#![allow(dead_code)]

mod lexer;          // lexer module
mod parser;         // parser module
mod evaluator;      // evaluator module
mod struct_printer; // structure printer module
use struct_printer::test_program; // import test_program directly


fn main() {
    
    test_program(r#"
        type A {
            value: Number = 10;
            getValue() => self.value;
            inc() => {
                self.value := self.value + 1;
                self.value
            };
        }

        type Person(name: String, age: Number) inherits A {
            name: String = name;
            age: Number = age;

            getName() => self.name;

            birthday() => {
                self.age := self.age + 1;
                self.age
            };

            isAdult() => self.age >= 18;
        }

        function greet(p: Person) => {
            print("Hola " @@ p.getName());
            p.getValue()
        };

        function makePeople(n: Number) => {
            let result: Person[] = [] in
            for (i in range(0, n)) {
                let p = new Person("User" @@ i, i + 10) in {
                    result := result @@ [p];
                };
            };
            result
        };

        protocol Printable {
            printSelf(x): String;
        }

        type Box(value: Number) {
            value: Number = value;

            double() => self.value * 2;
        }
	
        let xs = [1,2,3,4,5] in
        let ys = [x*2 | x in xs] in
        let p = new Person("Jery", 25) in
        let b = new Box(99) in
        {
            print("Adult? " @@ (if (p.isAdult()) "yes" else "no"));

            let i = 0 in
            while (i < 3) {
                print("Loop: " @@ i);
                i := i + 1;
            };

            print("People list:");
            let ps = makePeople(5) in
            for (q in ps) {
                print(q.getName() @@ " age=" @@ q.age);
            };

            print("Box double: " @@ b.double());

            if (b is Box) {
                let bb: Box = b as Box in {
                    print("Downcast ok: " @@ bb.value);
                }
            } else {
    			"No box"		
			};

            let f = (x: Number, y: Number) -> Number => x + y in
            print("Functor sum: " @@ f(10, 20));

        }
    "#);

    // test_program(r#"
    //     function sum_until(max : Number): Number => {
    //         let result = 0, i = 0 in
    //         while (i < max) {
    //             result := result + i;
    //             i := i + 1;
    //         };
    //         result
    //     };
    // "#);

    // test_program(r#"
    //     function sum_vec(v): Number => {
    //         let total = 0 in
    //         for (i in v) {
    //             if (i < 0) {
    //                 total := total + (0 - i);
    //             } elif (i == 0) {
    //                 total := total + 0;
    //             } else {
    //                 total := total + i;
    //             };
    //         };
    //         total
    //     };
    // "#);

    // test_program(r#"
    //     function factorial(n: Number): Number => {
    //         let result = 1, i = 1 in {
    //             while (i <= n) {
    //                 result := result * i;
    //                 i := i + 1;
    //             };
    //             result
    //         }
    //     };
    // "#);

    // test_program(r#"
    //     let evens = [ x * 2 | x in [1, 2, 3, 4, 5] ];
    //     evens;
    // "#);

    // test_program(r#"
    //     if (true) {
    //         1
    //     } elif (false) {
    //         2
    //     } else {
    //         3
    //     };
    // "#);

    // test_program(r#"
    //     let a = 10 in {
    //         let b = 20 in {
    //             a := a + b;
    //             a
    //         }
    //     };
    // "#);

    // test_program(r#"
    //     function make_adder(n): Function => {
    //         function (x): Number => { x + n }
    //     };
    //     make_adder(5)(3);
    // "#);

    // test_program(r#"
    //     let v = [1, 2, 3, 4] in v[2];
    // "#);

    // test_program(r#"
    //     function f(a, b): Number => { if (a > b) { a } else { b } };
    //     function g(): Number => {
    //         let r = f(10, 20) in
    //         r
    //     };
    //     g();
    // "#);

    // test_program(r#"
    //     { let x = 1 in { x := x + 1; x } };
    // "#);

    // test_program(r#"
    //     let s = "hello" in {
    //         s
    //     };
    // "#);

    // test_program(r#"
    //     function nested(a) : Number => {
    //         let sum = 0 in
    //         for (i in a) {
    //             for (j in i) {
    //                 if (j % 2 == 0) { sum := sum + j } else { sum := sum + 0 };
    //             };
    //         };
    //         sum
    //     };
	// "#);
}

