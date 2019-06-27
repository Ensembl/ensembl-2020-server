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

Multivalues exist to facilitate filtering, which is how tánaiste condition arrays manifest themselves in Dauphin. A raw filter is enclosed in braces and has two special variables available inside: `$` denotes the value of the monovalue (as a single-member multivalue) and `@` denotes the position. Other (defined) varaibles and expressions can also be used within the filter.

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

* an atomic monovalue constant;
* nil;
* a `$` or `@` (inside a filter);
* a vector of expressions;
* an enum branch of an expression;
* a struct of expressions;
* a variable;
* a star of an expression;
* an inline or function-like operator with appropriate expressions in its placeholders;
* an _expression macro_ call with appropriate expressions;
* an expression followed by:
  * a qualifier to a struct;
  * a square-bracket filter (to a vector);
  * a brace filter;
  * an enum test branch (to the respective enum);
  * an enum value branch (to the respective enum).

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

A brace filter comprises an expression followed by a boolean-typed expression within braces. It has the same type as the preceding expression. I is an lvalue if-and-only-if the preceding expression is an lvalue.

Inside the braces `$` has the same type as the first expression, and `@` has type `number`. Neither are lvalues.

A square bracket filter has the same syntax as a brace filter but with square brackets. The preceding expression must have a type `vec(`X`)` and the overall expression will have type X. It is an lvalue if-and-only-if the preceding expression is an lvalue.

Inside the braces `$` has type X and `@` has type number. Neither are lvalues.

A star comprises an asterisk and an expression. If the expression has type X the star will have type `vec(`X`)`. It is not an lvalue.

## Variables

A variable is represented by a keyword which is not a reserved word. Its type is determined by its inferred contents at any time. Its initial type is `_` and initial value «». It must be introduced with a `let` statement (which cna be combined with another statement as syntactic sugar to create that statements first argument as a variable). It is an lvalue.

The reserved words are: `enum`, `expr`, `false`, `func`, `import`, `let`, `nil`, `oper`, `struct`, `true`.

 ## Vectors

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

An enum must be declared prior to use. It is declared with the `enum` keyword, enum name, followed by braces. The contents of the braces are a comma separated list of branches. A branch is a branch name keyword followed by a colon and a type. For example `enum test2 { A: number, B: bool }`. Its type is `enum:`typename. For our example out example has type `enum:test2`. Note that the struct name is stored in the same namespace as enum names.

An enum constant comprises the enum keyword, a colon and the enum branch name, parentheses and a value of the given type. For example, `test2:A(6)` or `test2:B(false)`. It is not an lvalue.

An enum test branch comprises an expression, a question-mark and a colon-separated enum branch name. In our example, `x?test2:A` would be such an expression assuming variable `x` exists. It has a boolean value which is true if-and-only-if the preceding expression is the corresponding branch of the enum. The type of the preceding expression must be the relevant enum (in our case `enum:test2`) and it has type `bool`. It is not an lvalue.

An enum value branch comprises an expression, an exclamation-mark and a colon-separated enum branch name. In our example, `x!test2:A` would be such an expression assuming variable `x` exists. Its multivalue is either empty (if x has a different branch) or else the value of the branch. In our examples, if we have `x := test2:A(6)` then `x!test2:A` would have multivalue «`6`» whereas `x!test2:B` would have multivalue «». The type of the preceding expression must be the relevant enum (in our case `enum:test2`) and it has the type of the contained branches type. In our exmaple X`!test2:A` will have type `number` and  X`!test2:B` will have type bool. It is an lvalue if-and-only-if its preceding expression is an lvalue.

## Function-like operators

Operators access computation functionality within the corresponding tánaiste bytecode, which will vary based on the embedding. Every inline-like operator has a corresponding function-like operator but the converse is not the case. Where inline-like syntax exists it should be preferred.

A function-like operator has a type of its return value given its arguments. Operators may not be overloaded but may use placeholders within types.

A statement of the form "`func` name`(`X,Y,Z`) -> ` A `{` code `}`" declares a function-like operator which takes arguments of type X, Y, Z and has type A, with the given inline tánaiste code (see later).

For example `list_concat` may be declared as `func list_concat(list(X),list(X)) -> list(X)` to allow concatenation of arbitrary lists.

**TODO** func syntax when tánaiste is defined.

## Inline-like operators

An inline-like operator can be unary or binary. It is a series of one or more punctuation characters and can be declared left or right associative. An inline-like operator is syntactic sugar for a function-like operator. A function-like operator is declared in the preamble to associate itself with a corresponding function-like operator.

The following characters are permitted for inline-like operators: `#%&+-/<=>\^_|~`.

In addition, `!?:.` are valid at the start of an inline-like operator if followed by a non-keyword or number character. `*` is valid as a binary oparator.

An inline-like operator can begin with an open parenthesis followed by at least one unrestricted character or `!?*,.` then any character excluding close parenthesis, followed by a close parenthesis. For example `(:eg-1)` or `(?)`.

Note that inline-like operators share a namespace with inline-like statements.

If it does not begin with a parenthesis but begins directly with an unrestricted punctuation character it may comprise any unambiguous prefix-free combination subsequent characters. For example `<hello>` may be defined on the condition that `<` is not (it probably will be).

A preamble takes the form "`oper ` op-syntax func-name nature precedence". Here nature is one of `infixl`, `infixr`, `unaryl`, `unaryr` and precedence is a number (low is tighter). For example `oper + infixr 2`.

The considerable additional effort of allowing the definition of additional operators is to compensate for the lack of object-like syntax and the absence of overloading.

## Expression Macro Calls

An expression macro call is introduced by "`expr` name`(`X,Y,Z`) {` expression `}`". This literally substitutes the given expression into the place it is used. Note that expression must be a *valid* expression and all (non-argument) variables are local however argument variables are by-name. It is an lvalue if-and-only-if the contained expression is such and has the same type.

# Statements

Dauphin statements are separated by `;`. Dauphin source is a sequence of statements. Dauphin statements are executed in-order such that definition must precede use. The `import` statement allows inclusion of files with further content which is evaluated as if it occurred at the import point.

A statement may be inline-like or function-like.

## Import statements

## Function-like statements

## Inline-like statements
