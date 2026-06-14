//! SIMD loop emission for the Rust backend (`std::arch` + scalar tail).
//!
//! See `crate::analyzer::simd_loops` for legality / pattern detection.

use crate::analyzer::simd_loops::{
    analyze_for_loop_simd, half_open_range_from_zero, SimdLoopPattern,
};
use crate::codegen::rust::CodeGenerator;
use crate::parser::{Expression, Pattern, Statement};
use crate::CompilationTarget;

/// If the `for` loop matches a SIMD pattern, emit an arch-specific block; otherwise `None`.
pub fn try_emit_simd_for_loop<'ast>(
    gen: &mut CodeGenerator<'ast>,
    pattern: &Pattern<'ast>,
    iterable: &'ast Expression<'ast>,
    body: &[&'ast Statement<'ast>],
) -> Option<String> {
    if gen.target != CompilationTarget::Rust {
        return None;
    }
    let simd_pat = analyze_for_loop_simd(pattern, iterable, body)?;
    let upper = half_open_range_from_zero(iterable)?;
    let bound_rust = gen.generate_expression(upper);

    match simd_pat {
        SimdLoopPattern::F32DotProductAccum {
            loop_var,
            accum_var,
            slice_a,
            slice_b,
        } => Some(emit_f32_dot_product(
            gen,
            &bound_rust,
            &loop_var,
            &accum_var,
            &slice_a,
            &slice_b,
        )),
        SimdLoopPattern::F32ArrayAddInto {
            loop_var,
            out_array,
            slice_a,
            slice_b,
        } => Some(emit_f32_array_add(
            gen,
            &bound_rust,
            &loop_var,
            &out_array,
            &slice_a,
            &slice_b,
        )),
    }
}

fn emit_f32_dot_product(
    gen: &mut CodeGenerator<'_>,
    bound_rust: &str,
    loop_var: &str,
    accum_var: &str,
    slice_a: &str,
    slice_b: &str,
) -> String {
    let ind = gen.indent();
    let mut out = String::new();
    out.push_str(&ind);
    out.push_str("// windjammer simd: f32 dot-product reduction (4-wide + scalar tail)\n");
    out.push_str(&ind);
    out.push_str("{\n");
    gen.indent_level += 1;
    let i1 = gen.indent();
    out.push_str(&i1);
    out.push_str(&format!(
        "let __wj_len = ({bound}).min({a}.len()).min({b}.len());\n",
        bound = bound_rust,
        a = slice_a,
        b = slice_b
    ));
    out.push_str(&i1);
    out.push_str("#[cfg(target_arch = \"x86_64\")]\n");
    out.push_str(&i1);
    out.push_str("{\n");
    gen.indent_level += 1;
    let i2 = gen.indent();
    out.push_str(&i2);
    out.push_str("unsafe {\n");
    gen.indent_level += 1;
    let i3 = gen.indent();
    out.push_str(&i3);
    out.push_str("use std::arch::x86_64::*;\n");
    out.push_str(&i3);
    out.push_str(&format!("let __wj_ap = {a}.as_ptr();\n", a = slice_a));
    out.push_str(&i3);
    out.push_str(&format!("let __wj_bp = {b}.as_ptr();\n", b = slice_b));
    out.push_str(&i3);
    out.push_str("if std::arch::is_x86_feature_detected!(\"avx\") {\n");
    gen.indent_level += 1;
    let iax = gen.indent();
    out.push_str(&iax);
    out.push_str("let __wj_chunks_avx = __wj_len / 8_usize;\n");
    out.push_str(&iax);
    out.push_str("let mut __wj_vacc8 = _mm256_setzero_ps();\n");
    out.push_str(&iax);
    out.push_str("for __wj_c in 0..__wj_chunks_avx {\n");
    gen.indent_level += 1;
    let iax2 = gen.indent();
    out.push_str(&iax2);
    out.push_str("let __wj_o = __wj_c * 8_usize;\n");
    out.push_str(&iax2);
    out.push_str("let __wj_va = _mm256_loadu_ps(__wj_ap.add(__wj_o));\n");
    out.push_str(&iax2);
    out.push_str("let __wj_vb = _mm256_loadu_ps(__wj_bp.add(__wj_o));\n");
    out.push_str(&iax2);
    out.push_str("__wj_vacc8 = _mm256_add_ps(__wj_vacc8, _mm256_mul_ps(__wj_va, __wj_vb));\n");
    gen.indent_level -= 1;
    out.push_str(&gen.indent());
    out.push_str("}\n");
    out.push_str(&iax);
    out.push_str("let __wj_parts8: [f32; 8] = std::mem::transmute(__wj_vacc8);\n");
    out.push_str(&iax);
    out.push_str(&format!(
        "{acc} += __wj_parts8[0] + __wj_parts8[1] + __wj_parts8[2] + __wj_parts8[3]\n\
         + __wj_parts8[4] + __wj_parts8[5] + __wj_parts8[6] + __wj_parts8[7];\n",
        acc = accum_var
    ));
    out.push_str(&iax);
    out.push_str(&format!(
        "for {lv} in (__wj_chunks_avx * 8_usize)..__wj_len {{\n",
        lv = loop_var
    ));
    gen.indent_level += 1;
    out.push_str(&gen.indent());
    out.push_str(&format!(
        "{acc} += {a}[{lv}] * {b}[{lv}];\n",
        acc = accum_var,
        a = slice_a,
        b = slice_b,
        lv = loop_var
    ));
    gen.indent_level -= 1;
    out.push_str(&gen.indent());
    out.push_str("}\n");
    gen.indent_level -= 1;
    out.push_str(&gen.indent());
    out.push_str("} else {\n");
    gen.indent_level += 1;
    let sse = gen.indent();
    out.push_str(&sse);
    out.push_str("let __wj_chunks = __wj_len / 4_usize;\n");
    out.push_str(&sse);
    out.push_str("let mut __wj_vacc = _mm_setzero_ps();\n");
    out.push_str(&sse);
    out.push_str("for __wj_c in 0..__wj_chunks {\n");
    gen.indent_level += 1;
    let i4 = gen.indent();
    out.push_str(&i4);
    out.push_str("let __wj_o = __wj_c * 4_usize;\n");
    out.push_str(&i4);
    out.push_str("let __wj_va = _mm_loadu_ps(__wj_ap.add(__wj_o));\n");
    out.push_str(&i4);
    out.push_str("let __wj_vb = _mm_loadu_ps(__wj_bp.add(__wj_o));\n");
    out.push_str(&i4);
    out.push_str("__wj_vacc = _mm_add_ps(__wj_vacc, _mm_mul_ps(__wj_va, __wj_vb));\n");
    gen.indent_level -= 1;
    out.push_str(&gen.indent());
    out.push_str("}\n");
    out.push_str(&sse);
    out.push_str("let __wj_parts: [f32; 4] = std::mem::transmute(__wj_vacc);\n");
    out.push_str(&sse);
    out.push_str(&format!(
        "{acc} += __wj_parts[0] + __wj_parts[1] + __wj_parts[2] + __wj_parts[3];\n",
        acc = accum_var
    ));
    out.push_str(&sse);
    out.push_str(&format!(
        "for {lv} in (__wj_chunks * 4_usize)..__wj_len {{\n",
        lv = loop_var
    ));
    gen.indent_level += 1;
    out.push_str(&gen.indent());
    out.push_str(&format!(
        "{acc} += {a}[{lv}] * {b}[{lv}];\n",
        acc = accum_var,
        a = slice_a,
        b = slice_b,
        lv = loop_var
    ));
    gen.indent_level -= 1;
    out.push_str(&gen.indent());
    out.push_str("}\n");
    gen.indent_level -= 1;
    out.push_str(&gen.indent());
    out.push_str("}\n");
    gen.indent_level -= 1;
    out.push_str(&gen.indent());
    out.push_str("}\n");
    gen.indent_level -= 1;
    out.push_str(&gen.indent());
    out.push_str("}\n");

    out.push_str(&i1);
    out.push_str("#[cfg(target_arch = \"aarch64\")]\n");
    out.push_str(&i1);
    out.push_str("{\n");
    gen.indent_level += 1;
    let ia = gen.indent();
    out.push_str(&ia);
    out.push_str("unsafe {\n");
    gen.indent_level += 1;
    let ia2 = gen.indent();
    out.push_str(&ia2);
    out.push_str("use std::arch::aarch64::*;\n");
    out.push_str(&ia2);
    out.push_str(&format!("let __wj_ap = {a}.as_ptr();\n", a = slice_a));
    out.push_str(&ia2);
    out.push_str(&format!("let __wj_bp = {b}.as_ptr();\n", b = slice_b));
    out.push_str(&ia2);
    out.push_str("let __wj_chunks = __wj_len / 4_usize;\n");
    out.push_str(&ia2);
    out.push_str("let mut __wj_vacc = vdupq_n_f32(0.0);\n");
    out.push_str(&ia2);
    out.push_str("for __wj_c in 0..__wj_chunks {\n");
    gen.indent_level += 1;
    let ia3 = gen.indent();
    out.push_str(&ia3);
    out.push_str("let __wj_o = __wj_c * 4_usize;\n");
    out.push_str(&ia3);
    out.push_str("let __wj_va = vld1q_f32(__wj_ap.add(__wj_o));\n");
    out.push_str(&ia3);
    out.push_str("let __wj_vb = vld1q_f32(__wj_bp.add(__wj_o));\n");
    out.push_str(&ia3);
    out.push_str("__wj_vacc = vfmaq_f32(__wj_vacc, __wj_va, __wj_vb);\n");
    gen.indent_level -= 1;
    out.push_str(&gen.indent());
    out.push_str("}\n");
    out.push_str(&ia2);
    out.push_str("let __wj_parts: [f32; 4] = std::mem::transmute(__wj_vacc);\n");
    out.push_str(&ia2);
    out.push_str(&format!(
        "{acc} += __wj_parts[0] + __wj_parts[1] + __wj_parts[2] + __wj_parts[3];\n",
        acc = accum_var
    ));
    out.push_str(&ia2);
    out.push_str(&format!(
        "for {lv} in (__wj_chunks * 4_usize)..__wj_len {{\n",
        lv = loop_var
    ));
    gen.indent_level += 1;
    out.push_str(&gen.indent());
    out.push_str(&format!(
        "{acc} += {a}[{lv}] * {b}[{lv}];\n",
        acc = accum_var,
        a = slice_a,
        b = slice_b,
        lv = loop_var
    ));
    gen.indent_level -= 1;
    out.push_str(&gen.indent());
    out.push_str("}\n");
    gen.indent_level -= 1;
    out.push_str(&gen.indent());
    out.push_str("}\n");
    gen.indent_level -= 1;
    out.push_str(&gen.indent());
    out.push_str("}\n");

    out.push_str(&i1);
    out.push_str("#[cfg(not(any(target_arch = \"x86_64\", target_arch = \"aarch64\")))]\n");
    out.push_str(&i1);
    out.push_str("{\n");
    gen.indent_level += 1;
    out.push_str(&gen.indent());
    out.push_str(&format!("for {lv} in 0..__wj_len {{\n", lv = loop_var));
    gen.indent_level += 1;
    out.push_str(&gen.indent());
    out.push_str(&format!(
        "{acc} += {a}[{lv}] * {b}[{lv}];\n",
        acc = accum_var,
        a = slice_a,
        b = slice_b,
        lv = loop_var
    ));
    gen.indent_level -= 1;
    out.push_str(&gen.indent());
    out.push_str("}\n");
    gen.indent_level -= 1;
    out.push_str(&gen.indent());
    out.push_str("}\n");

    gen.indent_level -= 1;
    out.push_str(&gen.indent());
    out.push_str("}\n");
    out
}

fn emit_f32_array_add(
    gen: &mut CodeGenerator<'_>,
    bound_rust: &str,
    loop_var: &str,
    out_arr: &str,
    slice_a: &str,
    slice_b: &str,
) -> String {
    let ind = gen.indent();
    let mut out = String::new();
    out.push_str(&ind);
    out.push_str("// windjammer simd: f32 element-wise add (4-wide + scalar tail)\n");
    out.push_str(&ind);
    out.push_str("{\n");
    gen.indent_level += 1;
    let i1 = gen.indent();
    out.push_str(&i1);
    out.push_str(&format!(
        "let __wj_len = ({bound}).min({out}.len()).min({a}.len()).min({b}.len());\n",
        bound = bound_rust,
        out = out_arr,
        a = slice_a,
        b = slice_b
    ));
    out.push_str(&i1);
    out.push_str("#[cfg(target_arch = \"x86_64\")]\n");
    out.push_str(&i1);
    out.push_str("{\n");
    gen.indent_level += 1;
    let i2 = gen.indent();
    out.push_str(&i2);
    out.push_str("unsafe {\n");
    gen.indent_level += 1;
    let i3 = gen.indent();
    out.push_str(&i3);
    out.push_str("use std::arch::x86_64::*;\n");
    out.push_str(&i3);
    out.push_str(&format!(
        "let __wj_op = {out}.as_mut_ptr();\n",
        out = out_arr
    ));
    out.push_str(&i3);
    out.push_str(&format!("let __wj_ap = {a}.as_ptr();\n", a = slice_a));
    out.push_str(&i3);
    out.push_str(&format!("let __wj_bp = {b}.as_ptr();\n", b = slice_b));
    out.push_str(&i3);
    out.push_str("if std::arch::is_x86_feature_detected!(\"avx\") {\n");
    gen.indent_level += 1;
    let iax = gen.indent();
    out.push_str(&iax);
    out.push_str("let __wj_chunks_avx = __wj_len / 8_usize;\n");
    out.push_str(&iax);
    out.push_str("for __wj_c in 0..__wj_chunks_avx {\n");
    gen.indent_level += 1;
    let iax2 = gen.indent();
    out.push_str(&iax2);
    out.push_str("let __wj_o = __wj_c * 8_usize;\n");
    out.push_str(&iax2);
    out.push_str("let __wj_va = _mm256_loadu_ps(__wj_ap.add(__wj_o));\n");
    out.push_str(&iax2);
    out.push_str("let __wj_vb = _mm256_loadu_ps(__wj_bp.add(__wj_o));\n");
    out.push_str(&iax2);
    out.push_str("let __wj_vs = _mm256_add_ps(__wj_va, __wj_vb);\n");
    out.push_str(&iax2);
    out.push_str("_mm256_storeu_ps(__wj_op.add(__wj_o), __wj_vs);\n");
    gen.indent_level -= 1;
    out.push_str(&gen.indent());
    out.push_str("}\n");
    out.push_str(&iax);
    out.push_str(&format!(
        "for {lv} in (__wj_chunks_avx * 8_usize)..__wj_len {{\n",
        lv = loop_var
    ));
    gen.indent_level += 1;
    out.push_str(&gen.indent());
    out.push_str(&format!(
        "{out}[{lv}] = {a}[{lv}] + {b}[{lv}];\n",
        out = out_arr,
        a = slice_a,
        b = slice_b,
        lv = loop_var
    ));
    gen.indent_level -= 1;
    out.push_str(&gen.indent());
    out.push_str("}\n");
    gen.indent_level -= 1;
    out.push_str(&gen.indent());
    out.push_str("} else {\n");
    gen.indent_level += 1;
    let sse = gen.indent();
    out.push_str(&sse);
    out.push_str("let __wj_chunks = __wj_len / 4_usize;\n");
    out.push_str(&sse);
    out.push_str("for __wj_c in 0..__wj_chunks {\n");
    gen.indent_level += 1;
    let i4 = gen.indent();
    out.push_str(&i4);
    out.push_str("let __wj_o = __wj_c * 4_usize;\n");
    out.push_str(&i4);
    out.push_str("let __wj_va = _mm_loadu_ps(__wj_ap.add(__wj_o));\n");
    out.push_str(&i4);
    out.push_str("let __wj_vb = _mm_loadu_ps(__wj_bp.add(__wj_o));\n");
    out.push_str(&i4);
    out.push_str("let __wj_vs = _mm_add_ps(__wj_va, __wj_vb);\n");
    out.push_str(&i4);
    out.push_str("_mm_storeu_ps(__wj_op.add(__wj_o), __wj_vs);\n");
    gen.indent_level -= 1;
    out.push_str(&gen.indent());
    out.push_str("}\n");
    out.push_str(&sse);
    out.push_str(&format!(
        "for {lv} in (__wj_chunks * 4_usize)..__wj_len {{\n",
        lv = loop_var
    ));
    gen.indent_level += 1;
    out.push_str(&gen.indent());
    out.push_str(&format!(
        "{out}[{lv}] = {a}[{lv}] + {b}[{lv}];\n",
        out = out_arr,
        a = slice_a,
        b = slice_b,
        lv = loop_var
    ));
    gen.indent_level -= 1;
    out.push_str(&gen.indent());
    out.push_str("}\n");
    gen.indent_level -= 1;
    out.push_str(&gen.indent());
    out.push_str("}\n");
    gen.indent_level -= 1;
    out.push_str(&gen.indent());
    out.push_str("}\n");
    gen.indent_level -= 1;
    out.push_str(&gen.indent());
    out.push_str("}\n");

    out.push_str(&i1);
    out.push_str("#[cfg(target_arch = \"aarch64\")]\n");
    out.push_str(&i1);
    out.push_str("{\n");
    gen.indent_level += 1;
    let ia = gen.indent();
    out.push_str(&ia);
    out.push_str("unsafe {\n");
    gen.indent_level += 1;
    let ia2 = gen.indent();
    out.push_str(&ia2);
    out.push_str("use std::arch::aarch64::*;\n");
    out.push_str(&ia2);
    out.push_str(&format!(
        "let __wj_op = {out}.as_mut_ptr();\n",
        out = out_arr
    ));
    out.push_str(&ia2);
    out.push_str(&format!("let __wj_ap = {a}.as_ptr();\n", a = slice_a));
    out.push_str(&ia2);
    out.push_str(&format!("let __wj_bp = {b}.as_ptr();\n", b = slice_b));
    out.push_str(&ia2);
    out.push_str("let __wj_chunks = __wj_len / 4_usize;\n");
    out.push_str(&ia2);
    out.push_str("for __wj_c in 0..__wj_chunks {\n");
    gen.indent_level += 1;
    let ia3 = gen.indent();
    out.push_str(&ia3);
    out.push_str("let __wj_o = __wj_c * 4_usize;\n");
    out.push_str(&ia3);
    out.push_str("let __wj_va = vld1q_f32(__wj_ap.add(__wj_o));\n");
    out.push_str(&ia3);
    out.push_str("let __wj_vb = vld1q_f32(__wj_bp.add(__wj_o));\n");
    out.push_str(&ia3);
    out.push_str("let __wj_vs = vaddq_f32(__wj_va, __wj_vb);\n");
    out.push_str(&ia3);
    out.push_str("vst1q_f32(__wj_op.add(__wj_o), __wj_vs);\n");
    gen.indent_level -= 1;
    out.push_str(&gen.indent());
    out.push_str("}\n");
    out.push_str(&ia2);
    out.push_str(&format!(
        "for {lv} in (__wj_chunks * 4_usize)..__wj_len {{\n",
        lv = loop_var
    ));
    gen.indent_level += 1;
    out.push_str(&gen.indent());
    out.push_str(&format!(
        "{out}[{lv}] = {a}[{lv}] + {b}[{lv}];\n",
        out = out_arr,
        a = slice_a,
        b = slice_b,
        lv = loop_var
    ));
    gen.indent_level -= 1;
    out.push_str(&gen.indent());
    out.push_str("}\n");
    gen.indent_level -= 1;
    out.push_str(&gen.indent());
    out.push_str("}\n");
    gen.indent_level -= 1;
    out.push_str(&gen.indent());
    out.push_str("}\n");

    out.push_str(&i1);
    out.push_str("#[cfg(not(any(target_arch = \"x86_64\", target_arch = \"aarch64\")))]\n");
    out.push_str(&i1);
    out.push_str("{\n");
    gen.indent_level += 1;
    out.push_str(&gen.indent());
    out.push_str(&format!("for {lv} in 0..__wj_len {{\n", lv = loop_var));
    gen.indent_level += 1;
    out.push_str(&gen.indent());
    out.push_str(&format!(
        "{out}[{lv}] = {a}[{lv}] + {b}[{lv}];\n",
        out = out_arr,
        a = slice_a,
        b = slice_b,
        lv = loop_var
    ));
    gen.indent_level -= 1;
    out.push_str(&gen.indent());
    out.push_str("}\n");
    gen.indent_level -= 1;
    out.push_str(&gen.indent());
    out.push_str("}\n");

    gen.indent_level -= 1;
    out.push_str(&gen.indent());
    out.push_str("}\n");
    out
}
