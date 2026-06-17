# HULKForge ⚒️

**A compiler for the HULK language, written in Rust.** 🧱

HULKForge is an end-to-end compiler that takes a program in the HULK language
(*Havana University Language Kompilation*) and produces a native binary, going
through the four classical phases: lexical analysis, syntactic analysis,
semantic analysis, and C code generation.

The project is structured as a single-pass monolithic pipeline: each phase
consumes the output of the previous one and halts the flow as soon as it
detects the first error, emitting a diagnostic in the format
`(line, column) TYPE: message` and an exit code that identifies the failing
phase (`1` lexical, `2` syntactic, `3` semantic, `0` success).

---

## Project structure 📁

```
HulkForge/
│
├── Cargo.toml                 # Rust manifest: dependencies and metadata
├── Cargo.lock                 # Pinned dependency versions
├── Makefile                   # `make build` -> leaves ./hulk at the root
├── run_linux_tests.sh         # Test harness for Linux/macOS
├── run_local_tests.sh         # Test harness for Windows/MinGW
├── README.md
│
├── examples/                  # Sample HULK programs
│   ├── vectors.hulk
│   ├── operator_overloading.hulk
│   ├── compound_assignment.hulk
│   └── inference_protocols.hulk
│
└── src/                       # Compiler source code
    │
    ├── main.rs                # CLI driver: orchestrates the 4 phases
    │
    ├── lexer/                 # ── Phase 1: Lexical analysis ──
    │   ├── mod.rs             #    Module declaration
    │   ├── lexer.rs           #    Token enum, TokenStream, Span, LexError
    │   └── test.rs            #    Lexer unit tests
    │
    ├── parser/                # ── Phase 2: Syntactic analysis ──
    │   ├── mod.rs             #    Module declaration
    │   ├── ast.rs             #    Typed AST definitions
    │   ├── parser.rs          #    Hand-written recursive-descent LL(1) parser
    │   └── tests.rs           #    Parser unit tests
    │
    ├── semantic/              # ── Phase 3: Semantic analysis ──
    │   ├── mod.rs             #    Module declaration
    │   ├── context.rs         #    Context: scopes, type/function registries
    │   ├── checker.rs         #    Type validation and inference visitor
    │   └── tests.rs           #    Checker unit tests
    │
    ├── codegen.rs             # ── Phase 4: AST lowering to C ──
    │
    ├── evaluator/             # Frozen experimental probe (unused in prod)
    └── struct_printer.rs      # AST pretty-printer for debugging
```

### Module roles 🧭

| Module         | Role in the pipeline                                                                   |
|----------------|----------------------------------------------------------------------------------------|
| `lexer/`       | Tokenizes the source with Logos (DFA compiled at build time). Reports invalid characters and malformed strings. |
| `parser/`      | Builds a typed AST via recursive-descent LL(1). Handles errors with `ParseError { span, message }`. |
| `semantic/`    | Walks the AST validating types, scopes, inheritance and structural protocols. Performs limited type inference. |
| `codegen.rs`   | Lowers the verified AST into C code with a tagged-value runtime and per-type vtables. Delegates to `cc`/`gcc`/`clang` to produce the final binary. |
| `main.rs`      | Reads the source file, runs the phases in order, and emits diagnostics. Defines the exit-code contract (0/1/2/3). |

---

## How to run the project ▶️

### Prerequisites 🛠️

HULKForge requires two tools installed on the system:

| Tool                | Min. version | Purpose                              |
|---------------------|--------------|--------------------------------------|
| **Rust**            | 1.70+        | Compile the compiler                 |
| **C compiler**      | any          | Compile the C output to a native bin |

The C compiler can be `cc`, `gcc`, or `clang` — HULKForge tries them in that
order and uses the first one it finds. On Ubuntu/Debian install with:

```bash
sudo apt install build-essential
```

### Dependency installation 📦

Rust dependencies are declared in `Cargo.toml` and **managed automatically by
Cargo** — no manual installation and no `requirements.txt`-style file is
needed. On the first `cargo build`, Cargo downloads and compiles `logos`
(lexer), `thiserror` (error types), and `indexmap` (ordered maps).

If you need to pre-download dependencies (e.g. for an offline environment), run
once on a connected machine:

```bash
cargo fetch                    # downloads all deps into the local cargo cache
```

### Step-by-step 🧭

#### 1. Clone the repository and switch to the active branch 🔀

```bash
git clone https://github.com/JosuSC/HulkForge.git
cd HulkForge
```

#### 2. Build the compiler 🏗️

```bash
cargo build --release          # produces target/release/hulk_forge
make build                     # alias: copies the binary to ./hulk at the root
```

Verify it works:

```bash
./hulk                         # prints usage (not an error)
# expected output: (0,0) SYNTACTIC: usage: hulk <file.hulk>
```

#### 3. Compile a HULK program ⚙️

```bash
./hulk examples/vectors.hulk   # generates ./output.c and ./output
```

On a clean run it prints nothing and the exit code is `0`. On error it reports
to `stderr` in the contracted format:

```
(line,column) TYPE: message
```

#### 4. Run the generated binary ▶️

```bash
./output                       # runs the compiled HULK program
```

#### 5. Try your own programs ✍️

Create a `my_program.hulk` file and run it:

```bash
./hulk my_program.hulk && ./output
```

#### 6. (Optional) Run the professor's test suite 🧪

HULKForge ships with a harness that mirrors the professor's grader:

```bash
# Linux / macOS
bash run_linux_tests.sh <tests_dir>

# Windows / MinGW (uses ./hulk.exe)
bash run_local_tests.sh <tests_dir>
```

`<tests_dir>` is the path to the `tests/hulk/` folder of the official test
repository. The script walks the `ok/` categories (compares stdout against
`.expected`) and the `errors/` categories (verifies exit code and error type).

#### 7. (Optional) Run the Rust unit tests ✅

Each module ships its own test suite in `src/*/test.rs` or `tests.rs`:

```bash
cargo test                     # runs every unit test
cargo test lexer               # filter by module name
```

---

## Typical workflow 🔁

```bash
# Edit a .hulk file in examples/ or your own path
$EDITOR examples/vectors.hulk

# Compile it with HULKForge
./hulk examples/vectors.hulk

# If there are errors, you'll see something like:
#   (3,15) SEMANTIC: type A does not have attribute z
# Fix the .hulk and retry.

# If it passes, run the generated binary
./output
```

---

## Dependencies 📚

| Crate       | Version | Purpose                                    |
|-------------|---------|--------------------------------------------|
| `logos`     | 0.14    | Declarative lexer with compiled DFA        |
| `thiserror` | 2       | Error types with derived `Display`         |
| `indexmap`  | 2       | Insertion-order-preserving maps            |

No runtime dependencies: the generated `./output` binary links only against
`libc` and `libm`.

---

## License 📜

Academic project © 2026 — Compilers course, University of Havana.
