use crate::{Diagnostic, DiagnosticCode, DiagnosticsContext};

// Diagnostic: expected-trait
//
// This diagnostic is triggered when a qualified path uses a non-trait definition as a trait.
pub(crate) fn expected_trait(
    ctx: &DiagnosticsContext<'_, '_>,
    d: &hir::ExpectedTrait,
) -> Diagnostic {
    let name = d
        .def
        .name(ctx.sema.db)
        .map(|name| format!(" `{}`", name.display(ctx.sema.db, ctx.edition)))
        .unwrap_or_default();
    Diagnostic::new_with_syntax_node_ptr(
        ctx,
        DiagnosticCode::RustcHardError("E0404"),
        format!("expected trait, found {}{name}", d.def.description()),
        d.segment.map(Into::into),
    )
    .stable()
}

#[cfg(test)]
mod tests {
    use crate::tests::check_diagnostics;

    #[test]
    fn expected_trait_qualified_path() {
        check_diagnostics(
            r#"
struct S;
impl S { fn item() {} }
fn f() {
    <S as S>::item();
       // ^ error: expected trait, found struct `S`
}
"#,
        );
    }

    #[test]
    fn non_trait_kinds() {
        check_diagnostics(
            r#"
enum E {}
union U { field: u8 }
type Alias = E;
type A = <() as E>::Assoc;
             // ^ error: expected trait, found enum `E`
type B = <() as U>::Assoc;
             // ^ error: expected trait, found union `U`
type C = <() as Alias>::Assoc;
             // ^^^^^ error: expected trait, found type alias `Alias`
"#,
        );
    }

    #[test]
    fn qualified_generic_struct() {
        check_diagnostics(
            r#"
mod module {
    pub struct S<T>(T);
    impl<T> S<T> { pub fn item() {} }
}
fn f() {
    <() as module::S<u8>>::item();
                // ^^^^^ error: expected trait, found struct `S`
}
"#,
        );
    }

    #[test]
    fn valid_qualified_trait_path() {
        check_diagnostics(
            r#"
trait Trait { type Assoc; fn item(); }
struct S;
impl Trait for S { type Assoc = (); fn item() {} }
impl S { fn inherent() {} }
type A = <S as Trait>::Assoc;
fn f() {
    <S as Trait>::item();
    S::item();
    <S>::inherent();
}
"#,
        );
    }
}
