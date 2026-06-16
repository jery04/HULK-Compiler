# Reporte del Compilador HULK: Arquitectura y Decisiones de Diseño

## 1. 🏗️ Visión General de la Arquitectura

Este documento presenta una descripción técnica del **compilador HULK**, sus decisiones de diseño, las características implementadas y las limitaciones actuales. El objetivo es exponer cómo las fases clásicas de un compilador —análisis léxico, análisis sintáctico, análisis semántico y generación de código— se han materializado en la implementación de este proyecto, justificando cada decisión con los principios teóricos aprendidos en la asignatura de Compilación.

La arquitectura del compilador sigue una estructura modular y por fases. El flujo de datos es el siguiente: el código fuente se alimenta al lexer, que produce una secuencia de tokens; estos tokens son consumidos por el parser para producir un árbol de derivación y, a partir de él, un Árbol de Sintaxis Abstracta (AST); el AST es analizado por el componente semántico que realiza la resolución de símbolos y la verificación de tipos usando una tabla de símbolos (contexto); finalmente, si no existen errores, se recorre el AST para producir una representación intermedia (IR) y después el código de salida (por ejemplo, bytecode o un formato de tres direcciones).

La modularidad se expresa en distintos módulos del repositorio: un módulo para el lexer (`lexer/`), otro para el parser (`parser/`), módulos para `semantic` y `evaluator` (`semantic/`, `evaluator/`), y utilidades como `struct_printer.rs` y `codegen.rs`. Esta separación facilita pruebas unitarias por fase, mantenimiento y la posible sustitución o extensión de fases (por ejemplo, reemplazar el backend por LLVM).

Desde el punto de vista teórico, el compilador refleja la clásica pipeline de compilación: transformación del lenguaje fuente a tokens (modelo formal: lenguajes regulares), análisis sintáctico (lenguajes libres de contexto), análisis semántico (propiedades dependientes de contexto) y generación de código (representaciones intermedias y optimizaciones). En la práctica, cada una de estas fases se implementa como componentes intercambiables y con interfaces claras.

## 2. 🔍 Análisis Léxico

El lexer de HULK se encarga de transformar la secuencia de caracteres del programa fuente en una secuencia de tokens etiquetados con tipo y, cuando procede, información de posición (línea/columna). En la implementación, el lexer está contenido en el directorio `lexer/` y sigue un enfoque clásico basado en expresiones regulares y autómatas finitos.

Desde la perspectiva teórica, la especificación de tokens se define mediante expresiones regulares para categorías como identificadores, literales numéricos, literales de cadena, operadores y palabras reservadas. En la práctica, estas expresiones regulares se convierten en un Autómata Finito No-Determinista (NFA) utilizando la construcción de Thompson, y posteriormente se aplica la determinización por el algoritmo de subconjuntos para obtener un DFA eficiente para el análisis en tiempo lineal. Esta estrategia justifica la implementación porque los DFA permiten escanear la entrada en una única pasada, en O(n) respecto al tamaño del texto.

La implementación considera la prioridad de tokens: por ejemplo, una palabra reservada como `if` debe ser reconocida como token distinto y de mayor prioridad frente a un token genérico `IDENTIFIER`. Esto se gestiona mediante una tabla de palabras reservadas que transforma lexemas coincidentes con el patrón de identificador en tokens de palabra reservada, o mediante prioridades definidas en la lista de expresiones regulares, tomando el match más largo y, en empates, la prioridad establecida por diseño.

Para la recuperación de errores léxicos, el lexer identifica secuencias no reconocidas y las describe con mensajes que incluyen contexto de línea y columna. La recuperación es conservadora: cuando se detecta un lexema inválido, se reporta un error léxico y el lexer intenta avanzar hasta un separador (espacio, fin de línea o símbolo conocido) para reanudar el escaneo. Esta política permite reportar múltiples errores léxicos en una sola ejecución sin colapsar en el primer fallo.

En la práctica (y como se refleja en `lexer/test.rs`), hay pruebas unitarias que verifican la tokenización de ejemplos de `examples/` para garantizar que literales, operadores y palabras reservadas se reconozcan correctamente, así como que el manejo de comentarios y espacios en blanco sea el esperado.

## 3. 📜 Análisis Sintáctico (Parser)

El parser transforma la secuencia de tokens en un árbol de derivación y, a continuación, en un AST. En el repositorio, el parser vive en `parser/` y está acompañado de pruebas en `parser/tests.rs` que validan la correcta interpretación de constructos del lenguaje HULK.

La estrategia de parsing adoptada es un parser recursivo descendente con predicción (estilo LL) adaptado a la gramática del lenguaje. Durante el diseño de la gramática se aplicaron transformaciones prácticas: eliminación de recursión izquierda para evitar llamadas infinitas en un parser top-down, y factorización a la izquierda cuando varias producciones compartían prefijos, de modo que las decisiones pasen a basarse en un único token de lookahead. En la práctica, esto se tradujo en reescrituras de reglas y en la inclusión de nodos intermedios en el AST para normalizar construcciones.

Para fundamentar la predictibilidad del parser se calcularon los conjuntos FIRST y FOLLOW de las producciones relevantes; estos conjuntos guían la selección de producción y sirven para construir una tabla de parsing conceptual usada por las funciones recursivas. En la implementación, las funciones del parser reflejan estas decisiones: antes de consumir un token se comprueba su pertenencia a `FIRST(p)` o, en caso de epsilon, a `FOLLOW(p)`; esta lógica es la traducción directa de la teoría a código.

Cuando la gramática planteó ambigüedades (por ejemplo, expresiones con diferente prioridad o construcciones con parseo no determinista), la estrategia fue desambiguación por prioridades y la reestructuración de las reglas para evitar conflictos. Esta aproximación es equivalente a introducir precedencias y asociatividades en un parser LR, pero aquí se resolvió por diseño de la gramática para mantener la simplicidad del parser LL.

En términos de diagnóstico, el parser aporta mensajes claros indicando token esperado y token recibido, y emplea sincronización básica (salto hasta `;`, `}` o tokens de nivel superior) para intentar continuar el análisis y detectar múltiples errores en una sola ejecución — una técnica de recuperación que equilibra precisión con resiliencia.

Ejemplo práctico: la regla de expresiones aritméticas fue refactorizada para evitar recursión izquierda transformando las producciones en formas iterativas con bucles en el código del parser, garantizando la correcta aplicación de la asociatividad y precedencia de operadores.

## 4. 🌳 Construcción del AST (Árbol de Sintaxis Abstracta)

Tras la fase sintáctica, el compilador construye un AST: una representación estructurada y minimalista de la semántica del programa. En el código, la construcción del AST se realiza en `parser/ast.rs` y `parser/parser.rs`, donde las funciones del parser devuelven nodos del AST en lugar de árboles de derivación completos.

Teóricamente, el AST elimina información sintáctica irrelevante (como paréntesis redundantes, nodos intermedios de la gramática o símbolos de puntuación) para mantener únicamente la estructura semántica significativa: declaraciones, expresiones, llamadas a función, bloques y criterios de control. Esta compacidad facilita el análisis semántico y la optimización posterior porque reduce el árbol a los elementos que realmente afectan al comportamiento del programa.

En la práctica, la conversión del árbol de derivación al AST se realiza mediante acciones semánticas integradas en las reglas del parser: al reconocer una producción, el código crea el nodo AST correspondiente, conectando hijos relevantes y normalizando construcciones sintácticas en nodos semánticamente relevantes. Por ejemplo, una expresión aritmética se representa como un nodo binario con subnodos para los operandos y un enumerador para el operador, en lugar de nodos para operadores y paréntesis por separado.

El diseño del AST facilita posteriormente recorridos en distintos órdenes (pre-order para análisis sintáctico-semántico, post-order para generación de código), y permite adjuntar metadatos a los nodos (tipo inferido, información de posición, información de evaluación constante) que sirven en las siguientes fases.

## 5. 🧠 Análisis Semántico y Gestión de Contexto

El análisis semántico valida las propiedades dependientes del contexto que la gramática no puede garantizar. En HULK, esta fase incluye resolución de símbolos, verificación de tipos, y la comprobación de reglas de declaración y alcance. La implementación se organiza en `semantic/` con componentes para el contexto (`context.rs`) y el checker (`checker.rs`).

Desde la teoría, estas comprobaciones se modelan mediante una tabla de símbolos (o pila de tablas) que representa los ámbitos léxicos: al entrar en un bloque se crea un nuevo entorno, y al salir se restaura el anterior. Esta estructura permite verificar que cada referencia a una variable o función tenga una definición visible en el ámbito correspondiente y que las reglas de sombra (shadowing) y visibilidad sean respetadas.

La resolución de símbolos en HULK consiste en insertar las definiciones (de variables, parámetros, funciones) en el contexto durante el análisis de declaraciones y en buscar esas definiciones en la pila de contextos en tiempo de uso. Para funciones, se registra su firma (nombre, tipos de parámetros y tipo de retorno) y se comprueba que las llamadas respeten la aridad y tipos esperados.

La verificación de tipos se apoya en reglas formales de compatibilidad: cada operación tiene un conjunto de tipos válidos para sus operandos y un tipo resultado. El checker recorre el AST y, empleando un algoritmo de inferencia local y comprobaciones explícitas, asigna tipos a expresiones y reporta inconsistencias. En caso de operaciones entre tipos incompatibles (por ejemplo, sumar un `String` y un `Number` sin conversión explícita), se emite un error semántico con localización.

Si HULK soporta características avanzadas como funciones recursivas o sobrecarga básica, la infraestructura de la tabla de símbolos y la verificación de firmas está preparada para manejarlas: las firmas se registran antes del chequeo de cuerpos de funciones para permitir llamadas recursivas, y la comprobación de sobrecarga (si existe) usa mecanismos de selección de firma según tipos de argumentos.

El análisis semántico también incorpora comprobaciones adicionales: variables no inicializadas, retornos en funciones (asegurando que todas las ramas devuelvan el tipo correcto), y verificación de tipos en estructuras de control (p. ej. condición de `if` debe ser booleano). Estas comprobaciones reflejan la distinción teórica entre propiedades sintácticas (libres de contexto) y propiedades semánticas (dependientes del contexto).

## 6. 🧬 Sistema de Tipos (Type System)

El lenguaje HULK ofrece un conjunto de tipos primitivos y compuestos básicos: Number, Boolean y String como tipos primitivos. Además, existen tipos derivados para arreglos (vectors) y posiblemente `Null`/`Void` para funciones sin retorno.

Las reglas de compatibilidad se definen explícitamente: operaciones aritméticas (`+`, `-`, `*`, `/`) requieren operandos `Number` y producen `Number`; las operaciones lógicas (`&&`, `||`, `!`) requieren `Boolean`; la concatenación puede estar definida para `String + String`. Cuando se permite coerción implícita (por ejemplo, entre `Number` y `String`), estas reglas se declaran en el checker y generan advertencias cuando la coerción es potencialmente ambigua.

Desde la teoría, el sistema de tipos se describe como un conjunto de reglas de inferencia y verificación que asignan tipos a expresiones: en términos formales, se usan reglas de deducción tipo-judgment (Γ ⊢ e : τ) donde Γ es el contexto. La implementación traduce estas reglas a comprobaciones programáticas que recorren el AST: para cada nodo, se calculan los tipos de sus hijos y se aplica la regla correspondiente para derivar el tipo del nodo padre o emitir un error si no existe una regla que lo permita.

El compilador aplica una inferencia local de tipos (no un sistema Hindley–Milner completo), suficiente para inferrir tipos a partir de literales y expresiones compuestas. Para funciones, la comprobación de firmas exige que los tipos de argumentos coincidan con los parámetros, y que el cuerpo de la función retorne el tipo declarado.

Los errores de tipo se reportan con ubicación y una explicación del conflicto — por ejemplo, “se esperaba `Number` pero se encontró `String` en la suma”. Esta precisión es crucial para el desarrollo y depuración de programas en HULK.

## 7. 💻 Generación de Código y Optimización

La fase de generación de código transforma el AST tipado en una representación intermedia (IR) y luego en el código de salida. En este proyecto, `codegen.rs` y `evaluator/` contienen lógica para transformar expresiones y declaraciones en instrucciones ejecutables. La IR adoptada es una forma de código de tres direcciones o instrucciones simples que representan operaciones atómicas (cargar, almacenar, operar, saltar), lo cual es coherente con la práctica académica de usar una IR lineal y fácil de optimizar.

La generación recorre el AST en post-order (orden post-orden) para garantizar que las subexpresiones se traduzcan antes que su operador padre. Esta estrategia es la recomendada teóricamente para generar código de manera que los valores intermedios estén disponibles cuando se emiten las instrucciones de la operación que los consume.

Respecto a la independencia de la arquitectura, la IR actúa como una capa intermedia: las transformaciones y optimizaciones se aplican sobre la IR sin considerar detalles de la máquina destino, lo cual permite exportar múltiples backends (por ejemplo, un backend que emita bytecode para una VM propia, o un backend que genere LLVM IR o C intermedio).

En cuanto a optimizaciones, la implementación incluye (o está preparada para incluir) técnicas clásicas: propagación de constantes, evaluación de expresiones constantes, y eliminación de subexpresiones comunes. Estas optimizaciones se implementan como pases sobre la IR o sobre el AST antes de la generación final. Por ejemplo, si una expresión se reduce a una constante, se sustituye por el literal correspondiente y se evita generar código innecesario.

Otras optimizaciones más avanzadas (reordenamiento de instrucciones, asignación de registros basada en grafos de interferencia, optimizaciones intraprocedurales más agresivas) no están completamente desarrolladas pero la arquitectura del proyecto permite añadir estos pases como transformaciones independientes sobre la IR.

Finalmente, `evaluator/tests.rs` y el módulo `evaluator` permiten ejecutar programas de HULK en un entorno de prueba (interpretativo o de ejecución del IR), lo cual facilita la verificación de la corrección del codegen y las optimizaciones.


## 8. 📐 Análisis del Lenguaje HULK y Diseño de Extensiones

Esta sección analiza con mayor profundidad el diseño del lenguaje HULK, su sistema de tipos y las extensiones propuestas o ya implementadas en el repositorio. Se ofrecen comparativas con lenguajes de referencia y justificaciones teóricas para cada decisión de diseño.

1) Sistema de Tipos de HULK

HULK incorpora tipos primitivos Number, Boolean y String, además de tipos compuestos como vectores y el tipo `Void` para funciones sin retorno. El sistema, tal como está implementado en `semantic/checker.rs`, se aproxima a un sistema de tipos estático y con comprobación en tiempo de compilación: los tipos de literales y las firmas de funciones se conocen en la compilación y la mayoría de las comprobaciones se hacen antes de la ejecución. Esto ofrece garantías de seguridad de tipo temprana (reducción de errores en tiempo de ejecución) y permite optimizaciones basadas en tipos.

Desde la teoría, el sistema de HULK se clasifica como estático y relativamente fuerte: no se admiten coerciones implícitas inseguras por defecto, y las operaciones aplican reglas de compatibilidad explícitas. La implementación usa un enfoque de inferencia local —no Hindley–Milner completo— que deduce tipos a partir de literales y contexto inmediato. Esto simplifica la implementación y evita el coste de resolver variables de tipo generales en programas educativos.

Comparativa: contrastando con C# / Java (estático y fuerte) y Python / JavaScript (dinámico y débil/mixto), HULK se sitúa más cerca de C#/Java en cuanto a verificación temprana, pero con menos complejidad en inferencia. Frente a JavaScript, HULK evita errores de tipo tardíos a costa de mayor verbosidad en algunas anotaciones. Frente a Java, HULK carece hoy de genéricos y del sistema de subtipado robusto, lo que limita la expresividad pero reduce la superficie de errores para el público objetivo educativo.

2) Extensiones Implementadas y Justificación



3) Análisis de Decisiones de Diseño

Las decisiones se han basado en criterios pedagógicos y técnicos: simplicidad, predecibilidad y seguridad. Elegir un sistema de tipos estático con inferencia local facilita la enseñanza de reglas de tipado (Γ ⊢ e : τ) sin abrumar con metaprogramación de tipos. Respecto a la organización de memoria, se usa el stack para marcos de activación y el heap para valores dinámicos (strings, estructuras), lo que refleja la distinción teórica entre duración automática y dinámica.

En cuanto al procesamiento, se prefirió un enfoque compilado a IR con posibilidad de ejecución interpretada por el `evaluator` en pruebas; esto permite ilustrar las diferencias entre procesamiento interpretado y compilado, y abre camino a JIT si se adopta LLVM como backend.

Varianza y seguridad de tipos: la propuesta de genéricos contempla restricciones covariantes/contravariantes para evitar violaciones de tipo en asignaciones y llamadas. En la comparación con C#/Java, se observa que estos lenguajes resuelven la varianza con anotaciones explícitas y reglas de subtipado; en HULK, por simplicidad, se propondría inicialmente restringir la varianza para evitar complejidad en la asignación.

4) Comparativa con Otros Lenguajes

HULK se diseña como lenguaje educativo con orientación a la seguridad y claridad en lugar de máxima expresividad o rendimiento. Frente a Python ofrece mayor seguridad de tipo; frente a Java ofrece mayor ligereza y rapidez para experimentar; frente a JavaScript evita coerciones implícitas que causan errores sutiles. La elección de características (tipado estático parcial, funciones de primera clase pero sin closures completos aún, soporte para recursividad) busca un equilibrio entre paradigmas imperativo y funcional, facilitando la transición del estudiante entre modelos teóricos y prácticos.

Conclusión de la sección: el diseño de HULK prioriza enseñar y aplicar principios de lenguajes (tipos, ámbitos, ejecución en pila) con extensiones que permiten explorar paradigmas funcionales y genéricos sin introducir complejidad excesiva. Las extensiones propuestas siguen un camino incremental: primero garantizar corrección semántica y luego añadir expresividad (generics, closures, optimizaciones).

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
