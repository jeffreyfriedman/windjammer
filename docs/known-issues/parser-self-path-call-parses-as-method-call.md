# `self::f()` parses identically to `self.f()`

Repro (any module):

```
fn up() {}
fn f() { self::up() }
```

The parser produces `Expression::MethodCall { object: self, method: "up" }` for `self::up()`, so the
AST cannot distinguish a module-relative call (`self::up`) from a method call on the receiver. The
resolver therefore reports it as a method call with candidates by name/arity. `use self::x` is
unaffected (it goes through `parse_use`). Fix belongs in the primary-expression parser: keep
`self::` followed by an identifier as a qualified path `Identifier("self::up")`.
