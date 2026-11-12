Implement an interpreter for a small Lisp. The whole standard input is the program. It is read completely and parsed first; if parsing fails nothing is executed.

Lexical structure. Whitespace separates tokens. A `;` starts a comment that runs to the end of the line. The tokens are `(`, `)`, integers (an optional `-` followed by digits, nothing else; more than 15 digits is the parse error `number too large`), `#t`, `#f`, and symbols (any other run of characters that contains no whitespace, `(`, `)` or `;`). Lines are numbered from 1. A form is a token, or a parenthesized list of forms. The program is the sequence of top-level forms.

Parse errors are reported as `parse error line N: MESSAGE` on standard output with exit status 2, and nothing else is printed:
- a `)` with no matching `(`: `unexpected )` (N is the line of that `)`)
- an unclosed `(` at end of input: `unclosed (` (N is the line of the outermost unclosed `(`)
- a number longer than 15 digits: `number too large` (N is its line)
Parse errors are detected in reading order, and the first one wins; an unclosed `(` is detected at end of input. The empty list `()` is a valid form (it is used for empty parameter lists); evaluating it as an expression is `bad syntax`.

Values: integers, booleans (`#t`, `#f`) and closures. `print` shows integers in decimal, booleans as `#t` and `#f`, closures as `<closure>`.

Evaluation. A symbol is looked up in the innermost environment that has it. An integer or boolean evaluates to itself. For a list, the first element decides:

- `(define NAME E)` evaluates E and binds NAME in the current environment (rebinding replaces it). Value: 0.
- `(set! NAME E)` evaluates E and assigns to the innermost existing binding of NAME. Value: the old value of NAME.
- `(lambda (P1 ... Pn) B1 ... Bk)` with k >= 1 creates a closure that remembers the environment in which it was created itself, not a copy: later changes to those variables are visible inside the closure and the other way round.
- `(let ((N1 E1) ... ) B1 ... Bk)` evaluates all Ei in the current environment, then runs the bodies in a new environment (child of the current one) holding the bindings. Value: the last body. `k >= 1`.
- `(if C T E)` and `(if C T)`: C must be a boolean. Without the else branch a false C gives 0.
- `(begin E1 ... En)` evaluates in order, value of the last (0 if n = 0). `(while C B1 ... Bk)` repeats the bodies while C is `#t` (C must be boolean each time); value 0.
- `(and E ...)` and `(or E ...)` short-circuit, require booleans for each evaluated operand, and return `#t` / `#f` when given no operands respectively. `(not E)` needs a boolean.
- `(print E)` prints the value and a newline; value 0.
- `(+ E ...)`, `(* E ...)` take any number of integer operands (`(+)` is 0, `(*)` is 1). `(- E ...)` with one operand negates, with more subtracts left to right (at least one operand). `(/ A B)` truncates toward zero and `(% A B)` has the sign of A (like C and Rust). `(< A B)`, `(> A B)`, `(<= A B)`, `(>= A B)`, `(= A B)` compare two integers and give booleans.
- Any other list is a call: evaluate the first element (it must give a closure), then the arguments from left to right, then run the closure body in a new environment whose parent is the closure's environment, binding parameters to arguments.

The names of the special forms and operators above are reserved: they cannot be used as the NAME of `define`, `set!` or as a parameter or `let` binding name (that is `bad syntax`), and a reserved name used as a variable reference is `unbound variable`. They cannot be redefined.

Runtime errors stop the program: print `error line N: MESSAGE` on standard output and exit with status 1 (output printed before stays). N is the line of the token that caused the error, as follows. For `unbound variable NAME` it is the line of the symbol. For everything else it is the line of the `(` of the innermost list being evaluated when the problem arose. Messages: `unbound variable NAME` (also `set!` of an unbound name), `not a function` (calling a non-closure), `wrong number of arguments` (closure call), `type error` (wrong operand types or a non-boolean condition), `division by zero` (`/` or `%`), `overflow` (an integer result with absolute value above 10^15, checked after every `+`, `-`, `*` step), `bad syntax` (malformed special form: wrong number of parts, parameters or bindings that are not symbols, empty body, reserved or duplicate parameter or `let` binding names, `while` without a body form, comparison or `/` with a wrong operand count), `stack overflow` (a closure call that would be the 101st nested active call), `step limit` (more than 200000 evaluation steps in total, one step per list evaluation).

Checks happen in this order for a list form: syntax first (operand counts and shapes), then evaluation of operands left to right, then operand types. For a call: the first element is evaluated and must be a closure (`not a function`) before any argument is evaluated; then the arguments are evaluated; then the argument count is checked; then the call depth is checked. Each evaluation of a list form counts as one step before any of its checks, so a form whose step would be number 200001 reports `step limit` at its own line.

Example:

```
(define make-counter
  (lambda ()
    (let ((n 0))
      (lambda () (set! n (+ n 1)) n))))
(define c (make-counter))
(c)
(print (c))
(print (c))
(print (/ 1 0))
```

prints

```
2
3
error line 9: division by zero
```

and exits with status 1.

A second example, with a parse error:

```
(print 1)
(define f (lambda (x)
  (* x 2))
(print (f 4))
```

prints `parse error line 2: unclosed (` and exits with status 2; `1` is not printed because nothing runs.

A third example, showing that closures share variables rather than copy them:

```
(define x 1)
(define show (lambda () x))
(define x 2)
(print (show))
(let ((x 10)) (print (show)))
(print (set! x 7))
(print x)
```

prints

```
2
2
2
7
```
