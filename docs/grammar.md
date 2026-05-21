(* ─────────────────────────────────────────────
   HULK — Gramática Libre de Contexto Completa
   Notación: EBNF estándar
     X*   = cero o más X
     X+   = uno o más X
     X?   = cero o uno X
     (X)  = agrupación
     X|Y  = alternativa
   ───────────────────────────────────────────── *)

(* ══════════════════════════════════════════════
   PROGRAMA
   ══════════════════════════════════════════════ *)

Program     = Decl* GlobalExpr ;

GlobalExpr  = Expr ";"? ;

Decl        = FuncDecl
            | TypeDecl
            | ProtocolDecl
            | MacroDecl ;

(* ══════════════════════════════════════════════
   DECLARACIONES DE FUNCIÓN
   ══════════════════════════════════════════════ *)

FuncDecl    = "function" IDENT "(" ParamList ")" ReturnType? FuncBody ;

FuncBody    = "=>" Expr ";"          (* inline *)
            | Block ;                 (* bloque *)

ParamList   = ( Param ( "," Param )* )? ;

Param       = IDENT TypeAnnotation? ;

ReturnType  = ":" TypeExpr ;

(* ══════════════════════════════════════════════
   DECLARACIONES DE TIPO
   ══════════════════════════════════════════════ *)

TypeDecl    = "type" IDENT TypeArgs? Inheritance? "{" TypeMember* "}" ;

TypeArgs    = "(" TypeParamList ")" ;

TypeParamList = ( TypeParam ( "," TypeParam )* )? ;

TypeParam   = IDENT TypeAnnotation? ;

Inheritance = "inherits" IDENT ( "(" ArgList ")" )? ;

TypeMember  = AttrDef
            | MethodDef ;

AttrDef     = IDENT TypeAnnotation? "=" Expr ";" ;

MethodDef   = IDENT "(" ParamList ")" ReturnType? FuncBody ;

(* ══════════════════════════════════════════════
   DECLARACIONES DE PROTOCOLO
   ══════════════════════════════════════════════ *)

ProtocolDecl = "protocol" IDENT ProtocolExtends? "{" MethodSig* "}" ;

ProtocolExtends = "extends" IDENT ;

MethodSig   = IDENT "(" MethodSigParams ")" ":" TypeExpr ";" ;

MethodSigParams = ( MethodSigParam ( "," MethodSigParam )* )? ;

MethodSigParam  = IDENT ":" TypeExpr ;

(* ══════════════════════════════════════════════
   DECLARACIONES DE MACRO
   ══════════════════════════════════════════════ *)

MacroDecl   = "def" IDENT "(" MacroParamList ")" FuncBody ;

MacroParamList = ( MacroParam ( "," MacroParam )* )? ;

MacroParam  = "*" IDENT ":" TypeExpr     (* block argument *)
            | "@" IDENT ":" TypeExpr     (* symbolic argument *)
            | "$" IDENT ":" TypeExpr     (* variable placeholder *)
            | Param ;                     (* regular argument *)

(* ══════════════════════════════════════════════
   EXPRESIONES — JERARQUÍA DE PRECEDENCIA
   (de menor a mayor precedencia)
   ══════════════════════════════════════════════ *)

Expr        = LetExpr
            | IfExpr
            | WhileExpr
            | ForExpr
            | AssignExpr ;

AssignExpr  = PostfixExpr ":=" AssignExpr  (* derecho asosiativo *)
            | OrExpr ;

OrExpr      = OrExpr "|" AndExpr        (* izquierdo-asociativo *)
            | AndExpr ;

AndExpr     = AndExpr "&" NotExpr       (* izquierdo-asociativo *)
            | NotExpr ;

NotExpr     = "!" NotExpr
            | CmpExpr ;

CmpExpr     = CmpExpr ( "==" | "!=" | "<" | ">" | "<=" | ">=" ) CatExpr
            | CatExpr ;

CatExpr     = CatExpr ( "@" | "@@" ) AddExpr   (* izquierdo-asociativo *)
            | AddExpr ;

AddExpr     = AddExpr ( "+" | "-" ) MulExpr    (* izquierdo-asociativo *)
            | MulExpr ;

MulExpr     = MulExpr ( "*" | "/" | "%" ) PowerExpr  (* izquierdo-asociativo *)
            | PowerExpr ;

PowerExpr   = UnaryExpr "^" PowerExpr   (* derecho-asociativo *)
            | UnaryExpr ;

UnaryExpr   = "-" UnaryExpr
            | PostfixExpr ;

PostfixExpr = PostfixExpr "." IDENT ( "(" ArgList ")" )?   (* acceso/método *)
            | PostfixExpr "[" Expr "]"                       (* indexación *)
            | PostfixExpr "is" TypeExpr                      (* type test *)
            | PostfixExpr "as" TypeExpr                      (* downcast *)
            | PrimaryExpr ;

PrimaryExpr = NUMBER
            | STRING
            | "true"
            | "false"
            | IDENT ( "(" ArgList ")" )?    (* variable o llamada *)
            | "self"
            | "base" ( "(" ArgList ")" )?
            | "new" IDENT ( "(" ArgList ")" )?
            | "(" Expr ")"
            | Block
            | VectorExpr ;

(* ══════════════════════════════════════════════
   CONSTRUCCIONES ESPECIALES
   ══════════════════════════════════════════════ *)

LetExpr     = "let" LetBinding ( "," LetBinding )* "in" Expr ;

LetBinding  = IDENT TypeAnnotation? "=" Expr ;

IfExpr      = "if" "(" Expr ")" Expr ElifClause* ElseClause ;

ElifClause  = "elif" "(" Expr ")" Expr ;

ElseClause  = "else" Expr ;

WhileExpr   = "while" "(" Expr ")" Expr ;

ForExpr     = "for" "(" IDENT "in" Expr ")" Expr ;

Block       = "{" ( Expr ";" )* Expr "}"
            | "{" ( Expr ";" )+ "}" ;

VectorExpr  = "[" ArgList "]"                           (* vector explícito *)
            | "[" Expr "|" IDENT "in" Expr "]" ;        (* vector implícito *)

ArgList     = ( Expr ( "," Expr )* )? ;

(* ══════════════════════════════════════════════
   TIPOS
   ══════════════════════════════════════════════ *)

TypeExpr    = IDENT "*"     (* iterable de T *)
            | IDENT "[]"    (* vector de T *)
            | "(" TypeList ")" "->" TypeExpr   (* functor *)
            | IDENT ;        (* tipo nominal *)

TypeList    = ( TypeExpr ( "," TypeExpr )* )? ;

TypeAnnotation = ":" TypeExpr ;

(* ══════════════════════════════════════════════
   TERMINALES
   ══════════════════════════════════════════════ *)

IDENT       = [a-zA-Z][a-zA-Z0-9_]* ;
NUMBER      = [0-9]+ ( "." [0-9]+ )? ;
STRING      = '"' ( [^\n"\\] | '\\' . )* '"' ;