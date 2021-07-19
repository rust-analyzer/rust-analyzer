use ide_db::source_change::SourceChange;
use syntax::{
    ast::{self, ElseBranch},
    AstNode, SyntaxNode, TextRange,
};
use text_edit::TextEdit;

use crate::{fix, Diagnostic, FileId, Severity};

// Diagnostic: remove-unnecessary-else
//
// This diagnostic is triggered when we have an `else` block following an `if` block which ends in a `return` statement.
// (or a `continue`/`break` if inside a loop).
pub(crate) fn unnecessary_else(
    acc: &mut Vec<Diagnostic>,
    file_id: FileId,
    node: &SyntaxNode,
) -> Option<()> {
    let if_expr = ast::IfExpr::cast(node.clone())?;
    if let Some(else_branch) = if_expr.else_branch() {
        match else_branch {
            ElseBranch::Block(else_block) => {
                let else_token_range = if_expr.else_token().unwrap().text_range();
                let else_block_range = TextRange::new(
                    else_token_range.start(),
                    else_block.syntax().text_range().end(),
                );

                let then_branch = if_expr.then_branch()?;
                let then_r_curly_token = then_branch.syntax().last_child_or_token()?;
                let del_range =
                    TextRange::new(then_r_curly_token.text_range().end(), else_block_range.end());

                let else_block_str = else_block.to_string();
                let trim_pat: &[_] = &[' ', '{'];
                let else_expr_str = else_block_str.trim_start_matches(trim_pat);
                let mut else_expr_str_lines: Vec<&str> = else_expr_str.lines().collect();
                else_expr_str_lines.pop();
                let new_expr_str = else_expr_str_lines
                    .iter()
                    .map(|line| line.replacen("    ", "", 1))
                    .collect::<Vec<String>>()
                    .join("\n");

                let mut edit_builder = TextEdit::builder();
                edit_builder.replace(del_range, new_expr_str);
                let edit = edit_builder.finish();

                acc.push(
                    Diagnostic::new(
                        "unnecessary-else",
                        "Unnecessary else in if-else expression".to_string(),
                        else_token_range,
                    )
                    .severity(Severity::WeakWarning)
                    .with_fixes(Some(vec![fix(
                        "remove_unnecessary_else",
                        "Remove unnecessary else",
                        SourceChange::from_text_edit(file_id, edit),
                        else_block_range,
                    )])),
                );
            }
            _ => (),
        }
    }

    Some(())
}

#[cfg(test)]
mod tests {
    use crate::tests::{check_diagnostics, check_fix};

    #[test]
    fn unnecessary_else() {
        check_diagnostics(
            r#"
fn test() {
    if foo {
        return bar;
    } else {
    //^^^^ 💡 weak: Unnecessary else in if-else expression
        do_something_else();
    }
}
"#,
        );
    }

    #[test]
    fn unnecessary_else_break() {
        check_diagnostics(
            r#"
fn test() {
    let mut x = 5;
    while true {
        if x == 0 {
            break;
        } else {
        //^^^^ 💡 weak: Unnecessary else in if-else expression
            x -= 1;
        }
    }
}
"#,
        );
    }

    #[test]
    fn unnecessary_else_no_diagnostic() {
        check_diagnostics(
            r#"
fn test() {
    if foo {
        return bar;
    }

    do_something_else();
}
"#,
        );
    }

    #[test]
    fn remove_unnecessary_else() {
        check_fix(
            r#"
fn test() {
    if foo {
        return bar;
    } $0else {
        do_this();
        do_something_else();
    }
}
"#,
            r#"
fn test() {
    if foo {
        return bar;
    }
    do_this();
    do_something_else();
}
"#,
        );
    }

    #[test]
    fn remove_unnecessary_else_break() {
        check_fix(
            r#"
fn test() {
    let mut x = 5;
    while true {
        if x == 0 {
            break;
        } $0else {
            x -= 1;
        }
    }
}
"#,
            r#"
fn test() {
    let mut x = 5;
    while true {
        if x == 0 {
            break;
        }
        x -= 1;
    }
}
"#,
        );
    }

    #[test]
    fn remove_unnecessary_else_nested() {
        check_fix(
            r#"
fn test() {
    let x = {
        if foo {
            return bar;
        } $0else {
            do_this();
            do_something_else();
        }
    }
}
"#,
            r#"
fn test() {
    let x = {
        if foo {
            return bar;
        }
        do_this();
        do_something_else();
    }
}
"#,
        );
    }
}
