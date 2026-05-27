#![allow(dead_code)]

mod lexer;          // lexer module
mod parser;         // parser module
mod evaluator;      // evaluator module
mod struct_printer; // structure printer module
mod semantic;       // semantic checker module
use struct_printer::test_program; // import test_program directly


/// Demo entry point with a few sample programs.
fn main() {

    test_program(false, r#"
        protocol C {
            greet() : String;
        }

        protocol A extends C {
            hey() : String;
        }

        protocol B extends A {
            hello() : String;
        }
        print(42);
    "#);

    test_program(false, r#"

        type A {
            x = 0;

            get_x() => self.x;
        }

        type Person(firstname, lastname) inherits A {
            firstname = firstname;
            lastname = lastname;

            num(a: Number): Number => a+1;
            hole() => "This is a hole in the Person type";
            name(a: String, b: Number): String => self.firstname @@ self.lastname;
        }

        type Knight inherits Person {
            name(a: String, b: Number): String => "Sir" @@ base();
        }

        let p : Person = new Knight("Phil", "Collins", 3) in
            print(p.get_x()); 
    "#);

    test_program(false, r#"
        protocol Greetable {
            greet() : String;
        }

        type Person(name) {
            name: String = name;

            greet(): String => "Hello, I am " @ self.name;
        }

        let p : Greetable = new Person("Alice") in print(p.greet());

    "#);

    test_program(false, r#"
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

    test_program(false, r#"
        protocol MyProtocol  {
            greet() : String;
            alwaysTrue() : Boolean;

        }
    
        protocol Printable extends MyProtocol {
            printSelf() : String;
            printValue() : Number;
        }

        type Box {
            value = 10;

            printSelf(): String => "Box(" @ self.value @ ")";
            printValue(): Number => self.value;
            alwaysTrue(): Boolean => true;
            greet(): String => "Hello, I am a box!";

        }

        let p : Printable = new Box() in print(p.greet());

    "#);

    test_program(false, r#"
        function g(a): Number => a+5;

        let b: Number = 4*2 in
            let a: Number = g(5) + b in {
                print(a);
            };
    "#);

    test_program(false, r#"
        {
            let a = 42, let mod = a % 3, let b: Boolean = true in
                print(
                    if (mod == 0 & b) "Magic"
                    elif (mod % 3 == 1) "Woke"
                    else "Dumb"
                );

            let a: Number = 42, mod = a % 3, b = true in
                print(
                    if (mod == 0 & b) "Magic"
                    elif (mod % 3 == 1) "Woke"
                    else "Dumb"
                );


            let a = 42 in 
                let mod: Number = a % 3 in
                    let b = true in
                        print(
                            if (mod == 0 & b) "Magic"
                            elif (mod % 3 == 1) "Woke"
                            else "Dumb"
                        );
            
            let a = (let b = 6 in b * 7) in print(a);
        };
    "#);

    test_program(false, r#"
        type B {
            d = 0;

            get_d() => self.d;
        }
        type A inherits B {
            c = 0;

            get_c() => self.c;
        }
        type Person(name: String, age: Number) inherits A {
            name: String = name;
            age: Number = age;

            greet() => print("Hola, soy " @ self.name @ " y tengo " @ self.age @ " años");
            get_age() => self.age;
        }

        {
            let jery = new Person("Jery", 21) in 
                print(jery.get_d());
        }
    "#);
    
    test_program(false, r#"
        print(42); 
    "#);

    test_program(false, r#"
        function sum_until(max : Number): Number {
            let result = 0, i = 0 in (
                while (i < max) {
                    result := result + i;
                    i := i + 1;
                };
                result
            )
        }
        print(sum_until(10));
    "#);

    test_program(false, r#"
        function factorial(n: Number, j: String): Number {
            let result = 1, i = 1 in {
                while (i <= n) {
                    result := result * i;
                    i := i + 1;
                };
                result
            }
        }
        if (factorial (1, "testing_param") > 2 & true) {
            print("Factorial of 1 is 1");
        } else {
            print("Error in factorial function");
        };

    "#);

    test_program(false, r#"
        let x: Number=2, y: Number=4 in(
        let b: String="text", h: Boolean = true in
        if (true) {
            1
        } elif (false & ((true | x>y) & (y>10))) {
            2
        } elif (true == h) {
            3
        } else {
            4
        };)
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
        function f(a, b): Number { if (a > b) { a } else { b } }
        
        function g(): Number {
            let r = f(10, 20) in
            r
        }
        g();
    "#);

    test_program(false, r#"
        function nested(a: Number) : Number {
            let sum = 0 in {
                for (i in range(0, a)) {
                    if (i % 2 == 0) { sum := sum + i  } else { sum := sum + 0};
                };
                sum
            }
        }
        nested(5);
    "#);

    test_program(false, r#"
        {
            print(42);
            print(sin(PI/2));
            print("Hello World");
        };
    "#);

    // ── self válido: referencia a atributo ──
    test_program(false, r#"
        type Counter(n) {
            n = n;
            get() => self.n;
        }
        new Counter(0).get()
    "#);

    // ── self válido: llamada a método ──
    test_program(false, r#"
        type Counter(n) {
            n = n;
            inc() => self.n + 1;
            double() => self.inc() * 2;
        }
        new Counter(3).double()
    "#);

    // ── self válido: como argumento de función ──
    test_program(false, r#"
        function getId(c) => 42;
        type Counter(n) {
            n = n;
            id() => getId(self);
        }
        new Counter(0).id()
    "#);

    // ── self oculto por parámetro de método (spec: no es keyword) ──
    test_program(false, r#"
        type Counter(n) {
            n = n;
            add(self) => self + 1;
        }
        new Counter(0).add(5)
    "#);

    // ── self oculto por let dentro de método ──
    test_program(false, r#"
        type Counter(n) {
            n = n;
            compute() => let self = 42 in self * 2;
        }
        new Counter(0).compute()
    "#);

    // ── self fuera de método: error semántico ──
    test_program(false, r#"
        self.x
    "#);

    // ── self fuera de método en función global: error semántico ──
    test_program(false, r#"
        function bad() => self;
        bad()
    "#);

    // ── self como target de := : error semántico ──
    test_program(false, r#"
        type A {
            f() {
                self := new A();
            }
        }
        new A().f()
    "#);

    test_program(false,r#"
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

    // ── base válido: llamada al método padre ──
    test_program(false, r#"
        type Animal {
            name() => "Animal";
        }
        type Dog inherits Animal {
            name() => base() @ " Dog";
        }
        new Dog().name()
    "#);

    // ── base fuera de método: error semántico ──
    test_program(false, r#"
        type A {
            x = 0;
        }
        base()
    "#);

    // ── base en tipo sin herencia: error semántico ──
    test_program(false, r#"
        type A {
            f() => base();
        }
        new A().f()
    "#);

}