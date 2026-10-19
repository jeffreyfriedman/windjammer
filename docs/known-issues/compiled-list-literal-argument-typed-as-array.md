# Compiled: a list literal `["a", "b"]` passed to a `Vec<string>` parameter is emitted as a borrowed array

```windjammer
fn count(v: Vec<string>) -> int {
    v.len()
}

fn main() {
    println("${count(["a", "b"])}")
}
```

`wj build` emits `count(&["a", "b"])` against `fn count(v: &Vec<String>)` (rustc `E0308: expected &Vec<String>, found
&[&str; 2]`). `[..]` is the documented list literal (`vec!` is deprecated, W0007), so the literal must take the parameter's
type (`Vec<string>`, elements `string`) in argument position. Same for `Vec<int>` (`[i32; N]` found).

Status: fixed. A list literal in argument position takes the formal's type (`Vec<T>` / `&Vec<T>`) and lowers to `vec![..]`. Gate: `ecosystem_findings_test::list_literal_argument_takes_vec_parameter_type`.
