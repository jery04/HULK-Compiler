#![allow(dead_code)]

mod lexer;          // lexer module
mod parser;         // parser module
mod evaluator;      // evaluator module
mod struct_printer; // structure printer module
#[path = "semantic/semantic.rs"]
mod semantic;       // semantic checker module
use struct_printer::test_program; // import test_program directly


fn main() {

    test_program(true, r#"
        type A {
            value: Number = 10;
            getValue() => self.value;
            inc() {
                self.value := self.value + 1;
                self.value
            }
        }

        type Person(name: String, age: Number) inherits A {
            name: String = name;
            age: Number = age;

            getName() => self.name;

            birthday() {
                self.age := self.age + 1;
                self.age
            }

            isAdult() => self.age >= 18;
        }

        function greet(p: Person) {
            print("Hola " @@ p.getName());
            p.getValue()
        }

        function makePeople(n: Number) {
            let result: Person[] = [] in
            for (i in range(0, n)) {
                let p = new Person("User" @@ i, i + 10) in (
                    result := result @@ [p];
                );
            };
            result
        }

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

        };
    "#);

    test_program(false, r#"
        function square(x: Number): Number => x ^ 2;

        function hypotenuse(a: Number, b: Number): Number =>
            sqrt(square(a) + square(b));

        function fancyMessage(n: Number): String =>
            "The hypotenuse is " @ n;

        function printOps(x: Number, y: Number) {
            print("x + y = " @ (x + y));
            print("x - y = " @ (x - y));
            print("x * y = " @ (x * y));
            print("x / y = " @ (x / y));
        }

        function factorial(n: Number): Number =>
            if (n <= 1)
                1
            else
                n * factorial(n - 1);

        function gcd(a: Number, b: Number): Number =>
            while (a > 0)
                let m = a % b in {
                    b := a;
                    a := m;
                };

        function testLetAndAssign(): Number {
            let a = 5, b = 10, c = 20 in {
                print("a + b = " @ (a + b));
                print("b * c = " @ (b * c));
                print("c / a = " @ (c / a));

                a := a + 1;
                let a = a * 2 in {
                    print("inner a = " @ a);
                };

                print("outer a = " @ a);

                let x = 0 in {
                    print("x before := " @ x);
                    x := 42;
                    print("x after := " @ x);
                };

                a + b + c
            }
        }

        function parityLabel(n: Number): String =>
            if (n % 2 == 0)
                "even"
            else
                "odd";

        function mod3Label(n: Number): String =>
            let m = n % 3 in
                if (m == 0)
                    "Magic"
                elif (m == 1)
                    "Woke"
                else
                    "Dumb";

        function testIfBlocks(): String {
            let a = 42 in
                if (a % 2 == 0) {
                    print(a);
                    print("Even");
                    "done-even"
                } else {
                    print("Odd");
                    "done-odd"
                }
        }

        function sumRange(start: Number, end: Number): Number {
            let acc = 0 in
                for (x in range(start, end)) {
                    acc := acc + x;
                    acc
                }
        }

        function countdown(n: Number): Number {
            let a = n in
                while (a >= 0) {
                    print(a);
                    a := a - 1;
                }
        }

        type Point(x: Number, y: Number) {
            x: Number = x;
            y: Number = y;

            getX(): Number => self.x;
            getY(): Number => self.y;

            setX(nx: Number): Number => self.x := nx;
            setY(ny: Number): Number => self.y := ny;

            norm(): Number =>
                sqrt(square(self.x) + square(self.y));

            toString(): String =>
                "(" @ self.x @ ", " @ self.y @ ")";
        }

        type PolarPoint(phi: Number, rho: Number)
            inherits Point(rho * cos(phi), rho * sin(phi)) {

            rho(): Number => sqrt(square(self.getX()) + square(self.getY()));

            phi(): Number =>
                if (self.getX() == 0)
                    0
                else
                    atan(self.getY() / self.getX());
        }

        type Person(firstname: String, lastname: String) {
            firstname: String = firstname;
            lastname: String = lastname;

            name(): String => self.firstname @@ self.lastname;
        }

        type Knight(title: String, firstname: String, lastname: String)
            inherits Person(firstname, lastname) {

            name(): String => title @@ base();
        }

        function typedArithmetic(a: Number, b: Number): Number {
            let x: Number = a + b,
                y: Number = a * b in
                x + y
        }

        type BoxedNumber(value: Number) {
            value: Number = value;

            get(): Number => self.value;
            set(v: Number): Number => self.value := v;
        }

        type A {
            id(): String => "A";
        }

        type B(msg: String) inherits A {
            msg: String = msg;

            id(): String => "B: " @ self.msg;
        }

        type C(label: String) inherits A {
            label: String = label;

            id(): String => "C: " @ self.label;
        }

        function randomA(): A {
            let r = rand() in
                if (r < 0.33)
                    new B("from B")
                elif (r < 0.66)
                    new C("from C")
                else
                    new A()
        }

        function describeA(x: A): String =>
            if (x is B)
                let y: B = x as B in
                    "It is a B with msg = " @ y.id()
            elif (x is C)
                let z: C = x as C in
                    "It is a C with label = " @ z.id()
            else
                "It is a plain A: " @ x.id();

        {
            print("=== Basic expressions and functions ===");
            print(fancyMessage(hypotenuse(3, 4)));

            print("=== Function forms and recursion ===");
            print("factorial(5) = " @ factorial(5));
            print("gcd(48, 18) = " @ gcd(48, 18));

            print("=== Let, scopes and assignment ===");
            print("testLetAndAssign() = " @ testLetAndAssign());

            print("=== Conditionals and elif ===");
            print("parityLabel(42) = " @ parityLabel(42));
            print("mod3Label(42) = " @ mod3Label(42));
            print(testIfBlocks());

            print("=== Loops ===");
            print("sumRange(0, 10) = " @ sumRange(0, 10));
            countdown(5);

            print("=== Types, inheritance and polymorphism ===");
            let p: Point = new Point(3, 4) in {
                print("p = " @ p.toString());
                print("p.norm() = " @ p.norm());
            };

            let pp: PolarPoint = new PolarPoint(PI / 4, 5) in {
                print("pp = " @ pp.toString());
                print("pp.rho() = " @ pp.rho());
            };

            let k: Knight = new Knight("Sir", "Phil", "Collins") in {
                print("Knight name = " @ k.name());
            };

            print("=== Typed arithmetic and BoxedNumber ===");
            print("typedArithmetic(2, 3) = " @ typedArithmetic(2, 3));
            let bn: BoxedNumber = new BoxedNumber(10) in {
                print("BoxedNumber initial = " @ bn.get());
                bn.set(99);
                print("BoxedNumber after set = " @ bn.get());
            };

            print("=== Conforming, is, as (downcasting) ===");
            let a0: A = randomA() in {
                print(describeA(a0));
            };

            print("=== End of HULK test program up to A.8.6 ===");
        }
    "#);

    test_program(false, r#"
       let evens = [ x * 2 | x in [1, 2, 3, 4, 5] ] in
       print(evens);
    "#);

    test_program(false, r#"
        function sum_until(max : Number): Number {
            let result = 0, i = 0 in 
            while (i < max) {
                result := result + i;
                i := i + 1;
            };
            result
        }
    "#);

    test_program(false, r#"
        function sum_vec(v): Number {
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
        }
    "#);

    test_program(false, r#"
        function factorial(n: Number): Number {
            let result = 1, i = 1 in {
                while (i <= n) {
                    result := result * i;
                    i := i + 1;
                };
                result
            }
        }
    "#);

    test_program(false, r#"
        if (true) {
            1
        } elif (false) {
            2
        } else {
            3
        };
    "#);

    test_program(false, r#"
        let a = 10, c = 0 in {
            let b = 20 in {
                a := a + b + c;
                a
            }
        };
    "#);

    test_program(false, r#"
        let v = [1, 2, (2+4), 3, 4] in v[2];
    "#);

    test_program(false, r#"
        function f(a, b): Number { if (a > b) { a } else { b } }
        function g(): Number {
            let r = f(10, 20) in
            r
        }
        g();
    "#);

    test_program(false, r#"
        { let x = 1 in { x := x + 1; x } };
    "#);

    test_program(false, r#"
        let s = "hello" in {
            s
        };
    "#);

    test_program(false, r#"
        function nested(a) : Number {
            let sum = 0 in
            for (i in a) {
                for (j in i) {
                    if (j % 2 == 0) { sum := sum + j } else { sum := sum + 0 };
                };
            };
            sum
        }
    "#);
}

