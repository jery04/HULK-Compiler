# Reporte del Compilador HULK: Arquitectura y Decisiones de Diseño

## 1. 🏗️ Visión General de la Arquitectura

Este documento presenta una descripción técnica del **compilador HULK**, sus decisiones de diseño, las características implementadas y las limitaciones actuales. El objetivo es exponer cómo las fases clásicas de un compilador —análisis léxico, análisis sintáctico, análisis semántico y generación de código— se han materializado en la implementación de este proyecto, justificando cada decisión con los principios teóricos aprendidos en la asignatura de Compilación.

La arquitectura del compilador sigue una estructura modular y por fases. El flujo de datos es el siguiente: el código fuente es consumido por el analizador léxico, el cual genera una secuencia de tokens como salida; estos tokens son consumidos por el parser para producir un árbol de derivación y, a partir de él, un Árbol de Sintaxis Abstracta (AST); el AST es analizado por el componente semántico que realiza la resolución de símbolos y la verificación de tipos usando una tabla de símbolos (contexto); finalmente, si no existen errores, se recorre el AST para producir una representación intermedia (IR) y después el código de salida (por ejemplo, bytecode o un formato de tres direcciones).

La modularidad se expresa en distintos módulos del repositorio: la organización en módulos incluye el lexer (lexer/), el parser (parser/) y el analizador semántico (semantic/), además de utilidades como struct_printer.rs (que imprime la estructura del AST con todos los detalles de sus componentes) y codegen.rs. Esta separación facilita pruebas unitarias por fase, mantenimiento y la posible sustitución o extensión de fases (por ejemplo, reemplazar el backend por LLVM).

Desde el punto de vista teórico, el compilador refleja la clásica pipeline de compilación: transformación del lenguaje fuente a tokens (modelo formal: lenguajes regulares), análisis sintáctico (lenguajes libres de contexto), análisis semántico (propiedades dependientes de contexto) y generación de código (representaciones intermedias y optimizaciones). En la práctica, cada una de estas fases se implementa como componentes intercambiables y con interfaces claras.

## 2. 🔍 Análisis Léxico

El lexer de HULK se encarga de transformar la secuencia de caracteres del programa fuente en una secuencia de tokens etiquetados con tipo y, cuando procede, información de posición (línea/columna). En la implementación, el lexer está contenido en el directorio `src/lexer/` y sigue un enfoque clásico basado en expresiones regulares y autómatas finitos.

Desde la perspectiva teórica, la especificación de tokens se define mediante expresiones regulares para categorías como identificadores, literales numéricos, literales de cadena, operadores y palabras reservadas. En la práctica, estas expresiones regulares se convierten en un Autómata Finito No-Determinista (NFA) utilizando la construcción de Thompson, y posteriormente se aplica la determinización por el algoritmo de subconjuntos para obtener un DFA eficiente para el análisis en tiempo lineal. Esta estrategia justifica la implementación porque los DFA permiten escanear la entrada en una única pasada, en O(n) respecto al tamaño del texto.

La implementación considera la prioridad de tokens: por ejemplo, una palabra reservada como `if` debe ser reconocida como token distinto y de mayor prioridad frente a un token genérico `IDENTIFIER`. Esto se gestiona mediante una tabla de palabras reservadas que transforma lexemas coincidentes con el patrón de identificador en tokens de palabra reservada, o mediante prioridades definidas en la lista de expresiones regulares, tomando el match más largo y, en empates, la prioridad establecida por diseño.

Para la recuperación de errores léxicos, el lexer identifica secuencias no reconocidas y las describe con mensajes que incluyen contexto de línea y columna. La recuperación es conservadora: cuando se detecta un lexema inválido, se reporta un error léxico y el lexer intenta avanzar hasta un separador (espacio, fin de línea o símbolo conocido) para reanudar el escaneo. Esta política permite reportar múltiples errores léxicos en una sola ejecución sin colapsar en el primer fallo.

> NOTA:  Si bien es conocido que el tratamiento de errores léxicos —tales como identificadores mal formados (p. ej., _x, x+y, some method, 8ball)— podría abordarse durante la fase de análisis lexicográfico, se ha optado en su lugar por delegar esta responsabilidad al análisis semántico, aprovechando la estructura jerárquica del AST y la información contextual disponible en esta fase, lo que permite una identificación más sencilla y robusta de este tipo de anomalías.

En la práctica (y como se refleja en `src/lexer/test.rs`), hay pruebas unitarias que verifican la tokenización de ejemplos de `examples/` para garantizar que literales, operadores y palabras reservadas se reconozcan correctamente, así como que el manejo de comentarios y espacios en blanco sea el esperado.

## 3. 📜 Análisis Sintáctico (Parser)

El parser transforma la secuencia de tokens en un árbol de derivación que luego se abstrae en un AST aplicando las reglas de la gramática. En el repositorio, el parser vive en `src/parser/` y está acompañado de pruebas en `parser/tests.rs` que validan la correcta interpretación de constructos del lenguaje HULK.

>  NOTA: Se invita al lector a consultar el módulo `src/parser/ast.rs`, el cual contiene de forma organizada toda la definición del AST (Abstract Syntax Tree).

La estrategia de parsing adoptada es un parser recursivo descendente con predicción (estilo LL) adaptado a la gramática del lenguaje. Durante el diseño de la gramática se aplicaron transformaciones prácticas: eliminación de recursión izquierda para evitar llamadas infinitas en un parser top-down, y factorización a la izquierda cuando varias producciones compartían prefijos, de modo que las decisiones pasen a basarse en un único token de lookahead. En la práctica, esto se tradujo en reescrituras de reglas y en la inclusión de nodos intermedios en el AST para normalizar construcciones.

Para fundamentar la predictibilidad del parser se calcularon los conjuntos FIRST y FOLLOW de las producciones relevantes; estos conjuntos guían la selección de producción y sirven para construir una tabla de parsing conceptual usada por las funciones recursivas. En la implementación, las funciones del parser reflejan estas decisiones: antes de consumir un token se comprueba su pertenencia a `FIRST(p)` o, en caso de epsilon, a `FOLLOW(p)`; esta lógica es la traducción directa de la teoría a código.

Cuando la gramática planteó ambigüedades (por ejemplo, expresiones con diferente prioridad o construcciones con parseo no determinista), la estrategia fue desambiguación por prioridades y la reestructuración de las reglas para evitar conflictos. Esta aproximación es equivalente a introducir precedencias y asociatividades en un parser LR, pero aquí se resolvió por diseño de la gramática para mantener la simplicidad del parser LL.

En términos de diagnóstico, el parser aporta mensajes claros indicando token esperado y token recibido, y emplea sincronización básica (salto hasta `;`, `}` o tokens de nivel superior) para intentar continuar el análisis y detectar múltiples errores en una sola ejecución — una técnica de recuperación que equilibra precisión con resiliencia.

Ejemplo práctico: la regla de expresiones aritméticas fue refactorizada para evitar recursión izquierda transformando las producciones en formas iterativas con bucles en el código del parser, garantizando la correcta aplicación de la asociatividad y precedencia de operadores.

## 4. 🌳 Construcción del AST (Árbol de Sintaxis Abstracta)

Tras la fase sintáctica, y justo un paso después de esta, el compilador construye un AST: una representación estructurada y minimalista de la semántica del programa. En el código, la construcción del AST se realiza en `parser/ast.rs` y `parser/parser.rs`, donde las funciones del parser devuelven nodos del AST en lugar de árboles de derivación completos.

En el archivo `parser.rs`, específicamente encabezando el script, se encuentra un conjunto de métodos esenciales que determinan el conjunto de operaciones a seguir durante la secuenciación del listado de tokens. Estos métodos constituyen el núcleo del análisis sintáctico descendente recursivo, implementando la lógica de navegación, verificación y consumo de tokens que permite la construcción incremental del árbol sintáctico abstracto (AST). La arquitectura del parser se fundamenta en una máquina de estados que, mediante estrategias de anticipación y recuperación de errores, garantiza la robustez del proceso de parsing incluso ante entradas sintácticamente incorrectas. Posteriormente, se encuentran las implementaciones de parseo correspondientes a cada una de las estructuras gramaticales definidas en la especificación formal del lenguaje HULK, abarcando desde las expresiones primarias y operaciones binarias hasta las construcciones de control de flujo, declaraciones y demás elementos sintácticos que conforman la totalidad del lenguaje. Cada una de estas implementaciones sigue fielmente las reglas de producción establecidas en la gramática, garantizando así la correcta correspondencia entre la secuencia de tokens de entrada y la representación semántica resultante en el AST.

Teóricamente, el AST elimina información sintáctica irrelevante (como paréntesis redundantes, nodos intermedios de la gramática o símbolos de puntuación) para mantener únicamente la estructura semántica significativa: declaraciones, expresiones, llamadas a función, bloques y criterios de control. Esta compacidad facilita el análisis semántico y la optimización posterior porque reduce el árbol a los elementos que realmente afectan al comportamiento del programa.

En la práctica, la conversión del árbol de derivación al AST se realiza mediante acciones semánticas integradas en las reglas del parser: al reconocer una producción, el código crea el nodo AST correspondiente, conectando hijos relevantes y normalizando construcciones sintácticas en nodos semánticamente relevantes. Por ejemplo, una expresión aritmética se representa como un nodo binario con subnodos para los operandos y un enumerador para el operador, en lugar de nodos para operadores y paréntesis por separado.

El diseño del AST facilita posteriormente recorridos en distintos órdenes (pre-order para análisis sintáctico-semántico, post-order para generación de código), y permite adjuntar metadatos a los nodos (tipo inferido, información de posición, información de evaluación constante) que sirven en las siguientes fases.

## 5. 🧠 Análisis Semántico y Gestión de Contexto

El análisis semántico valida las propiedades dependientes del contexto que la gramática no puede garantizar. En HULK, esta fase incluye resolución de símbolos, verificación de tipos, y la comprobación de reglas de declaración y alcance. La implementación se organiza en `semantic/` con componentes para el contexto (`context.rs`) y el checker (`checker.rs`).

Desde la teoría, estas comprobaciones se modelan mediante una tabla de símbolos (o pila de tablas) que representa los ámbitos léxicos: al entrar en un bloque se crea un nuevo entorno, y al salir se restaura el anterior. Esta estructura permite verificar que cada referencia a una variable o función tenga una definición visible en el ámbito correspondiente y que las reglas de sombra (shadowing) y visibilidad sean respetadas.

La resolución de símbolos en HULK consiste en insertar las definiciones (de variables, parámetros, funciones) en el contexto durante el análisis de declaraciones y en buscar esas definiciones en la pila de contextos en tiempo de uso. Para funciones, se registra su firma (nombre, tipos de parámetros y tipo de retorno) y se comprueba que las llamadas respeten la aridad y tipos esperados.

La verificación de tipos se apoya en reglas formales de compatibilidad: cada operación tiene un conjunto de tipos válidos para sus operandos y un tipo resultado. El checker recorre el AST y, empleando un algoritmo de inferencia local y comprobaciones explícitas, asigna tipos a expresiones y reporta inconsistencias. En caso de operaciones entre tipos incompatibles (por ejemplo, sumar un `String` y un `Number` sin conversión explícita), se emite un error semántico con localización.

HULK soporta características avanzadas como funciones recursivas o sobrecarga básica. Para ello, la infraestructura de la tabla de símbolos y el sistema de verificación de firmas se encuentran capacitados para manejar dichos casos: las firmas se registran antes del chequeo de cuerpos de funciones para permitir llamadas recursivas, y la comprobación de sobrecarga usa mecanismos de selección de firma según tipos de argumentos.

> NOTA: Si bien la especificación del lenguaje HULK establece explícitamente en la Sección A.3.1 que "no hay sobrecargas en HULK" —dado que todas las funciones residen en un único espacio de nombres global y no se permite repetir identificadores—, el propio documento añade la salvedad de que esta restricción aplica "al menos en el HULK 'básico'". Por tanto, nos propusimos extender el lenguaje incorporando sobrecarga de métodos y funciones como un mecanismo legítimo de polimorfismo

### Comprobaciones semánticas (errores detectados)

El análisis semántico incorpora múltiples comprobaciones que detectan errores dependientes del contexto. A continuación se detallan las principales, con una breve explicación de cada una:

- **Variables no inicializadas:** Verifica el uso de variables antes de asignarles un valor, evitando lecturas indefinidas en tiempo de ejecución.
- **Identificadores no declarados:** Detecta referencias a variables o funciones que no existen en el ámbito visible.
- **Redefiniciones / shadowing inválido:** Señala declaraciones que colisionan con identificadores ya existentes cuando la política del lenguaje lo prohíbe.
- **Incompatibilidad de tipos en asignaciones y expresiones:** Comprueba que el tipo del valor asignado o de los operandos en una operación sea compatible con el tipo esperado.
- **Llamadas a funciones con aridad/firmas incorrectas:** Verifica que el número y tipos de argumentos en una llamada coincidan con la firma registrada de la función.
- **Retornos en funciones:** Asegura que todas las rutas de ejecución en una función devuelvan el tipo declarado (o que las funciones `void` no retornen valor), evitando inconsistencias de tipo en retornos.
- **Comprobación de tipos en estructuras de control:** Verifica que la condición en `if`, `elif` y `while` sea siempre de tipo `Boolean`. Por ejemplo, `if (x > 0)` es válido porque `x > 0` devuelve un booleano, pero `if (42)` es inválido porque `42` es un número. Esto previene errores lógicos y asegura que el programa sea claro y predecible.
- **Operaciones no válidas entre tipos:** Detecta operaciones entre tipos que no tienen semántica definida (p. ej. sumar `String` y `Number` sin conversión explícita).
- **Conversión/coerción insegura:** Señala coerciones implícitas problemáticas o pérdidas de precisión cuando existen conversiones entre tipos incompatibles.
- **Acceso a miembros inexistentes en objetos:** Esta comprobación verifica que cuando accedes a un miembro de un objeto (usando el operador `.`), ese miembro (método) exista realmente en el tipo del objeto (ya sea propio o heredado).
- **Comprobación de expresiones constantes y errores detectables en tiempo de compilación:** Detecta divisiones por cero constantes, accesos fuera de rango estáticos y otras anomalías evaluables en compilación.

- **Protocolos e implementación estructural:** Soporte para declarar `protocols` con herencia entre protocolos y comprobación de que un `type` implementa los métodos requeridos por un `protocol` (incluye comprobación de compatibilidad de firmas por nombre y aridad).
- **Verificación de herencia de tipos y detección de ciclos:** Comprobación de que el `parent` de un `type` exista, que los argumentos genéricos coincidan con la aridad esperada y detección de ciclos de herencia que producirían inconsistencias.
- **Comprobación de overrides en métodos heredados:** Las sobrescrituras de métodos en subtipos deben conservar la firma exacta del método heredado; se reportan errores cuando la firma no coincide (parámetros o tipo de retorno).
- **Compatibilidad subtipada y comprobación LCA:** Determinación de subtipado por la cadena de padres; cálculo del "lowest common ancestor" para deducir tipos resultantes en expresiones condicionales multi-rama.
- **Soporte de type parameters (aritmetrica de parámetros):** Registro y verificación del número de parámetros de tipo esperados en una declaración `type` y en cláusulas `inherits` al instanciar tipos genéricos.
- **Protocol conformance entre protocolos:** Verificación de que un `protocol` concreto conforma a otro `protocol` esperado (incluye herencia de protocolos y compatibilidad de firmas) es decir, si un `type` que implementa el protocolo A puede ser usado en todos los lugares donde se espera el protocolo B.
- **Comprobación de firmas y compatibilidad de llamadas:** Registro de firmas de funciones, métodos y builtins; comprobación de aridad y tipos de argumentos en llamadas y en llamadas a métodos/builtins.
- **Iterable/Enumerable y verificación de `for` loops:** Protocolos `Iterable` / `Enumerable` integrados; el análisis decide si una expresión es iterable, marca usos iterables y registra el tipo del elemento iterado para propagar tipos en el cuerpo del bucle.
- **Inferencia conservadora de tipos simples:** Inferencia local y conservadora para `Number`, `String`, `Boolean` y tipos nombrados; inferencia de tipos de retorno de funciones/bodies cuando es posible y actualización de firmas registradas.
- **Restricciones en `self` y `base`:** Validación del ámbito de `self` restringido exclusivamente al contexto de métodos de instancia, incluyendo sus reglas de uso y limitaciones; `base` solo dentro de métodos de tipos con padre y verificación de llamadas a la implementación del padre con firma compatible.
- **Control de asignaciones e inferencia en `let` bindings:** Verificación de tipos en inicializadores; si existe anotación, se comprueba conformidad; si no, se infiere y se propaga a la variable en el scope adecuado (incluye advertencias cuando la inferencia detecta incompatibilidades transitorias).

Estas comprobaciones reflejan la distinción teórica entre propiedades sintácticas (libres de contexto) y propiedades semánticas (dependientes del contexto), y permiten detectar errores que solo son visibles una vez que se dispone de información de ámbito y tipos.

## 6. 🧬 Sistema de Tipos (Type System)

El lenguaje HULK ofrece un conjunto de tipos primitivos y compuestos básicos: Number, Boolean y String como tipos primitivos. Además, existen tipos `Null`/`Void` para funciones sin retorno.

Las reglas de compatibilidad se definen explícitamente: operaciones aritméticas (`+`, `-`, `*`, `/`) requieren operandos `Number` y producen `Number`; las operaciones lógicas (`&&`, `||`, `!`) requieren `Boolean`. Cuando se permite coerción implícita (por ejemplo, entre `Number` y `String`), estas reglas se declaran en el checker y generan advertencias cuando la coerción es potencialmente ambigua.

Desde la teoría, el sistema de tipos se describe como un conjunto de reglas de inferencia y verificación que asignan tipos a expresiones: en términos formales, se usan reglas de deducción tipo-judgment (Γ ⊢ e : τ) donde Γ es el contexto. La implementación traduce estas reglas a comprobaciones programáticas que recorren el AST: para cada nodo, se calculan los tipos de sus hijos y se aplica la regla correspondiente para derivar el tipo del nodo padre o emitir un error si no existe una regla que lo permita.

El compilador aplica una inferencia local de tipos, suficiente para inferrir tipos a partir de literales y expresiones compuestas. Para funciones, la comprobación de firmas exige que los tipos de argumentos coincidan con los parámetros, y que el cuerpo de la función retorne el tipo declarado.

Los errores de tipo se reportan con ubicación y una explicación del conflicto — por ejemplo, “se esperaba `Number` pero se encontró `String` en la suma”. Esta precisión es crucial para el desarrollo y depuración de programas en HULK.

## 7. 💻 Generación de Código y Optimización

La fase de generación de código transforma el AST tipado en una representación intermedia (IR) y luego en el código de salida. En este proyecto, `codegen.rs` y `evaluator/` contienen lógica para transformar expresiones y declaraciones en instrucciones ejecutables. La IR adoptada es una forma de código de tres direcciones o instrucciones simples que representan operaciones atómicas (cargar, almacenar, operar, saltar), lo cual es coherente con la práctica académica de usar una IR lineal y fácil de optimizar.

La generación recorre el AST en post-order (orden post-orden) para garantizar que las subexpresiones se traduzcan antes que su operador padre. Esta estrategia es la recomendada teóricamente para generar código de manera que los valores intermedios estén disponibles cuando se emiten las instrucciones de la operación que los consume.

Respecto a la independencia de la arquitectura, la IR actúa como una capa intermedia: las transformaciones y optimizaciones se aplican sobre la IR sin considerar detalles de la máquina destino, lo cual permite exportar múltiples backends (por ejemplo, un backend que emita bytecode para una VM propia, o un backend que genere LLVM IR o C intermedio).

En cuanto a optimizaciones, la implementación incluye (o está preparada para incluir) técnicas clásicas: propagación de constantes, evaluación de expresiones constantes, y eliminación de subexpresiones comunes. Estas optimizaciones se implementan como pases sobre la IR o sobre el AST antes de la generación final. Por ejemplo, si una expresión se reduce a una constante, se sustituye por el literal correspondiente y se evita generar código innecesario.

Otras optimizaciones más avanzadas (reordenamiento de instrucciones, asignación de registros basada en grafos de interferencia, optimizaciones intraprocedurales más agresivas) no están completamente desarrolladas pero la arquitectura del proyecto permite añadir estos pases como transformaciones independientes sobre la IR.

Finalmente, `evaluator/tests.rs` y el módulo `evaluator` permiten ejecutar programas de HULK en un entorno de prueba (interpretativo o de ejecución del IR), lo cual facilita la verificación de la corrección del codegen y las optimizaciones.

## 8. 📐 Análisis del Lenguaje HULK y Diseño de Extensiones

Esta sección analiza con mayor profundidad el diseño del lenguaje HULK, su sistema de tipos y las extensiones propuestas. Se ofrecen comparativas con lenguajes de referencia y justificaciones teóricas para cada decisión de diseño.

### 1) Sistema de Tipos de HULK

HULK (Havana University Language for Kompilers) es un lenguaje de programación didáctico, *type-safe*, orientado a objetos e incremental, diseñado para el curso de Introducción a los Compiladores en la Universidad de La Habana. Su diseño prioriza la seguridad de tipos y la claridad, permitiendo una curva de aprendizaje gradual para nosotros (el estudiantado).

**Clasificación del Sistema de Tipos**

El sistema de tipos de HULK se clasifica como:

*   **Estático y Fuerte**: Los tipos se verifican en tiempo de compilación, y las operaciones no realizan coerciones implícitas inseguras. Esto se alinea con la filosofía de los lenguajes C# y Java, ofreciendo garantías de seguridad tempranas (reducción de errores en tiempo de ejecución).
*   **Con Inferencia de Tipos Opcional**: HULK permite anotar tipos explícitamente, pero también puede inferirlos en la mayoría de los casos, simplificando la escritura de código. Esta característica didáctica permite a los estudiantes implementar primero un evaluador y luego abordar la verificación de tipos.
*   **Nominal y Estructural**: La jerarquía de clases se basa en *tipado nominal*, donde la conformidad se define por herencia. Sin embargo, los **protocolos** introducen *tipado estructural*, permitiendo que un tipo implemente un protocolo implícitamente si tiene los métodos con las firmas adecuadas.

**Comparativa con otros lenguajes**

| Característica | HULK | C# / Java | Python / JavaScript |
| :--- | :--- | :--- | :--- |
| **Tipado** | Estático con inferencia | Estático y fuerte | Dinámico y débil/mixto |
| **Verificación** | Compilación | Compilación | Ejecución |
| **Complejidad** | Baja (educativo) | Alta | Media |
| **Seguridad** | Alta | Alta | Baja (errores tardíos) |

HULK se sitúa más cerca de C#/Java en cuanto a verificación temprana, pero con menos complejidad en su sistema de inferencia. Frente a JavaScript, evita errores de tipo tardíos a costa de mayor verbosidad en algunas anotaciones. Frente a Java, HULK carece de genéricos (aunque se proponen extensiones) y de un sistema de subtipado robusto como el de C# con sus variantes de varianza.

### 2) Extensiones Implementadas y Justificación

Las extensiones propuestas para HULK se han diseñado siguiendo un camino incremental para no sobrecargar la implementación base.

1.  **Protocolos (Structural Typing)**
    *   **Descripción**: Los protocolos permiten definir un conjunto de métodos que un tipo debe implementar. A diferencia de la herencia, la conformidad es implícita.
    *   **Justificación**: Aportan una gran flexibilidad, permitiendo escribir código polimórfico que funciona con cualquier tipo que tenga ciertas capacidades, sin necesidad de que esos tipos hereden de una clase común. Esto es fundamental para funcionalidades como los iterables (`Iterable`).
    *   **Fundamento Teórico**: Implementa la noción de *tipado estructural*, contrastando con el *tipado nominal* de las clases. La regla de conformidad con un protocolo es: `T <= P` si `T` tiene todos los métodos de `P` con tipos que respeten la varianza (los argumentos pueden ser de un tipo más general y el retorno de uno más específico).

2.  **Iterables y Vectores**
    *   **Descripción**: Se añade un protocolo `Iterable` (con métodos `next()` y `current()`). El método `next()` avanza el iterador y devuelve un `Boolean` indicando si existe un elemento siguiente; `current()` devuelve el elemento actual. El tipo `Vector` (array) implementa este protocolo. El bucle `for` se transpila a código que usa este protocolo.
    *   **Justificación**: Permite manejar colecciones de datos de forma uniforme y eficiente, una necesidad básica en cualquier lenguaje de programación.
    *   **Fundamento Teórico**: Introduce el concepto de *transpilación* para funcionalidades de alto nivel. El bucle `for` no es una construcción primitiva, sino *azúcar sintáctico* que se traduce a un bucle `while` estándar. Los vectores, con su sintaxis `T[]`, son un tipo genérico especial cuyo compilador genera un protocolo específico para el tipo de elemento.

3.  **Inferencia de Tipos (Type Inference)**
    *   **Descripción**: El compilador de Hulk incorpora un sistema de inferencia de tipos que permite al programador omitir las anotaciones de tipo en variables, parámetros y retornos de funciones, siempre que el contexto sea suficientemente explícito. El compilador deduce automáticamente el tipo más general posible basándose en el uso y las operaciones realizadas sobre los valores.
    *   **Justificación**: Mejora significativamente la ergonomía del lenguaje, reduciendo la verbosidad y la carga cognitiva del programador, especialmente en código con tipos complejos o genéricos. Permite escribir código más conciso y mantenible sin sacrificar la seguridad de tipos.
    *   **Fundamento Teórico**: Aunque no constituye una extensión gramatical del lenguaje (la sintaxis permanece inalterada), sí representa una extensión semántica e interna al compilador. Se basa en el algoritmo de *unificación* de Hindley-Milner (o una variante adaptada), que recorre el AST (Árbol de Sintaxis Abstracta) recogiendo restricciones de tipos y resolviéndolas para asignar tipos a todas las expresiones. Esto implica que el compilador debe realizar un análisis de tipos en dos fases: una primera de recolección de restricciones y una segunda de sustitución, permitiendo que incluso el bucle `for` y las operaciones con `Vector` e `Iterable` infieran los tipos correctos de los elementos sin necesidad de anotaciones explícitas.


### 3) Análisis de Decisiones de Diseño

Las decisiones de diseño de HULK se basan en criterios pedagógicos y técnicos: **simplicidad, predecibilidad y seguridad**.

*   **Simplicidad del Sistema de Tipos**: Elegir un sistema de tipos estático con inferencia local (no Hindley-Milner) facilita la enseñanza de reglas de tipado (Γ ⊢ e : τ) sin abrumar con metaprogramación de tipos.
*   **Organización de la Memoria**: Aunque la implementación del compilador de HULK gestiona la memoria para el código intermedio, su modelo conceptual se asemeja a los lenguajes con pila y heap para valores dinámicos (strings, objetos), reflejando la distinción teórica entre duración automática y dinámica.
*   **Procesamiento y Transpilación**: Se prefiere un enfoque compilado a un IR (Intermediate Representation) con posibilidad de ejecución interpretada para pruebas. El uso extensivo de la transpilación (para `for`, `funtores`, `T[]`) permite añadir funcionalidades complejas sin modificar el núcleo del evaluador.
*   **Varianza y Seguridad de Tipos**: Los protocolos de HULK respetan la varianza en su implementación (los argumentos son contravariantes, los retornos covariantes). Para extensiones futuras como genéricos, se propone un enfoque similar a C# (con anotaciones `in`/`out`) para garantizar la seguridad de tipos en asignaciones y llamadas polimórficas.

### 4) Comparativa con Otros Lenguajes

HULK se diseña como un lenguaje educativo, priorizando la claridad y la seguridad sobre la máxima expresividad o el rendimiento.

*   **Frente a Python**: Ofrece mayor seguridad de tipo gracias a su verificación estática. Sin embargo, la sintaxis de HULK es más verbosa y su ecosistema de bibliotecas es casi inexistente.
*   **Frente a Java**: Ofrece una experiencia de desarrollo más ligera y rápida para experimentar, con características como la inferencia de tipos y funciones de primera clase sin la necesidad de definir interfaces explícitas para cada caso.
*   **Frente a JavaScript**: Evita coerciones implícitas que causan errores sutiles en tiempo de ejecución.

La elección de características (tipado estático parcial, funciones de primera clase, soporte para recursividad, protocolos) busca un equilibrio entre paradigmas imperativo y funcional. Esto facilita la transición del estudiante entre modelos teóricos y prácticos, preparándolo para aprender lenguajes más complejos.

**Conclusión de la sección**: El diseño de HULK prioriza enseñar y aplicar principios de lenguajes (tipos, ámbitos, ejecución en pila) con extensiones que permiten explorar paradigmas funcionales y genéricos sin introducir complejidad excesiva. Las extensiones propuestas siguen un camino incremental: primero garantizar corrección semántica (con el sistema de tipos y protocolos) y luego añadir expresividad (funtores, lambdas, genéricos).

## 9. 🧪 Pruebas y validación incremental

Durante el desarrollo del compilador hemos seguido una estrategia de pruebas por secciones que ha sido fundamental para garantizar la corrección y la evolución ordenada del proyecto. Cada vez que se implementaba una nueva estructura sintáctica o semántica, se añadían y ejecutaban varios tests unitarios y de integración focalizados en esa funcionalidad. A continuación se resumen los beneficios principales de este enfoque:

- **Validación puntual:** Los tests por sección permitieron verificar inmediatamente el comportamiento esperado de la nueva característica sin depender de cambios posteriores.
- **Regresión controlada:** Tras cada incorporación comprobábamos que el conjunto completo de tests existentes continuaba pasando, lo que nos aseguraba que las modificaciones no rompieran funcionalidades previas.
- **Retroalimentación rápida:** El ciclo corto de escribir tests, implementar y ejecutar redujo el tiempo de depuración y facilitó localizar la causa de fallos en módulos concretos.
- **Documentación ejecutable:** Los tests actúan como especificación viva: describen casos de uso reales y sirven como referencia al extender o refactorizar componentes del compilador.
- **Confianza para refactorizaciones:** Al disponer de una batería de tests amplia y segmentada, pudimos realizar refactorizaciones y mejoras internas con mayor seguridad, sabiendo que los tests detectarían cualquier ruptura funcional.

En resumen, la práctica sistemática de desarrollar tests junto con cada nueva pieza del compilador fue decisiva para mantener la estabilidad del proyecto y acelerar el progreso técnico de manera disciplinada y reproducible.

## 10. 🚧 Limitaciones y Trabajo Futuro

Limitaciones actuales:

- Cobertura del sistema de tipos: el compilador aplica inferencia local y verifica firmas, pero no implementa un sistema de inferencia completo tipo Hindley–Milner ni tipado polimórfico avanzado. Esto limita la expresividad en cuanto a genéricos y inferencia automática en contextos complejos.
- Optimización limitada: sólo se han implementado optimizaciones básicas (evaluación de constantes y propagación). Pases más potentes como la eliminación de código muerto, la reducción por SSA o la asignación de registros óptima no están completos.
- Soporte de runtime y biblioteca estándar: la biblioteca estándar de HULK es mínima; funciones I/O, manejo avanzado de errores en tiempo de ejecución y operaciones de E/S necesitan ampliación.
- Análisis de flujo de control avanzado: los análisis de flujo de datos y control (liveness, reaching definitions) aún no están integrados, lo que limita optimizaciones interprocedurales.

Trabajo futuro recomendado:

- Implementar un sistema de tipos polimórfico o soporte para genéricos para aumentar la expresividad del lenguaje.
- Añadir un backend basado en LLVM IR para conseguir generación de código optimizada y aprovechar optimizaciones maduras y asignación de registros.
- Incorporar pases de optimización adicionales: eliminación de código muerto, SSA, y optimización basada en análisis de flujo.
- Mejorar la biblioteca estándar y el runtime para incluir manejo de errores, I/O asíncrono y colecciones avanzadas.
- Diseñar e implementar un sistema de módulos y gestión de dependencias para proyectos HULK de mayor escala.

## 11. 📝 Conclusión

El compilador HULK es una implementación práctica y educativa que materializa los conceptos fundamentales de la asignatura de Compilación: desde la transformación de expresiones regulares a autómatas finitos para el análisis léxico, la estructuración de la gramática para permitir un parser predictivo, hasta la construcción de un AST y un flujo semántico que valida propiedades dependientes del contexto mediante una tabla de símbolos. La generación de una representación intermedia y la preparación para optimizaciones muestran una comprensión de la separación entre análisis y generación que exige el diseño de compiladores modernos.

Las decisiones de diseño han favorecido la claridad, la modularidad y la facilidad de extensión, siguiendo principios teóricos (separación de fases, representaciones intermedias independientes de la máquina, y análisis por pasos). En términos pedagógicos, el proyecto proporciona una base sólida para experimentar con mejoras: incorporar un sistema de tipos más potente, añadir optimizaciones avanzadas y conectar con backends consolidados como LLVM.

En síntesis, HULK es tanto una herramienta de aprendizaje como una base de ingeniería: permite comprobar de forma práctica cómo los modelos formales y las transformaciones teóricas —autómatas, gramáticas, tablas de símbolos, reglas de inferencia de tipos y recorridos de árboles— se traducen en un producto de software funcional. Las limitaciones actuales señalan un camino claro de evolución que, si se aborda, ofrecerá un compilador con mayor expresividad, rendimiento y madurez industrial.

---

**Archivos y módulos relevantes:**

- `lexer/lexer.rs` y `lexer/test.rs` — implementación y pruebas del análisis léxico.
- `parser/parser.rs`, `parser/ast.rs`, `parser/tests.rs` — parser y construcción del AST.
- `semantic/context.rs`, `semantic/checker.rs` — tabla de símbolos y verificación semántica.
- `codegen.rs`, `evaluator/` — generación de código y evaluación/ejecución de la representación intermedia.

Si desea, puedo:

- Ejecutar la suite de tests del proyecto y adjuntar un resumen de fallos y cobertura.
- Añadir secciones específicas en el reporte sobre un módulo concreto (por ejemplo, detallar la estructura de `context.rs`).
- Generar diagramas de flujo de datos o mermaid para ilustrar la pipeline del compilador.

Indíqueme cuál de estas acciones prefiere y lo haré a continuación.
