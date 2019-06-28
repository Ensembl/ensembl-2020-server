# Background

## Document Scope

Note that this document is intended for designers and maintainers of the dauphin compiler or tánaiste bytecode interpreter and for language experts looking for the absolute truth on some minor issue. It is not a good place to learn or practice writing Dauphin and any attempt to do so from this document will be unduly and wildly daunting.

## Purpose

Dauphin is not a general purpose programming language. It was not intended to be such and it is not formally such. Its specific purposes are two, closely related tasks:

1. the unzipping of compacted information sent by a server into a form suitable for rendering;
2. the addition of style to supplied data.

The two tasks can be seen as aspects of the same process each takes an information-dense source and expands it massively with additional attirbutes until it is in the form of explicit instructions.

## Design Constraints (Tánaiste)

The interpereter of compiled dauphin bytecode (tánaiste) runs in a challenging environment:

1. platform requirements require the implementation to be small;
2. it has some pretty hard realtime constraints;
3. it processes large data sets;
4. it must be supportable by a small team.

In all unoptimised bytecode implementations, inter-instruction dispatch tends to dominate execution time. With the requirements above, the hard realtime constraints and platform requirements exaserpate those issues: we have to do lots of fiddling with timers and schedulers and rightly aren't allowed just to compile our own code and jump to it on a browser. The large datasets make the issue critical: a single instruction dispatch per data item would instantly break the time budget.

Tánaiste is designed to efficiently handle these large data sets by applying transforms to entire vectors per instruction (per GPU languages). It is also designed to be rigid in the _shape_ of data which it accepts (almost exclusively flat arrays with uniformly typed members) to allow such looping instructions to be coded efficiently.

There are no explicit conditionals or loops in tánaiste and no functions (though dauphin includes macros). The dominant model is to supply boolean arrays of whether or not an operation should be applied to a given array member (called _condition arrays_, inspired by ARM condition codes), allowing fast evaluation of conditionals. Removing conditional, loop, and function instructions entirely significantly improves implementation and compiler simplicity, type inference, adds execution time bound guarantees, and allows for future paralelisation (service workers, GPU, or MMX, for exmaple). Condition arrays allow conditionals to be expressed without an explicit instruction, and the array-based types allow many looping type constructs to be expressed without explicit loops.

While this is fine and dandy for the tánaiste interpereter, the result is a bytecode language which it is fiendishly difficult to read and write. Condition arrays and the uniform type system, fundamental to tánaiste, are a particular challenge.

## Design Constraints (Dauphin)

The primary design purpose of dauphin is to make the authoring of tánaiste bytecode simple and enjoyable through a more compact, readable and maintainable representation. It is a balancing act between not obscuring parts of tánaiste which are best directly manipulated by the programmer and parts which are best obscured.

As described in the section above, then the primary targets are:

1. a simple representation for simple tasks;
2. a richer type system;
3. easier handling of condition arrays to allow conditional execution of instructions without an _if_-equivalent.

# Type System Structure

## Monovalues

Dauphin _monovalues_ are equivalent to values in other languages. They include numbers, strings, booleans and so on, and lists, structures of such, arbitrarily nested.

### Atomic monovalues

Daphin supports the following atomic monovalues:

* numbers (handled as 64-bit IEEE floats);
* booleans;
* strings;
* byte arrays.

Note the usual modern distinction between strings and byte-arrays and the need for encoding/decoding, even in the case of ASCII.

### Structured monovalues

Dauphin supports the following composite monovalues:

* structures containing zero or more other monovalues identified by a key;
* vectors of arbitrary length containing a single type of monovalue;
* enums of a descriminated union of a monovalue.

Note that there are _no tuples_. Tuples are useful but to avoid overstretching our punctuation budget are expressed as structs with numeric keys with a shorthand initialisation syntax.

Also note that each enum value can only contain a single other monovalue. This is to aid the syntax of tests which branch based on the enum value without supporting full destructuring assignment. Use a structure inside the enum if you need to contain multiple values.

## Multivalues

A Dauphin multivalue is an ordered sequence of zero-or-more monovalues of uniform monotype. Multivalues are represented in documents using « and ». Note that this is meta-syntax which is not avalilable in Dauphin itself.

Every expression writable in Dauphin evaluates to a multivalue. Even constants evaluate to a multivalue (one containing the single value corresponding to the monotype of the constant). For example `2` evaluates to «2». 

Multivalues are not the same thing as vectors: they are not designed for the storage of list information but are a tool to filter an action _only within an individual statement_. However, multivalues can easily be interconverted into vectors and back. For exmaple, `[1,2,3]` is a vector. Its multivalue has a single member, «`[1,2,3]`». However, `[1,2,3][]` breaks that vector into multiple monovlaues, «`1`, `2`, `3`». Whereas the `*` operator collects a multivalue into a list: `*[1,2,3][]` has the single-memeber multivalue «`[1,2,3]`» again.

Variables can only store zero or one monovalue. If they are assigned from longer multivalues the first value is used. For example, `x := [1,2,3][]` sets x to «`1`», whereas `y := [][]` sets y to «».

Note that multivalues can only contain monovalues, not other multivalues. So they are always present in a value, but only at the top level.

## Filtering

Multivalues exist to facilitate filtering, which is how tánaiste condition arrays manifest themselves in Dauphin. A raw filter is enclosed in braces and has two special variables available inside: `$` evaluates to a multivalue of the values of the filtered expression and `@` evaluates to the position. Other (defined) varaibles and expressions can also be used within the filter.

For example, consider the multivalue «`1`, `2`, `3`» created, perhaps by `[1,2,3][]`. The expression `{$>1}` in, for exmaple, `[1,2,3][]{$>1}` would evaluate to «`2`, `3`». Similarly, `[1,2,3][]{@==0}` would evaluate to «`1`».

As syntactic sugar for the most common operaion of filters, vectors can be filtered using square brackets with the following expansion: "vec`[`expr`]`" is equivalent to "vec`[]{`expr`}`". This takes an array, splits it into a multivalue and filters it. In practice this notation is much more common and convenient than the brace notation above.

For example, `[1,2,3][$!=3]` has multivalue «`1`, `2`» and `*[1,2,3][$!=3]` has multivalue «`[1,2]`». This is the kind of expression typically used for filtering in practice.

## Behaviour of multiple multivalues in statements

Each statement and expression may accomodate multiple subexpressions, and so needs to have a defined behaviour with respect to multivalues of different length. The behaviour is, in general, defined by the expression concerned but is _not_ typically a more product of the two multivalues. 

Typically a single expression is chosen as the "controlling" expression and the other expressions are repeated, cycling in sequence, until the controlling expression is consumed. If any expression is the empty multivalue, typically no operation is performed. This is the semantics for assignment, for example, as for arithmetic operations, where the lefthandside is controlling.

For example `[1,2,3][] + [4,5][]` equals «`5`, `7`, `7`» (the last 7 being 3+4 from cycling of the second array). Whereas `[4,5][] + [1,2,3][]` equals «`5`, `7`». Similarly, if `x := [1,2,3]`, then `x[$>1] += 10` assigns the multivalue «`[1,12,13]`» to x.

Vector operations, where defined for an operator or statement, typically follow a similar pattern.

## lvalues

An lvalue is an expression identifies a particular stored object. It contrasts with an rvalue, which is any expression with a value. All lvalues are also rvalues, but not the other way around. L and R are from left and right, refering to positions in an assignment statement (the notation is from the C-world). For exmaple, the variable `x` is an lvalue and an rvalue but the constant `2` is only an rvalue. This is because `x := 2` and `x := x` make sense, but `2 := x` does not.

Dauphin multivalues can be lvalues of monovalues. This means that filters can appear on the right-hand-side of assignments. This is useful for conditional update. For example, if `x := [1,2,3,11,12,13]` then `x[$>10] += 20` will set x to «`[1,2,3,31,32,33]`». As a more complex example, `x[$>10] := x[@<2]` will set x to «`[1,2,3,32,34,34]`».

Conditional assignment is achieved by reducing the lvalue multivalue to length zero with filters. For example, `x{y>1} := z` will set x to z if-and-only-if y is greater than one.

# Expressions

## Expressions and Types

An expression comprises one or more of.

* an atomic monovalue constant; ✓
* nil; ✓
* a `$` or `@` (inside a filter); ✓
* a vector of expressions; ✓
* an enum branch of an expression; ✓
* a struct of expressions; ✓
* a variable; ✓
* a star of an expression; ✓
* an inline or function-like operator with appropriate expressions in its placeholders;
* an _expression macro_ call with appropriate expressions; ✓
* an expression followed by:
  * a qualifier to a struct; ✓
  * a square-bracket filter (to a vector); ✓
  * a brace filter; ✓
  * an enum test branch (to the respective enum); ✓
  * an enum value branch (to the respective enum) ✓.

A type describes the structure of a value according to which of the options above are taken and the type of monovalues concerned. For example, `2` is an atomic monovalue of type `number`. `[2,3]` is a vector of expressions, these expressions having type `number` and so is of type `vec(number)`. Types may contain placeholders (which can be unified in expressions). A placeholder is a word beginning with an uppercase letter, or an underscore. Each underscore is treated as if a separate uppercase letter from an infinite set

## Atomic Monovalue Constants

An atomic monovalue constant is one of:

 * A number of type `number`, with constants represented in the usual way for floats;
 * A boolean of type `bool` with constants  represented as `true` or `false`;
 * A string of type `string` with constants enclosed in double quotes with backslash escaping (`"hello \"world\""`).
 * Bytes of type `bytes` with constants enclosed in single quotes and represented in two-digit hex. (`'68696c6c6f'`).

Its value is a multivalue with a single member, the obvious corresponding monovalue. It is not an lvalue.

## nil

Nil is represented by the constant `nil`. Its value is a multivalue with no members. It is not an lvalue. Its type is `_`.

## Filters and Star

A brace filter comprises an expression followed by a boolean-typed expression within braces. It has the same type as the preceding expression. It is an lvalue if-and-only-if the preceding expression is an lvalue.

Inside the braces `$` has the same type as the first expression, and `@` has type `number`. Neither are lvalues.

A square bracket filter has the same syntax as a brace filter but with square brackets. The preceding expression must have a type `vec(`X`)` and the overall expression will have type X. It is an lvalue if-and-only-if the preceding expression is an lvalue.

Inside the square brackets `$` has type X and `@` has type number if the original expression has type `vec(`X`)`. Neither are lvalues.

A star comprises an asterisk and an expression. If the expression has type X the star will have type `vec(`X`)`. It is not an lvalue.

## Empty Value

A filter which assigns to a cell in a vector beyond the current limit "backfills" the intermediary values with the "empty" value for its type.

The empty value for an atomic values is `0`, `false`, `""`, and `''`, for each atomic monotype respecitvely. The empty value for a vector is `[]`. For a struct, the empty value has all fields filled with their empty value. Enums choose the first declared branch and set their value to the empty value of the contained branch. For example, if `x:=[1]` and then `x[@==3] := 4` then x would have the final value «`[1,0,0,4]`».

This is one way to create a list of repeating elements. `let x[@==9] := true; x[] := true;` creates a list of ten true members.

## Variables

A variable is represented by a keyword which is not a reserved word. Its type is determined by its inferred contents at any time. Its initial type is `_` and initial value «». It must be introduced with a `let` statement (which cna be combined with another statement as syntactic sugar to create that statements first argument as a new variable). `let` must be supplied again before any type change. 

A variable is an lvalue.

The reserved words are: `enum`, `expr`, `false`, `func`, `import`, `let`, `lvalue`, `nil`, `oper`, `stmt`, `struct`, `true`.

 ## Vector Constants

A vector constant is constructed from a comma-separated list of expressions enclosed in square brackets. Each enclosed expression must have the same type. For example, `[2,false]` is invalid. The type of the vector is `vec(`X`)`, where X is the type of the enclosed values. If no values are enclosed it is of type `vec(_)`. Its value is a multivalue with a single member, the obvious corresponding monovalue. It is not an lvalue.

## Structs

An struct must be declared prior to use. It is declared with the `struct` keyword, the struct name, followed by braces. The contents of the braces are a comma-separated list of:
* a keyword or number, colon and a type, or
* a type.

The latter is syntactic sugar for numerically indexed struct. For example `struct test {number,bool}` is sugar for `struct test {0: number, 1: bool}`.

Its type is `struct:`typename. For example, the above struct has type `struct:test`. Note that the struct name is stored in the same namespace as enum names.

A struct constant comprises the struct keyword followed by the appropriate value in braces in one of the two above formats. For example, with the above definition, `x := test{0: 6, 1: true}` assigns such a constant to x, as does `x := test{6,true}`. It is not an lvalue.

A struct qualifier is an expression which has a struct type, followed by a period and then a valid key. For example, given x above, `x.0` has value «`6`». Its type is the type of the contained key in the type of the struct. It is an lvalue if-and-only-if the containing expression is an lvalue.

## Enums

An enum must be declared prior to use. It is declared with the `enum` keyword, enum name, followed by braces. The contents of the braces are a comma separated list of branches. A branch is a branch name keyword followed by a colon and a type. type may also be `nil` if that branch has no contents. For example `enum test2 { A: number, B: bool }`. Its type is `enum:`typename. For our example out example has type `enum:test2`. Note that the struct name is stored in the same namespace as enum names.

An enum constant comprises the enum keyword, a colon and the enum branch name, parentheses and a value of the given type. For example, `test2:A(6)` or `test2:B(false)`. It is not an lvalue.

An enum test branch comprises an expression, a question-mark and a colon-separated enum branch name. In our example, `x?test2:A` would be such an expression assuming variable `x` exists. It has a boolean value which is true if-and-only-if the preceding expression is the corresponding branch of the enum. The type of the preceding expression must be the relevant enum (in our case `enum:test2`) and it has type `bool`. It is not an lvalue.

An enum value branch comprises an expression, an exclamation-mark and a colon-separated enum branch name. In our example, `x!test2:A` would be such an expression assuming variable `x` exists. Its multivalue is either empty (if x has a different branch) or else the value of the branch. In our examples, if we have `x := test2:A(6)` then `x!test2:A` would have multivalue «`6`» whereas `x!test2:B` would have multivalue «». The type of the preceding expression must be the relevant enum (in our case `enum:test2`) and it has the type of the contained branches type. In our exmaple X`!test2:A` will have type `number` and  X`!test2:B` will have type bool. It is an lvalue if-and-only-if its preceding expression is an lvalue.

## Function-like operators

Operators access computation functionality within the corresponding tánaiste bytecode, which will vary based on the embedding. Every inline-like operator has a corresponding function-like operator but the converse is not the case. Where inline-like syntax exists it should be preferred.

A function-like operator has a type of its return value given its arguments. Operators may not be overloaded but may use placeholders within types.

A statement of the form "`func` name`(`X,Y,Z`) -> ` A `{` code `}`" declares a function-like operator which takes arguments of type X, Y, Z and has type A, with the given inline tánaiste code (see later).

For example `list_concat` may be declared as `func list_concat(list(X),list(X)) -> list(X)` to allow concatenation of arbitrary lists.

An operator is not an lvalue.

**TODO** func syntax when tánaiste is defined.

**TODO** overloading (eg `==`).

## Inline-like operators

### Declaration

An inline-like operator can be unary or binary. It is a series of one or more punctuation characters and can be declared left or right associative if binary. An inline-like operator is syntactic sugar for some function-like operator. An inline-like operator is declared in the preamble to associate itself with a corresponding function-like operator.

A preamble takes the form "`oper ` op-syntax func-name nature precedence". Here nature is one of `infixl`, `infixr`, `unary`. and precedence is a number (low is tighter). For example `oper + plus infixr 2`.

The considerable additional effort of allowing the definition of additional operators is to compensate for the lack of object-like syntax and the absence of overloading.

### Valid syntax

The syntax of validly definable operators is complex to ensure a wide range can be defined unambiguously.

In the following definition, the following sets are used:

* core characters: ``#%&+-/<=>\^`|~``
* bracket characters: `()[]{}`
* internal characters: `:*!?.,`

A valid operator symbol is a sequence which either:
* Class A:
  * contains one or more core characters
  * comprises only core or internal characters.
* Class B:
  * begins and ends with matching bracket characters;
  * contains only internal characters;
* Class C:
  * begins and ends with matching bracket characters;
  * contains a valid Class A, B, or C operator symbol.

For example, `+=`, `||`, `$add`, `!=` are valid Class A operators. `(!)` is a valid Class B operator. `(||)` is a valid Class C operator containing a valid Class A operator.

In addition, no operator can introduce a symbol such that one member of the set of operator symbols is now identical to some other operator symbol followed by zero-or-more further unary operator symbols. For example, if `+` is a unary operator and `=` a binary one, `+=` is valid but `=+` is not.

Some classes of operators are reserved even if not explicitly declared.

* `#`one-or-more-alpha-maybe-with-embedded-colon...  – reserved for cooked intermediate form
* `%alpha` – reserved for cooked intermediate form

*Rationale*: 
* Class A: a core character is not valid in Dauphin except in an operator definition. Internal characters cannot end an expreesion and so a sequence of them followed by a core character must be unambiguously introducing an operator. Internal characters can also not begin an expression and so any further internal or core characters must be a continuation of the operator.
* Class B: the contents of the brackets are entirely internal characters but such characters cannot introduce or end an expression so the contents are an invalid expression. The brackets ensure that they cannot be "composed" with any adjacent characters to make the operator ambiguous.
* Class C: if the contents of brackets can only be unambiguously interpreted as an operator symbol, these cannot stand alone in parentheses, so the whole sequence is only interpreretable as an operator symbol.

## Expression Macro Calls

An expression macro call is introduced by "`expr` name`(`X,Y,Z`) {` expression `}`". This literally substitutes the given expression into the place it is used. Note that expression must be a *valid* expression and all (non-argument) variables are local however argument variables are by-name. It is an lvalue if-and-only-if the contained expression is such. It has the same type as the contained expression.

# Statements

## Introduction

Dauphin statements are separated by `;`. Dauphin source is a sequence of statements. Dauphin statements are executed in-order such that definition must precede use. The `import` statement allows inclusion of files with further content which is evaluated as if it occurred at the import point. An import statement has the form "`import` location" where location is a path for the compiler.

Other statements may be inline-like or function-like. They may be:

* an `import` statement;
* a type declaration:
  * an `enum` type declaration;
  * a `struct` type declaration;
* a macro declaration:
  * an `expr` expression macro declaration;
  * a `stmt` statement macro declaration;
* an `oper` inline operator declaration;
* a function/procedure declaration;
  * a `func` function declaration;
  * a `proc` procedure declaration;
* a procedure call.

## Statement Macros

An statement macro call is introduced by "`stmt` name`(`X,Y,Z`) {` expression `}`". This literally substitutes the given statements into the place it is used. Note that statements must be a *valid* sequence of statements and all (non-argument) variables are local. However argument variables are by-name.

If arguments are required to be lvalues by the statement defiinition (because they are used as lvalues by some statement within), then that argument is required to be an lvalue at point of use.

## Procedure Declarations

Procedures resemble operators but take the role of statements rather than expressions.

A function-like procedure has no type (being a statement).

A statement of the form "`proc` name`(`X,Y,Z`) {` code `}`" declares a function-like operator which takes arguments of type X, Y, Z, with the given inline tánaiste code (see later).

For example `assign` may be declared as `proc assign(X,X)` to allow assignment of variables.

Oper statements can also define inline procedures as well as inline operators. Which is defined depends on the func or proc refered to. For example `oper := assign infixr 2`. Note that associativity is irrelevant in opers defining statements.

A procedure may define one or more of its arguments to be lvalues using the keyword `lvalue` before the type. This requires that in use that argument be a valid lvalue and is passed as a location to the definition.

**TODO** func syntax when tánaiste is defined.

# Dauphin to Tánaiste Translation

## Transformation into Cooked Instruction Form

### Introduction

Cooked instruction form is a linear, assembly-like form which still uses the rich types of Dauphin. The first stage in code generation is to transform parse trees of potentially complex expressions into this simple form. Instructions in this form are in the intermediate format `#instr %reg %reg`.... A few load instructions take atomic monovalues as an argument in addition to registers.

`import`, `expr`, `stmt` and `oper` statements are processed during this stage and removed from the statement stream.

`enum`, `struct`, `func` and `proc` statements survive unaltered into the statement stream.

Procedure call statements are translated (the primary purpose of the transformation).

### Types

Registers have types. Initially these types may be partially undetermined. Constraints on the types of generated instructions will further refine the type.

In addition to the types directly expressible in Dauphin, registers in cooked instruction format can be lvalues denoted by a leading `&` in the type. A placeholder can never contain a `&` type and `&` is always at the top level.

### Building Constants

 * `#nil %reg` — Put nil into `%reg`(gets type `_`)
 * `#number %reg number` — Put number in `%reg` (gets type `number`); 
 * `#bool %reg bool` — Put bool into `%reg`  (gets type `bool`);
 * `#string %reg string` — Put string into `%reg`  (gets type `string`);
 * `#const %reg bytes` — Put pytes into `%reg` (gets type `bytes`);
 * `#list %reg` — Put «`[]`» into `%reg` (gets type `vec(_)`);
 * `#push %reg %val` — add `%val` to list in `%reg` (`%reg` must be of type `vec(X)` where `X` is type of `%val`);
 * `#struct:`typename `%reg %val1 %val2`... — Create struct in `%reg` with given vlaues (gets type `struct:`typename);
 * `#enum:`typename`:`branch `%reg %val` — Create enum in `%reg` with given branch and value (gets type `enum:`typename).

For example, the following program:

```
struct s {bool, number};
enum e { A: s, B: nil };
x := [e:A(s{ 0: true, 1: 42}),e:B];
```

could have the following cooked instruction form:

```
struct s {bool, number};
enum e { A: s, B: nil };
#bool %true true;
#number %42 42;
#struct:s %s %true %42;
#enum:e:A %A %s;
#enum:e:B %B;
#list %x;
#push %x %A;
#push %x %B;
```

In this case the types of the registers are:

* `%true` — `bool`
* `%42` — `number`
* `%s` — `struct:s`
* `%A`, `%B` — `enum:e`
* `%x` — `vec(enum:e)` but only known to be `vec(_)` initially.

### Variables and lvalues

A variable used as an rvalue is simply represented by the variable which it is contained within and has the corresponding type.

When a variable is used in an statement which uses its argument as an lvalue, the value is represented as `&`type.

A reference is generated with `#ref %out %in` where `%out` is of type `&`X if `%in` is of type X.

### Qualifier and Branch rvalues

* `#etest:`typename`:`branch ` %bool %reg` — Put «true»/«false» into bool depending on whether `%reg` has given branch. (type of `%bool` is `bool`, type of `%reg` must be `enum:`typename)
* `#evalue:`typename`:`branch ` %val %reg` — Put branch value or nil into bool depending on whether `%reg` has given branch. (type of `%val` is type of branch, type of `%reg` must be `enum:`typename)
* `#svalue:`typename`:`key `%val %reg` — Put value of given struct member of `%reg` into `%val`. (type of `%val` is type of member, type of `%reg` must be `struct:`typename)
* `#refevalue:`typename`:`branch `%val %reg` — moves a reference into a reference to the branch value. Type of `%reg` must be `&`X where X is the corresponding enum type, and the type of `%val` becomes `&`Y, where Y is the type of the branch.
* `#refsvalue:`typename`:`key `%val %reg` — Put reference to given struct member of `%reg` into `%val`. (type of `%val` is `&`X where X is the type of the member, type of `%reg` must be `&struct:`typename)

### Filters

After removal of the [`expression`] syntactic sugar, the remaining filter operators are `*`, `[]` (aka *square*), and `{`expression`}`.

Filter expressions are first converted into equivalent expressions evaluating to a `vec(bool)` preceding the statement in question. These conversions are applied immediately before use. `$` is substituted with the expression in question. 

`@` is replaced by `#at %out, %val` which puts a `vec(number)` into `%out` which is the length ov `%val` and increasing from 0 by 1.

The resulting expression is applied with `#filter %out %in %filter`.

For example: `x := x[$==3];` becomes (assuming `%3` is «3» etc):

```
#oper:equal %filter %x %3;
#filter %new_x %x %filter;
```

Whereas `x := x[ y[$==2 && x[@==1]>z] || x[$==3] ]` becomes:

```
/* x[@==1]>z */
#at %atx %x;
#oper:equal %filter1 %1;
#filter %tmp1 %x %filter1;
#oper:gt %tmp2 %tmp1 %z;

/* y[$==2 && x[@==1]>z] */
#oper:equal %filter2 %y %2;
#oper:and %filter21 %filter2 %filter1;
#filter %tmp3 %y %filter21

/* x[$==3] */
#oper:equal %filter3 %x %3;
#filter %tmp4 %x %filter3;

/* x[ y[$==2 && x[@==1]>z] || x[$==3] ] */
#oper:or %tmp5 %tmp3 %tmp4;
#filter %out %x %tmp5;
```

* `#star %out %in` — Put vector of multival from `%in` into `%out`. type of `%out` is set to `vec(%in)`.
* `#square %out %in` — Expand vec into multival. Type of `%out` is X when type of `%in` is `vec(`X`)`.
* `#at %out %val` — Create run of values for position matching. Type of `%out` is `vec(number)`; type of `%val` must be `vec(_)`.
* `#filter %out %in %filter` — Apply filter `%filter` to `%in`, yielding `%out`. `%in` and `%out` must be of the same (non-reference) type and `%filter` of type `vec(bool)`.
* `#reffilter %out %in %filter` — Apply filter `%filter` to `%in`, yielding `%out`. `%in` and `%out` must be of the same (reference) type and `%filter` of type `vec(bool)`.


### lvalue example

Consider the following statements:

```
struct s {bool, vec(number)};
enum e { A: s, B: nil };
x := [e:A(s{ 0: true, 1: [0,42]}), e:B];
x?e:A.1[$<10] := 23;
```

The result should be that x equal «`[e:A(s{0, true, 1: [23,42]}), e:B]`» and the last statement could be represented by the statements:
```
#ref %refx %x;
#refevalue:e:A %refA %refx;
#refsvalue:s:1 %ref1 %refA;
#oper:lt %filter %x %10;
#reffilter %refs %ref1 %filter;
#oper:assign %refs, %23;
```

**TODO**: universalise infix

