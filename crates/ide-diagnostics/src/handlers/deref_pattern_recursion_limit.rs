use hir::HirDisplay;

use crate::{Diagnostic, DiagnosticCode, DiagnosticsContext};

// Diagnostic: deref-pattern-recursion-limit
//
// This diagnostic is triggered when implicitly dereferencing a pattern reaches
// the recursion limit.
pub(crate) fn deref_pattern_recursion_limit(
    ctx: &DiagnosticsContext<'_, '_>,
    d: &hir::DerefPatternRecursionLimit<'_>,
) -> Diagnostic {
    Diagnostic::new_with_syntax_node_ptr(
        ctx,
        DiagnosticCode::RustcHardError("E0055"),
        format!(
            "reached the recursion limit while auto-dereferencing `{}`",
            d.found.display(ctx.sema.db, ctx.display_target)
        ),
        d.pat.map(Into::into),
    )
    .stable()
}

#[cfg(test)]
mod tests {
    use crate::tests::check_diagnostics;

    #[test]
    fn deref_pattern_recursion_limit() {
        check_diagnostics(
            r#"
//- minicore: deref_pat
#![feature(deref_patterns, deref_pure_trait)]
#![recursion_limit = "4"]

struct Recursive;
impl core::ops::Deref for Recursive {
    type Target = Self;
    fn deref(&self) -> &Self { self }
}
unsafe impl core::ops::DerefPure for Recursive {}

fn f((): Recursive) {}
   //^^ error: reached the recursion limit while auto-dereferencing `Recursive`
"#,
        );
    }

    #[test]
    fn finite_deref_pattern() {
        check_diagnostics(
            r#"
//- minicore: deref_pat
#![feature(deref_patterns, deref_pure_trait)]
#![recursion_limit = "4"]

struct Wrapper<T>(T);
impl<T> core::ops::Deref for Wrapper<T> {
    type Target = T;
    fn deref(&self) -> &T { &self.0 }
}
unsafe impl<T> core::ops::DerefPure for Wrapper<T> {}

fn f((): Wrapper<Wrapper<()>>) {}
"#,
        );
    }

    #[test]
    fn builtin_ref_pattern_with_low_recursion_limit() {
        check_diagnostics(
            r#"
#![recursion_limit = "4"]
fn f((): &&&&&&()) {}
"#,
        );
    }
}
