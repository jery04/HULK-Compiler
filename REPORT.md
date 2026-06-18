# HULKForge — Reporte del Proyecto

Compilador de **HULK** escrito en **Rust** . Este documento explica la
arquitectura del compilador, las decisiones de diseño y su justificación, cómo está
implementada cada fase, y —con especial profundidad— los *features* del lenguaje y las
**extensiones propias** que diseñamos. Está pensado para leerse sin necesidad de abrir el código
fuente: cuando citamos un módulo es solo para que el lector pueda contrastar, no porque haga falta
leerlo para entender el reporte.

 
---

## Índice

- [1. Visión general y pipeline](#1-visión-general-y-pipeline)
- [2. Dependencias y restricciones de compilación](#2-dependencias)
- [3. El contrato de interfaz y el modelo de errores](#3-el-contrato-de-interfaz-y-el-modelo-de-errores)
- [4. Análisis léxico](#4-análisis-léxico)
- [5. Análisis sintáctico (parser) y el AST](#5-análisis-sintáctico-parser-y-el-ast)
- [6. Análisis semántico (las dos pasadas)](#6-análisis-semántico-las-dos-pasadas)
- [7. Sistema de tipos y relación de conformidad](#7-sistema-de-tipos-y-relación-de-conformidad)
- [8. Backend: generación de código a C](#8-backend-generación-de-código-a-c)
- [9. Features del lenguaje y extensiones (núcleo del reporte)](#9-features-del-lenguaje-y-extensiones)
  - [9.1 Inferencia de tipos (A.9)](#91-inferencia-de-tipos-a9)
  - [9.2 Protocolos / tipado estructural (A.10)](#92-protocolos--tipado-estructural-a10)
  - [9.3 Iterables (A.11)](#93-iterables-a11)
  - [9.4 Vectores (A.12)](#94-vectores-a12)
  - [9.5 Extensión propia: sobrecarga de operadores](#95-extensión-propia-sobrecarga-de-operadores)
  - [9.6 Extensión propia: asignación compuesta](#96-extensión-propia-asignación-compuesta)
  - [9.7 Extensión propia: interpolación de cadenas](#97-extensión-propia-interpolación-de-cadenas)
- [10. Alternativas investigadas pero descartadas](#10-alternativas-investigadas-pero-descartadas)
- [11. Pruebas y validación](#11-pruebas-y-validación)
- [12. Limitaciones y trabajo futuro](#12-limitaciones-y-trabajo-futuro)
- [13. Conclusión](#13-conclusión)

---

## 1. Visión general y pipeline

HULK es un lenguaje **estáticamente tipado, orientado a expresiones y a objetos**, con herencia
simple, polimorfismo, inferencia de tipos y protocolos estructurales. Un programa es una secuencia
de declaraciones (funciones, tipos, protocolos) seguida de **una única expresión global** que actúa
como punto de entrada.

El compilador es una *pipeline* clásica de fases bien separadas, con interfaces explícitas entre
ellas. El flujo real es:

```
código fuente
   │  lexer (logos)               → TokenStream (SpannedToken + LexError)
   ▼
tokens
   │  parser (descenso recursivo  → AST (parser/ast.rs) + Vec<ParseError>
   │          + escalada de precedencia)
   ▼
AST
   │  semántico (2 pasadas:       → AST validado + tipos inferidos
   │   predeclarar → chequear, con inferencia)
   ▼
AST tipado
   │  codegen (AST → C)           → output.c
   ▼
output.c
   │  cc/gcc/clang -O2 -lm        → ./output (ELF nativo Linux x86_64)
   ▼
./output
```

La decisión arquitectónica más importante es el **backend**: en lugar de construir una
representación intermedia propia y una máquina virtual, el compilador **transpila el AST tipado a
código C** y delega en el compilador de C del sistema la generación del binario nativo `./output`.
La sección 8 y la sección 10 justifican esta decisión.

**Organización del repositorio** (`src/`):

| Módulo | Responsabilidad |
| --- | --- |
| `lexer/` (`lexer.rs`, `test.rs`) | Tokenización con `logos`; posiciones 1-based |
| `parser/` (`ast.rs`, `parser.rs`, `tests.rs`) | Gramática → AST por descenso recursivo |
| `semantic/` (`checker.rs`, `context.rs`, `tests.rs`) | Inferencia + verificación de tipos en dos pasadas |
| `codegen.rs` | **Backend real**: AST tipado → C |
| `main.rs` | *Driver* del contrato (`./hulk <archivo>`): gating por prioridad, exit codes, emisión de `./output` |
| `struct_printer.rs` | Utilidad de depuración: imprime el AST con sangría de árbol |


`main.rs` es el corazón operativo: ejecuta las fases en orden, aplica el *priority gating* del
contrato (léxico → 2 errores antes que sintáctico, etc.), imprime los diagnósticos a `stderr` en el
formato exigido y, en caso de éxito, baja a C y compila `./output`.

---

## 2. Dependencias

El proyecto mantiene **deliberadamente pocas dependencias**, todas de propósito acotado:

| Crate | Uso |
| --- | --- |
| `logos` | Generador de analizadores léxicos: a partir de atributos `#[token(...)]`/`#[regex(...)]` produce un escáner basado en autómata finito determinista |
| `thiserror` | Derivación de tipos de error ergonómicos (`LexError`) |
| `indexmap` | Mapas con orden de inserción estable (importante para que el *layout* de atributos/métodos y la salida sean deterministas) |

El binario se construye con un `Makefile` mínimo (`make build` → `cargo build --release` → copia el
ejecutable a `./hulk`), y en tiempo de ejecución de `./hulk` se invoca el compilador de C del
sistema (`cc`/`gcc`/`clang`), todos presentes en el Ubuntu del evaluador.

---

## 3. El contrato de interfaz y el modelo de errores

El proyecto se evalúa mediante un CI automatizado que exige una **interfaz exacta**. La respetamos
al pie de la letra, porque si la interfaz falla, ningún test corre:

- `make build` deja un ejecutable **`./hulk`** en la raíz.
- `./hulk <archivo.hulk>`:
  - **Éxito** → produce un ejecutable **`./output`** y termina con código **0**.
  - **Error** → imprime **una línea por error** a `stderr` con el formato exacto
    `(línea,columna) TIPO: mensaje`, y termina con el código del error **más fundamental**
    presente.

| Exit | TIPO | Prioridad |
| --- | --- | --- |
| `1` | `LEXICAL` | más alta |
| `2` | `SYNTACTIC` | media |
| `3` | `SEMANTIC` | más baja |

El **gating por prioridad** está implementado en `main.rs`: si la fase léxica produjo errores, se
emiten y se sale con 1 (sin dejar que el parser genere ruido en cascada); si no, se intenta parsear
y, si hay errores sintácticos, se sale con 2; si no, se chequea semánticamente y, si hay errores, se
sale con 3; solo si todo está limpio se genera `./output` y se sale con 0. Las posiciones son
1-based (el tipo `Pos` ya lo es); se usa `(0,0)` cuando no hay una posición sensata.

Cada fase produce su propio tipo de diagnóstico con *span*: `LexError { msg, span, slice }`,
`ParseError { span, message }` y `SemanticError { span, message }`. El *driver* los traduce a la
línea del contrato. Verificamos contra los tests reales de Matcom que la suite de errores solo
exige **el exit code correcto y la presencia del token de TIPO** en `stderr` (no compara el texto
del mensaje), de modo que nuestros mensajes pueden ser descriptivos en español sin riesgo.

---

## 4. Análisis léxico

El lexer (`src/lexer/lexer.rs`) está construido sobre **`logos`**. Cada token se declara con
atributos sobre un `enum` (`#[token("if")]`, `#[regex(r"[0-9]+...")]`, …) y `logos` genera en tiempo
de compilación un escáner eficiente que reconoce la entrada con la regla de *match más largo* y
resuelve los empates por orden de declaración —así una palabra clave como `if` tiene prioridad sobre
el patrón genérico de identificador—. El escaneo es lineal respecto al tamaño del texto.

**Posiciones.** Como `logos` trabaja con *offsets* de byte, mantenemos un `LineIndex` que convierte
cada *offset* a `(línea, columna)` 1-based en O(log n) mediante búsqueda binaria sobre los inicios de
línea. Cada token se envuelve en un `SpannedToken { token, span, slice }`.

**`TokenStream`.** Es la única interfaz que el parser usa. Garantiza: (1) un `Eof` final siempre
presente (el parser nunca recibe `None`); (2) *lookahead* arbitrario mediante un *buffer* (`peek_n`);
y (3) **recolección de errores léxicos sin detenerse** — un carácter inválido no aborta el escaneo,
se registra y se continúa, de modo que una sola corrida reporta todos los errores léxicos.

**Cobertura de tokens.** Palabras clave de todo HULK (`type`, `function`, `let`, `in`, `if`/`elif`/
`else`, `while`, `for`, `new`, `inherits`, `is`, `as`, `base`, `true`/`false`, y `protocol` con su
sinónimo `interface`); operadores de uno y varios caracteres, incluidos `:=`, `=>`, `->`, `@`, `@@`,
los comparadores `== != <= >=` y los **operadores de asignación compuesta** `+= -= *= /= %= ^= @=`
(extensión, §9.6). Los identificadores que empiezan con `_` se tokenizan como `InternalIdent`, una
categoría **reservada** para el código que genera el propio compilador.

**Comentarios.** Se soportan comentarios de línea `//` (descartados con `#[logos(skip ...)]`). No
hay comentarios de bloque `/* */` (HULK no los exige).

**Errores léxicos.** Son los que corresponden a *caracteres que no forman ningún token válido*:
caracteres inesperados (`~`, backtick, `$` suelto), cadenas mal cerradas, **secuencias de escape
inválidas** (p. ej. `"\q"` — un escape desconocido produce error léxico, no se preserva). Los
escapes válidos son `\" \n \t \r \0 \\`. Estos casos terminan con exit 1.



---

## 5. Análisis sintáctico (parser) y el AST

### 5.1 Algoritmo de parsing

El parser (`src/parser/parser.rs`) es un **analizador de descenso recursivo predictivo con escalada
de precedencia** (*precedence climbing*) para las expresiones binarias. Cada no-terminal es una
función, y la jerarquía de precedencia se codifica como una cascada de funciones:

```
parse_expr → parse_assign → parse_or → parse_and → parse_not → parse_cmp
           → parse_cat (@,@@) → parse_add → parse_mul → parse_pow
           → parse_unary → parse_postfix → parse_primary
```

`parse_postfix` maneja los sufijos encadenables: acceso a miembro `obj.campo`, llamada a método
`obj.m(args)`, `is`/`as`, e **indexación** `a[i]` (vectores). `parse_primary` maneja literales,
identificadores y llamadas, `new`, paréntesis, bloques `{...}`, literales de vector, `let`, `if`,
`while`, `for`.

### 5.2 ¿Por qué este algoritmo y no otro?

Antes de fijar esta estrategia evaluamos otras alternativas:

- **Tabla LL(1) dirigida por FIRST/FOLLOW.** Es el enfoque "de pizarra", pero para una gramática de
  expresiones con muchos niveles de precedencia produce o bien una explosión de no-terminales
  (`Expr → Term → Factor → …`, uno por nivel) o bien una tabla difícil de mantener. La escalada de
  precedencia expresa lo mismo de forma más compacta y legible.
- **Generador LR/LALR (estilo `yacc`/`lalrpop`).** Habría dado parsing tabular eficiente, pero (1)
  introduce una **dependencia de generación de código en *build-time***, (2) los mensajes de error y
  la recuperación de un parser generado son más rígidos, y (3) el contrato nos exige *recuperación*
  con posiciones precisas para reportar varios errores por corrida. Un parser escrito a mano nos da
  control total sobre los diagnósticos.
- **Elegido: descenso recursivo + escalada de precedencia.** Predecible (un token de *lookahead*
  basta en casi todos los puntos), fácil de extender (cada *feature* nuevo es una función o un
  caso), y con recuperación de errores a medida.

### 5.3 Recuperación de errores

El parser **nunca hace `panic`** ante entrada inválida. Cuando `expect` falla, registra un
`ParseError` con *span* y sincroniza saltando hasta un punto seguro (`;`, `}`, `)`, o el inicio de
una declaración: `function`/`type`/`protocol`), insertando un nodo `Expr::Error` como marcador para
poder seguir. Así una sola corrida reporta múltiples errores sintácticos.

### 5.4 El AST

El AST (`src/parser/ast.rs`) tiene **un nodo por construcción semántica**, sin ruido de la gramática
concreta (no hay nodos para paréntesis ni para niveles intermedios de precedencia: la precedencia
queda codificada en la *forma* del árbol). `Program` = `Vec<Decl>` + una expresión global.

Las declaraciones son `FuncDecl`, `TypeDecl` (atributos, métodos, cláusula `inherits`) y
`ProtocolDecl` (firmas de métodos). Las expresiones (`Expr`) cubren literales, identificadores,
llamadas, `new`, acceso a campo, llamada a método, `base`, operadores binarios/unarios, `is`/`as`,
`if`/`while`/`for`, `let`, asignación `:=`, bloques, y los nodos de **vectores** que añadimos
(`VectorLit`, `VectorSized`, `VectorComp`, `Index`). **Cada nodo lleva su `Span`**, invariante que
sostiene la calidad de los diagnósticos en todas las fases posteriores.

---

## 6. Análisis semántico (las dos pasadas)

El análisis semántico (`src/semantic/`) valida las propiedades dependientes del contexto que la
gramática no puede capturar, e infiere los tipos faltantes. Está implementado en **dos pasadas**.

### 6.1 ¿Por qué dos pasadas y por qué son pasadas separadas?

- **Pasada 1 — *predeclaración* (`predeclare_*`).** Recorre solo las *declaraciones de nivel
  superior* y registra en el contexto: la firma de cada función, cada tipo (sus atributos, métodos,
  y su padre) y cada protocolo (sus firmas). No mira los cuerpos.
- **Pasada 2 — *chequeo* (`check_*`).** Ahora sí entra a los cuerpos: resuelve símbolos, infiere y
  verifica tipos, comprueba conformidad, *overrides*, aridad, etc.

**¿Por qué no se puede hacer en una sola pasada?** Por las **referencias hacia adelante**. En HULK,
una función puede llamar a otra definida más abajo; un tipo puede heredar de otro declarado después;
hay **recursión mutua** (`is_even`/`is_odd`). Si chequeáramos cuerpos a la vez que vemos las
declaraciones, al encontrar una llamada a un símbolo aún no visto no tendríamos su firma y
tendríamos que reportar un falso "no definido". Predeclarar **todas** las firmas primero hace que el
universo de nombres y tipos esté completo antes de chequear cualquier cuerpo. Son dos pasadas
*genuinamente separadas* porque la segunda **depende de que la primera haya terminado por completo**
sobre todo el programa: no es una fusión de pasos que se puedan intercalar.

### 6.2 Qué hace cada comprobación

Sobre la tabla de símbolos por ámbitos (`context.rs`), la pasada de chequeo detecta, entre otros:
identificadores/tipos/métodos no definidos; **incompatibilidades de tipo** (asignaciones, operandos,
retornos); **aridad incorrecta** en llamadas; **redefinición de funciones** (HULK no permite repetir
nombres ni tiene sobrecarga de funciones); condición no booleana en `if`/`while`;
expresión no iterable en `for`; herencia de un tipo inexistente y **ciclos de herencia**; uso de
`self` fuera de método y `self` como destino de asignación; uso de `base` fuera de método o en un
tipo sin padre; *override* con firma incompatible; acceso a atributo/método inexistente (incluyendo
heredados, vía la cadena de padres).


La pasada de chequeo es también donde corre la **inferencia de tipos** (§9.1): la inferencia y la
verificación están entrelazadas porque, en HULK, los tipos inferidos de parámetros sin anotar se
determinan a partir de cómo se usan en el cuerpo, lo cual solo se sabe al recorrer ese cuerpo.

---

## 7. Sistema de tipos y relación de conformidad

El tipo de trabajo durante el análisis es `SimpleType`, con variantes `Number`, `String`, `Boolean`,
`Named(String)` (tipos de usuario y protocolos) y `Vector(Box<SimpleType>)`.

HULK es **estático y fuerte** (verificación en compilación, sin coerciones implícitas inseguras),
**nominal con un componente estructural** (la herencia es nominal; los protocolos son
estructurales), y con **inferencia opcional**.

La relación central es **conformidad** (`<=`), implementada en `context.rs::simple_type_conforms_to`
y sus auxiliares:

- Todo tipo conforma a `Object` (raíz de la jerarquía) y a sí mismo.
- `Number`/`String`/`Boolean` solo conforman consigo mismos.
- `T1 <= T2` si `T1` desciende de `T2` por la cadena de `inherits`.
- **Protocolos**: `T <= P` si `T` tiene todos los métodos de `P` con firmas compatibles,
  respetando **varianza** (parámetros *contravariantes*, retornos *covariantes*) —
  `callable_signature_compatible`. Esto incluye conformidad implícita protocolo→protocolo: `P1 <= P2`
  si todo lo que conforma a `P1` conformaría a `P2`, aunque no haya `extends` explícito.
- **Conformidad iterable**: un tipo con `current(): U` conforma a `T*` (modelado como `Vector<T>`)
  cuando `U <= T` — esto permite pasar objetos generadores donde se espera un iterable.

Para tipar expresiones multi-rama (`if`/`elif`/`else`) se calcula el **ancestro común más bajo (LCA)**
(`lowest_common_ancestor`): el tipo más específico al que conforman todas las ramas, o `Object` en
última instancia (§9.1).

---

## 8. Backend: generación de código a C

Esta es la parte del compilador que más se aparta del enfoque clásico, así que la explicamos con
detalle: contra qué backend se genera, cómo luce el código producido, y cómo se materializan los
objetos, el polimorfismo y el dispatch dinámico en el código generado.

### 8.1 Decisión: transpilar a C

El backend (`src/codegen.rs`) **traduce el AST tipado a un programa en C** y luego invoca al
compilador de C del sistema (`cc`/`gcc`/`clang -O2 -lm -o output`) para producir el binario nativo
`./output`. No hay representación intermedia propia ni máquina virtual.

**Por qué.** El plan original contemplaba una IR de tres direcciones (BANNER) + una VM propia. La
descartamos a favor de C-transpilación porque: (1) **reutiliza todo el *front/middle*** ya hecho;
(2) **delega en C** lo difícil (aritmética, *stack frames*, convención de llamada, asignación de
registros, *intrínsecos* matemáticos); (3) produce un **ELF nativo** que trivialmente "corre en
Linux x86_64"; y (4) es la ruta de **menor riesgo** a un `./output` correcto. La
sección 10 amplía esta comparación.

### 8.2 Modelo de runtime: todo valor es un `Value` etiquetado

Cada valor de HULK se representa en C con una estructura etiquetada (*tagged union* "plana"):

```c
typedef struct { int tag; double num; char* str; int b; Obj* obj; Vec* vec; } Value;
struct Obj { int type_id; Value* fields; };
struct Vec { long len; Value* data; };
```

`tag` discrimina `NUM`/`STR`/`BOOL`/`OBJ`/`VEC`. Esta representación dinámica es la clave que
**desacopla el codegen de los tipos estáticos**: las operaciones y el *dispatch* deciden en runtime
según el `tag`, de modo que el polimorfismo, `is`/`as` y los valores tipados con protocolo
"simplemente funcionan" sin que el generador necesite resolver tipos estáticos.

### 8.3 Layout de objetos y *slots* globales

Un objeto es un `Obj` con un `type_id` y un arreglo plano de atributos `fields`. En vez de calcular
*offsets* por tipo, asignamos un **índice global por nombre de atributo** y un **índice global por
nombre de método**. Cada objeto reserva el arreglo completo de *slots* de atributo; cada método se
ubica en su *slot* global. Como dentro de una jerarquía los nombres no se repiten, este esquema es
correcto y hace que el acceso `obj.campo` sea simplemente `obj->fields[SLOT_campo]`,
independientemente del tipo estático.

### 8.4 Métodos virtuales y dispatch dinámico (vtables)

El *dispatch* de métodos se hace con una **vtable por tipo**, indexada por el *slot* global del
método:

```c
typedef Value (*Method)(Value, Value*);
static Method vtables[NUM_TYPES][NUM_METHOD_SLOTS];
```

Todos los métodos comparten una firma uniforme `Value m(Value self, Value* args)` para poder vivir
en la misma tabla. Una llamada `obj.m(args)` se compila a
`vtables[obj.obj->type_id][SLOT_m](obj, args)`. Como el `type_id` es el del **tipo dinámico** del
objeto, esto **es** el polimorfismo: una variable estática `a: Animal` que en runtime contiene un
`Dog` despacha a `Dog.sound`. Los *overrides* se materializan haciendo que la vtable del hijo apunte
a la función del hijo.

### 8.5 Cómo luce el código generado (ejemplo real)

Para el programa:

```hulk
type Animal(n: String) { name: String = n; sound(): String { "..."; } }
type Dog inherits Animal { sound(): String { "Woof"; } }
let a: Animal = new Dog("Rex") in print(a.sound());
```

el backend genera (extracto literal de `output.c`):

```c
static void hulk_initall_Dog(Obj* self_o, Value* args) {
    (void)args;
    Value self_v; self_v.tag = TAG_OBJ; self_v.obj = self_o;
    Value* t0 = NULL;
    hulk_initall_Animal(self_o, t0);   /* construcción padre-primero */
}

static Value hulk_new_Dog(Value* args) {
    Obj* o = (Obj*)malloc(sizeof(Obj));
    o->type_id = 1;
    o->fields = (Value*)calloc(NUM_ATTR_SLOTS, sizeof(Value));
    hulk_initall_Dog(o, args);
    Value v; v.tag = TAG_OBJ; v.obj = o; return v;
}

static Value hulk_m_Dog_sound(Value self, Value* args) {
    (void)args; (void)self;
    Value t0 = mk_str("Woof");
    return t0;
}

static void init_tables(void) {
    parent_id[0] = -1;  vtables[0][3] = hulk_m_Animal_sound;   /* Animal */
    parent_id[1] = 0;   vtables[1][3] = hulk_m_Dog_sound;      /* Dog override */
}
```

Se ve el patrón completo: **constructor** que asigna `type_id` y reserva *slots*; **inicialización
padre-primero** (se computan los argumentos de `inherits` y se llama al `initall` del padre antes de
inicializar los atributos propios); **método** con firma uniforme; y la **vtable** que, para el
*slot* 3 (`sound`), apunta a la implementación de cada tipo (`Dog` sobrescribe `Animal`).

### 8.6 Bajada de expresiones y control de flujo

El codegen recorre el AST emitiendo **sentencias C + variables temporales**: cada expresión genera
sus sentencias y devuelve el nombre de la temporal que contiene su `Value`. Las construcciones que
en HULK son expresiones pero en C son sentencias (bloques, `let`, `if`, `while`, `for`) se bajan con
una variable de resultado pre-declarada. Los operadores aritméticos/comparativos/lógicos se
delegan a funciones de runtime (`hulk_add`, `hulk_eq`, `hulk_concat`, …); los *builtins* `sqrt/sin/
cos/exp` a `libm`, `log(base,valor)` como `log(valor)/log(base)`, y `PI`/`E` como literales. `is`
camina la cadena `parent_id`; `as` es identidad (el chequeo estático ya validó el *downcast*).

### 8.7 Lo que el backend no hace (y por qué está bien)

- **No hay recolección de basura.** Se usa `malloc` sin liberar (*malloc-and-leak*). El contrato
  evalúa el comportamiento de `./output` por separado y no a nivel de gestión de memoria; para los
  programas evaluados, filtrar es correcto y simple. Un GC es trabajo futuro (§12).
- **No hay optimizaciones propias** (plegado de constantes, CSE, etc.): delegamos cualquier
  optimización al `-O2` del compilador de C, que es maduro. Esto es coherente con la filosofía de
  apoyarse en la *toolchain* de C.

---

## 9. Features del lenguaje y extensiones

Esta es la parte central del reporte. HULK básico (expresiones, funciones, variables, condicionales,
ciclos, tipos con herencia y polimorfismo, chequeo de tipos A.8) está implementado y verificado.
A continuación nos centramos en los *features* avanzados y en las **extensiones propias**, con su
sintaxis, su semántica, lo que añadieron a la gramática, su tratamiento especial en semántico y
codegen, la **justificación de diseño**, sus límites y cómo **interactúan** entre sí.

Resumen de lo implementado más allá de A.8:

| Feature | Tipo | Estado |
| --- | --- | --- |
| Inferencia de tipos (A.9) | *extra* elegido | Implementado (con extensiones propias: LCA + síntesis de protocolos) |
| Protocolos (A.10) | *extra* elegido | Implementado (con `interface` como sinónimo) |
| Iterables (A.11) | feature | Implementado (con `Iterable`/`Enumerable` builtin) |
| Vectores (A.12) | feature | Implementado (multi-sintaxis) |
| **Sobrecarga de operadores** | **extensión propia** | Implementado |
| **Asignación compuesta** | **extensión propia** | Implementado |
| **Interpolación de cadenas** | **extensión propia** | Implementado |
| Functors/Lambdas (A.13), Macros (A.14) | feature | **No implementados** (decisión consciente, §12) |

### 9.1 Inferencia de tipos (A.9)

**Qué hace.** Permite omitir anotaciones: `function square(x) { x * x; }` infiere `x: Number` y
retorno `Number`. La inferencia corre *durante* la pasada de chequeo, antes de verificar cada
construcción.

**Cómo está implementada.** No es Hindley-Milner ni unificación global. Es una
inferencia **bottom-up best-effort** (`infer_simple_type`) con tres mecanismos:

1. **Inferencia de expresiones** ascendente: literales → tipo fijo; aritmética → `Number`; `@`/`@@`
   → `String`; comparaciones → `Boolean`; el tipo de `let`/`while`/`for`/bloque es el de su cuerpo;
   el de una llamada es el retorno de su firma.
2. **Join por LCA en ramas (A.9.2) — extensión nuestra sobre el esquema básico.** El tipo de un
   `if` multi-rama es el **ancestro común más bajo** de las ramas, no la igualdad estricta. Así
   `if (c) new Dog() else new Cat()` infiere `Animal`, y `a.sound()` resuelve y despacha
   dinámicamente.
3. **Síntesis acotada de protocolos (A.9.5) — el puente entre A.9 y A.10.** Cuando un parámetro sin
   anotar se usa como receptor de método (`x.f()`), sintetizamos un protocolo `__SynthN` que exige
   esos métodos y atamos el parámetro a él. El tipo de retorno de cada método sintetizado se infiere
   del **contexto del operador** donde se usa el resultado: `String` si se usa con `@`/`@@`, `Number`
   si con aritmética, y un marcador *indeterminado* (`None`, equivalente a `Any`) en otro caso. Un
   retorno/parámetro indeterminado **no impone restricción** de conformidad, lo que mantiene la
   permisividad correcta. Ejemplo documentado del manual:

   ```hulk
   type A { f(): String => "Hello"; g(): String => "World"; }
   function h(x) => x.f() @@ x.g();   // x : protocolo sintetizado { f(): String; g(): String }
   ```

   `A` conforma estructuralmente al protocolo sintetizado, así que `h(new A())` tipa y ejecuta.

**Justificación de diseño.** El manual (A.9) deja la estrategia abierta y solo fija *restricciones de
correctitud*. La "estrategia básica" admitida (inferir expresiones y fallar para todos los símbolos)
es sólida pero rechazaría `square(x)`, que es un test requerido. Nuestra estrategia infiere lo que
puede de forma **sólida (*sound*)**: cuando no logra determinar un tipo, **subreporta** en vez de
arriesgar falsos positivos. Esto es una decisión deliberada de ingeniería: preferimos no rechazar
programas válidos a costa de no implementar el reporte "debe tiparse explícitamente".

**Comparativa.** Hindley-Milner (ML/Haskell) infiere tipos *principales* por unificación global y
generalización; es más potente pero su maquinaria (variables de tipo, sustituciones, *occurs check*)
es desproporcionada para HULK, donde no hay genéricos paramétricos de usuario. Nuestra síntesis de
protocolos se parece más al **tipado estructural** de TypeScript/Go: inferimos "qué *forma* debe
tener este valor" en vez de "qué tipo nominal exacto".

**Límites y por qué.** No hay punto fijo completo multi-llamada (un `f(x) => x.a()` cuyo retorno solo
se determina por una llamada posterior queda como `Any`), ni el error "debe tiparse explícitamente".
Implementar el punto fijo completo es justamente lo que el manual advierte como "mucho más difícil de
lo que parece"; lo dejamos como trabajo futuro porque el riesgo de introducir falsos positivos
superaba el beneficio.

**Interacciones.** La inferencia depende de los **protocolos** (§9.2) como mecanismo de síntesis, y
alimenta la **sobrecarga de operadores** (§9.5): el tipo de `a OP b` es el retorno del método del
operador.

### 9.2 Protocolos / tipado estructural (A.10)

**Sintaxis.** `protocol P { m(args): T; ... }`, opcionalmente `protocol P extends Q { ... }`.
Aceptamos **`interface` como sinónimo exacto de `protocol`** (un alias en el lexer): no es un feature
nuevo, es reconocer dos palabras clave para el mismo concepto, lo que además nos permite pasar la
suite de *interfaces* del evaluador, escrita con `interface`.

**Semántica.** Un tipo implementa un protocolo **implícitamente** (estructuralmente) por tener los
métodos con firma compatible; no se declara la conformidad. La conformidad respeta **varianza**
(argumentos contravariantes, retornos covariantes) y la regla implícita protocolo→protocolo (§7). Un
protocolo se puede usar como anotación en cualquier sitio (variable, parámetro, retorno, atributo).

**Codegen.** Los protocolos **no existen en runtime** (como dice el manual): se borran tras el
chequeo. Una variable tipada con protocolo contiene un objeto concreto, y `p.m()` despacha por la
vtable del tipo real. Por eso no requieren nada especial en el backend: el dispatch dinámico ya los
soporta.

**Justificación y comparativa.** El tipado estructural implícito (estilo **Go interfaces** o
**protocolos de Swift**) reduce el acoplamiento: un tipo no necesita "saber" qué protocolos cumple.
Frente a las *traits* de Rust o las interfaces de Java —donde la conformidad es nominal y explícita
(`impl Trait for T` / `implements`)— el enfoque estructural es más flexible pero da errores más
tardíos y menos localizados (el error aparece en el sitio de uso, no en la definición del tipo). Lo
elegimos porque es lo que pide A.10 y porque habilita la síntesis de protocolos de §9.1.

**Builtins.** Registramos `Iterable` y `Enumerable` como protocolos *builtin* (§9.3), de modo que se
pueden usar como anotación sin declararlos.

**Límites.** No hay métodos por defecto en protocolos (los protocolos son solo firmas); se discute
como posible extensión futura.

### 9.3 Iterables (A.11)

**Sintaxis y semántica.** El protocolo iterador es `next(): Boolean` (avanza y dice si hay elemento)
y `current(): T` (elemento actual). `for (x in it) cuerpo` itera mientras `it.next()` sea verdadero,
ligando `x = it.current()` en cada vuelta.

**Adiciones a la gramática.** El `for` ya existía; lo que añadimos fue la **semántica y el codegen**
de la iteración, más el reconocimiento de iterables. Tres fuentes de iterable conviven:

- **`range(a,b)`**: se baja a un **bucle contado** en C (sin objeto iterador).
- **Objetos generadores** (cualquier tipo con `next`/`current`): se itera por **dispatch de vtable**.
- **Enumerables** (A.11.3): si el objeto no tiene `next` pero sí `iter(): Iterable`, el `for` llama
  primero a `iter()` para obtener un iterador fresco — esto permite colecciones **re-iterables**.

**Tratamiento especial en semántico.** `for` exige que la expresión sea iterable: rechazamos
escalares (`for (x in 42)` → error). El tipo del elemento se infiere para propagarlo al cuerpo. Para
funciones tipadas `T*` (iterable de `T`), añadimos la **conformidad iterable estructural** (§7) que
permite pasar un objeto generador donde se espera `Number*`.

**Codegen.** Una única rutina (`gen_iter_loop`) cubre los tres casos: detecta `range` (bucle
contado), y para lo demás resuelve el iterador (con *fallback* a `iter()` para enumerables) y baja un
`while` que llama a `next`/`current` por vtable, **o** itera un vector por índice si el valor es un
`Value` con `tag == VEC`.

**Comparativa.** El protocolo `next/current` es cercano a los iteradores de **C#** (`MoveNext`/
`Current`) y al patrón *iterator* clásico. La distinción `Iterable` vs `Enumerable` (un solo recorrido
vs. crear un iterador nuevo por recorrido) es la misma que separa `IEnumerator` de `IEnumerable` en
.NET, o `Iterator` de `Iterable` en Java. Frente a los generadores de Python (con `yield`), nosotros
no transformamos funciones en máquinas de estado: el usuario implementa el protocolo a mano (lo cual
es justo lo que muestran los tests de generadores del evaluador, que pasamos).

### 9.4 Vectores (A.12)

**El reto de diseño: dos sintaxis, una implementación.** El manual documenta una sintaxis
(`[1,2,3]`, comprensión `[e | x in it]`) y los tests del evaluador usan otra, al estilo Java/C#
(`new T[n]`, `new T[n]{ i -> e }`, literal con llaves `{1,2,3}`, tipo `T[][]`). En vez de mantener
dos implementaciones, observamos que **todas producen el mismo valor-vector en runtime**: lo único
que cambia es la sintaxis de *construcción*. Así, hay **un único tipo de vector** y varias formas de
crearlo.

**Sintaxis soportada (toda sobre el mismo runtime):**

- Literal de documentación: `[1, 2, 3]`.
- Comprensión: `[e | x in iterable]`.
- Reserva con tamaño: `new T[n]` (inicializa a 0).
- Reserva con inicializador por índice: `new T[n]{ i -> expr }`.
- Literal estilo Matcom (llaves): `{1, 2, 3}`.
- Tipos: `T[]`, `T[][]` (multidimensional).
- Compartido: indexación `a[i]`, asignación indexada `a[i] := v`, `.size()`, e iteración (los
  vectores implementan el protocolo iterable).

**Adiciones a la gramática.** Cuatro nodos nuevos en el AST (`VectorLit`, `VectorSized`,
`VectorComp`, `Index`) y reglas en el parser. Dos puntos finos:

- **`{...}` colisiona con los bloques `{ e; e; }`.** Se desambigua por el separador: si tras la
  primera expresión viene `,` es literal de vector; si viene `;` (o `}`) es bloque.
- **`[e | x in it]` colisiona con el operador `|` (OR).** El cuerpo de la comprensión se parsea a un
  nivel de precedencia **por debajo del OR**, de modo que `|` queda libre como separador de la
  comprensión (limitación menor: el cuerpo no puede usar OR de nivel superior sin paréntesis).
- **El inicializador `{ i -> e }`** es una **lambda acotada** específica de `new T[n]{...}`: liga `i`
  al índice y evalúa `e`. No es un *functor* de primera clase (que no implementamos); es una forma
  cerrada solo válida en ese sitio.

**Codegen.** Runtime de vector `Vec { long len; Value* data; }` con `mk_vec`, `vec_lit`, `vec_index`,
`vec_set`, `vec_size` y `vec_append` (para la comprensión, que crece dinámicamente con `realloc`). La
indexación y `size()` se resuelven en runtime según el `tag`.

**Comparativa y justificación.** `new T[n]` sigue el modelo de arreglos de Java/C# (tamaño fijo,
inicialización por defecto); la comprensión `[e | x in it]` es cercana a las *list comprehensions* de
Python/Haskell. Optar por **un runtime único con varias sintaxis** evita duplicar lógica y nos
permite ser fieles al manual *y* pasar los tests del evaluador a la vez. **Límites**: no hay
*bounds-checking* en runtime (un índice fuera de rango es comportamiento indefinido, como en C); es
trabajo futuro acotado.

### 9.5 Extensión propia: sobrecarga de operadores

**Idea.** `a OP b` despacha a un **método del tipo de `a`** cuando ese tipo lo define; si no, usa el
operador *builtin*. El manual (A.9.4) ya contempla esta posibilidad: *"si implementas sobrecarga de
operadores, el tipo inferido del operando debe ser el protocolo de operador apropiado"*. Es la
extensión que más conecta el resto del diseño, porque **fusiona inferencia, protocolos y dispatch
dinámico**.

**Sintaxis.** Sin sintaxis nueva: se reutiliza la de los operadores. Un tipo define métodos con
nombres convencionales y queda "sobrecargado":

```hulk
type Vec2(x: Number, y: Number) {
    x = x; y = y;
    plus(o: Vec2): Vec2 => new Vec2(self.x + o.x, self.y + o.y);
    equals(o: Vec2): Boolean => self.x == o.x & self.y == o.y;
}
let a = new Vec2(1,2) in let b = new Vec2(3,4) in print((a + b).x);  // 4
```

El mapeo operador→método (`BinOp::operator_method`) es: `+`→`plus`, `-`→`minus`, `*`→`mult`,
`/`→`div`, `%`→`mod`, `^`→`pow`, `@`/`@@`→`concat`, `==`→`equals`, `!=`→`neq`, `<`→`less`,
`>`→`greater`, `<=`→`leq`, `>=`→`geq`.

**Semántico.** En el chequeo de `BinaryOp`, si el operando izquierdo es un tipo de usuario que define
el método del operador, se verifica el operando derecho contra el **parámetro** del método (con
varianza) y se **omiten** las reglas *builtin* (que exigirían `Number`). La inferencia devuelve el
**tipo de retorno del método** del operador, cerrando el guiño de A.9.4.

**Codegen.** En `gen_binop`, si algún tipo define el método del operador, se emite un *branch* en
runtime: `(a.tag == TAG_OBJ) ? vtables[a.obj->type_id][SLOT_plus](a, {b}) : hulk_add(a, b)`. Es
decir, los objetos despachan al método y los números/strings siguen usando el *builtin* — sin coste
para el código puramente numérico.

**Justificación y comparativa.** Hay tres familias de diseño:

- **Métodos *dunder*** (Python: `__add__`): el operador busca un método de nombre fijo en el objeto.
- **Traits/typeclasses** (Rust `std::ops::Add`, Haskell `Num`): el operador se define vía una
  abstracción nominal que el tipo implementa.
- **Funciones libres sobrecargadas** (C++ `operator+`).

Elegimos el modelo **dunder estructural**: un tipo "sobrecarga" `+` con solo tener un método `plus`.
Es coherente con nuestro tipado estructural (un objeto tiene la capacidad si tiene el método) y se
integra de forma natural con el dispatch por vtable que ya teníamos. La desventaja —compartida con
Python— es que es *no nominal*: cualquier tipo con un método `plus` sobrecarga `+`, aunque no lo
"pretendiera". Es una decisión consciente a favor de la simplicidad y la uniformidad.

**Interacciones.** Compone con la **asignación compuesta** (§9.6): `v += w` usa `Vec2.plus`. Y con la
**inferencia** (el tipo de `a + b` es el retorno de `plus`). **Límite**: el *dispatch* es por el
operando izquierdo (no hay *double dispatch*); `2 + v` (número a la izquierda) usa el *builtin*
numérico, no `v.plus`. Documentado como limitación de diseño.

### 9.6 Extensión propia: asignación compuesta

**Sintaxis.** `lhs OP= rhs` para `+= -= *= /= %= ^= @=`, donde `lhs` es una variable, un campo
(`self.x += 1`) o un elemento de vector (`a[i] += 1`).

**Semántica por desazúcar.** En el parser, `lhs OP= rhs` se reescribe a `lhs := lhs OP rhs`. Esto
**no requiere cambios en semántico ni codegen**: reutiliza por completo la asignación destructiva y
el operador binario. Una consecuencia elegante: **hereda gratis la sobrecarga de operadores**, así
que `v += w` (con `v` un `Vec2`) usa `Vec2.plus`.

**Justificación y comparativa.** Es azúcar sintáctico estándar (C, Python, Ruby, Java). El interés de
diseño está en la **composición**: al desazucarlo a nivel sintáctico, el operador `+=` se beneficia
automáticamente de la extensión de §9.5, igual que en Python `+=` usa `__iadd__`/`__add__`. **Límite
documentado**: el desazúcar evalúa el `lhs` dos veces (en `a[i] += e`, tanto `a` como `i` se evalúan
en ambos lados); para expresiones con efectos colaterales esto importaría, pero en HULK los índices y
receptores típicos son puros. Una implementación industrial introduciría un temporal para evaluar el
*lvalue* una sola vez.

### 9.7 Extensión propia: interpolación de cadenas

**Sintaxis.** `"texto ${expr} más texto"`, con expresiones arbitrarias dentro de `${...}`:

```hulk
let n = "Ana", e = 30 in print("Hola, ${n}, tienes ${e}");   // Hola, Ana, tienes 30
print("${a} * ${b} = ${a * b}");                              // p.ej. 6 * 7 = 42
```

**Semántica por desazúcar.** En el parser, si un literal de cadena contiene `${`, se desazucara a una
cadena de concatenaciones `@`: `"a ${e} b"` → `"a " @ e @ " b"`. Detalles de implementación:

- **Escaneo con llaves balanceadas**: se cuenta `{`/`}` para hallar el cierre de `${...}`, de modo
  que expresiones con llaves anidadas (p. ej. un literal de vector) no rompen el análisis.
- **Parseo recursivo** del fragmento embebido con un `Parser` independiente; los errores se propagan
  al parser principal.
- **Ancla de tipo String**: el primer segmento literal (aunque sea vacío) ancla la cadena, de modo
  que `"${n}"` con `n` numérico produce un `String` (vía `@`, que convierte el número), no un número.

**No requiere cambios en lexer/semántico/codegen**: reutiliza `BinaryOp::Concat`.

**Justificación y comparativa.** Es ergonomía pura, comparable a los *template literals* de
JavaScript (`` `${x}` ``), las *f-strings* de Python (`f"{x}"`), o la interpolación de Kotlin/Swift/
Scala. Optamos por desazucararlo **en el parser** (y no en el lexer con tokens especiales) porque así
el grueso del trabajo —analizar y validar la expresión embebida— reutiliza el parser completo, sin
duplicar lógica. **Límites**: no hay especificadores de formato (`${x:0.2f}`), y la interpolación
anidada dentro de una cadena embebida es un caso de borde no soportado; son extensiones acotadas para
el futuro.

### 9.8 Cómo interactúan las extensiones entre sí

Las extensiones no son piezas aisladas: en nuestro diseño se encadenan
limpiamente. La **inferencia** sintetiza **protocolos**; los **protocolos** y el **dispatch dinámico**
del backend hacen que **iterables**, **vectores** y **sobrecarga de operadores** "simplemente
funcionen" sin casos especiales en codegen; la **asignación compuesta** se apoya en la **sobrecarga
de operadores**; y la **interpolación** se apoya en la concatenación `@`. Casi todas las extensiones
se implementaron como **desazúcar en el parser** o como **reglas adicionales sobre maquinaria
existente**, lo que minimiza el riesgo de regresiones (cada una se validó dejando verde el resto).

---

## 10. Alternativas investigadas pero descartadas

Durante el desarrollo evaluamos varias alternativas que finalmente no adoptamos:

- **BANNER IR + máquina virtual propia.** Era el plan original (una IR de tres direcciones + un
  intérprete/VM con su propio modelo de objetos y GC). La descartamos por C-transpilación: reutiliza
  todo el *front/middle*, evita reimplementar aritmética/*stack*/dispatch, y produce un binario
  nativo. El coste es depender de un compilador de C en el evaluador. La IR
  BANNER podría reintroducirse en el futuro como capa intermedia (AST→BANNER→C) puramente para
  enriquecer el análisis, sin cambiar el resultado.
- **Backend LLVM (vía `inkwell`).** Daría código nativo optimizado, pero añade una dependencia del
  *toolchain* LLVM en la máquina de *build* y una curva de FFI más empinada. Para el plazo y el
  riesgo, C fue la opción pragmática; LLVM queda como evolución natural si se quisiera generación
  nativa sin pasar por C.
- **Inferencia Hindley-Milner.** Se consideró para A.9, pero su maquinaria (unificación global,
  variables de tipo, generalización) es desproporcionada para un lenguaje sin genéricos de usuario;
  la síntesis estructural de protocolos encaja mejor con el resto del diseño (§9.1).
- **Generador de parser (LALR).** Considerado y descartado a favor del descenso recursivo escrito a
  mano por control de errores/recuperación y por no introducir generación de código en *build-time*
  (§5.2).

---

## 11. Pruebas y validación

El desarrollo siguió una disciplina de **pruebas por sección**: cada feature nuevo llegó con sus
tests, y tras cada cambio se re-corría toda la batería para detectar regresiones.

- **397 tests unitarios** (`cargo test`) sobre lexer, parser y semántico.
- **Suite del contrato de Matcom**: pasamos las **6 categorías requeridas** (`ok/minimal`,
  `ok/types`, `ok/oop`, `errors/lexical`, `errors/syntactic`, `errors/semantic`) y, además, las
  categorías *bonus* `ok/interfaces`, `ok/extras` (incluye `for`/`range`/`while`), `ok/generators` y
  `ok/arrays`.
- **`run_local_tests.sh`**: una réplica local (Windows/MinGW) del *runner* del evaluador, que nos
  permitió iterar sin depender del CI.
- **`examples/`**: programas que demuestran las extensiones (`operator_overloading.hulk`,
  `vectors.hulk`, `inference_protocols.hulk`, `compound_assignment.hulk`, `string_interpolation.hulk`),
  ejecutables y con salida verificada.

Las únicas categorías *bonus* que **no** pasamos son `ok/lambdas` y `ok/macros`, porque no
implementamos A.13/A.14 (§12). Son *on-demand* y no afectan la nota de tests.

---

## 12. Limitaciones y trabajo futuro

Limitaciones conscientes (no accidentales):

- **Functors / lambdas (A.13) y macros (A.14): no implementados.** Requieren nodos de AST, reglas de
  parser y *runtime* nuevos (especialmente las macros, que operan sobre el AST con saneamiento de
  variables). Se priorizó pulir lo demás. La única forma lambda-*ish* es el inicializador acotado
  `new T[n]{ i -> e }` de vectores.
- **Inferencia A.9 sin punto fijo completo** ni reporte, "debe tiparse explícitamente" (§9.1):
  subreportamos por solidez.
- **Sin recolector de basura** en el backend (*malloc-and-leak*); no se evalúa a nivel de compilador.
- **Sin *bounds-checking* de vectores** en runtime.
- **Sin métodos por defecto en protocolos.**

Trabajo futuro priorizado: punto fijo de inferencia con reporte de fallo; implementación de
lambdas/functors (que habilitaría combinadores perezosos sobre iterables: `map`/`filter`/`take`);
GC preciso de marcado-y-barrido; reintroducir BANNER como IR intermedia documentable; *bounds-check*
opcional; y métodos por defecto en protocolos (estilo *traits* de Rust / *protocol extensions* de
Swift).

---

## 13. Conclusión

HULKForge implementa HULK hasta el chequeo de tipos (A.8, obligatorio) más los dos *extras* elegidos
—**inferencia de tipos (A.9)** y **protocolos (A.10)**—, los features de **iterables (A.11)** y
**vectores (A.12)**, y **tres extensiones propias** (sobrecarga de operadores, asignación compuesta e
interpolación de cadenas). El compilador respeta el contrato de interfaz del evaluador y produce un
`./output` nativo mediante **transpilación a C**.

Las decisiones de diseño que más nos definen —transpilar a C en vez de construir una VM, usar tipado
estructural con dispatch dinámico uniforme, e implementar las extensiones como desazúcar sobre
maquinaria existente— comparten un mismo criterio: **reutilizar mecanismos sólidos en lugar de
multiplicar casos especiales**. Esa coherencia es lo que permite que features aparentemente
independientes (inferencia, protocolos, iterables, vectores, operadores) encajen unos sobre otros con
poco código y poco riesgo.

---

### Apéndice: módulos relevantes

- `src/lexer/lexer.rs` — tokenización con `logos`, `LineIndex`, `TokenStream`.
- `src/parser/parser.rs`, `src/parser/ast.rs` — parser de descenso recursivo + AST.
- `src/semantic/checker.rs`, `src/semantic/context.rs` — dos pasadas, inferencia, conformidad.
- `src/codegen.rs` — backend de transpilación a C (runtime, vtables, layout).
- `src/main.rs` — *driver* del contrato (gating, exit codes, emisión de `./output`).
- `examples/` — programas de demostración de las extensiones.
