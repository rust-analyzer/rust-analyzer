//! This module generates AST datatype used by rust-analyzer.
//!
//! Specifically, it generates the `SyntaxKind` enum and a number of newtype
//! wrappers around `SyntaxNode` which implement `syntax::AstNode`.

use std::{
    borrow::Cow,
    collections::{BTreeSet, HashSet},
    fmt::Write,
};

use proc_macro2::{Punct, Spacing};
use quote::{format_ident, quote};
use ungrammar::{rust_grammar, Grammar, Rule};

use crate::{
    ast_src::{AstEnumSrc, AstNodeSrc, AstSrc, Cardinality, Field, KindsSrc, KINDS_SRC},
    codegen::{reformat, update, Mode},
    project_root, Result,
};

pub fn generate_syntax(mode: Mode) -> Result<()> {
    let grammar = rust_grammar();
    let ast = lower(&grammar);

    let syntax_kinds_file = project_root().join("crates/parser/src/syntax_kind/generated.rs");
    let syntax_kinds = generate_syntax_kinds(KINDS_SRC)?;
    update(syntax_kinds_file.as_path(), &syntax_kinds, mode)?;

    let ast_tokens_file = project_root().join("crates/syntax/src/ast/generated/tokens.rs");
    let contents = generate_tokens(&ast)?;
    update(ast_tokens_file.as_path(), &contents, mode)?;

    let ast_nodes_file = project_root().join("crates/syntax/src/ast/generated/nodes.rs");
    let contents = generate_nodes(KINDS_SRC, &ast)?;
    update(ast_nodes_file.as_path(), &contents, mode)?;

    let make_file = project_root().join("crates/syntax/src/ast/generated/make.rs");
    let contents = generate_make(&grammar)?;
    update(make_file.as_path(), &contents, mode)?;

    Ok(())
}

fn generate_tokens(grammar: &AstSrc) -> Result<String> {
    let tokens = grammar.tokens.iter().map(|token| {
        let name = format_ident!("{}", token);
        let kind = format_ident!("{}", to_upper_snake_case(token));
        quote! {
            #[derive(Debug, Clone, PartialEq, Eq, Hash)]
            pub struct #name {
                pub(crate) syntax: SyntaxToken,
            }
            impl std::fmt::Display for #name {
                fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                    std::fmt::Display::fmt(&self.syntax, f)
                }
            }
            impl AstToken for #name {
                fn can_cast(kind: SyntaxKind) -> bool { kind == #kind }
                fn cast(syntax: SyntaxToken) -> Option<Self> {
                    if Self::can_cast(syntax.kind()) { Some(Self { syntax }) } else { None }
                }
                fn syntax(&self) -> &SyntaxToken { &self.syntax }
            }
        }
    });

    let pretty = reformat(
        &quote! {
            use crate::{SyntaxKind::{self, *}, SyntaxToken, ast::AstToken};
            #(#tokens)*
        }
        .to_string(),
    )?
    .replace("#[derive", "\n#[derive");
    Ok(pretty)
}

fn generate_nodes(kinds: KindsSrc<'_>, grammar: &AstSrc) -> Result<String> {
    let (node_defs, node_boilerplate_impls): (Vec<_>, Vec<_>) = grammar
        .nodes
        .iter()
        .map(|node| {
            let name = format_ident!("{}", node.name);
            let kind = format_ident!("{}", to_upper_snake_case(&node.name));
            let traits = node.traits.iter().map(|trait_name| {
                let trait_name = format_ident!("{}", trait_name);
                quote!(impl ast::#trait_name for #name {})
            });

            let methods = node.fields.iter().map(|field| {
                let method_name = field.method_name();
                let ty = field.ty();

                if field.is_many() {
                    quote! {
                        pub fn #method_name(&self) -> AstChildren<#ty> {
                            support::children(&self.syntax)
                        }
                    }
                } else {
                    if let Some(token_kind) = field.token_kind() {
                        quote! {
                            pub fn #method_name(&self) -> Option<#ty> {
                                support::token(&self.syntax, #token_kind)
                            }
                        }
                    } else {
                        quote! {
                            pub fn #method_name(&self) -> Option<#ty> {
                                support::child(&self.syntax)
                            }
                        }
                    }
                }
            });
            (
                quote! {
                    #[pretty_doc_comment_placeholder_workaround]
                    #[derive(Debug, Clone, PartialEq, Eq, Hash)]
                    pub struct #name {
                        pub(crate) syntax: SyntaxNode,
                    }

                    #(#traits)*

                    impl #name {
                        #(#methods)*
                    }
                },
                quote! {
                    impl AstNode for #name {
                        fn can_cast(kind: SyntaxKind) -> bool {
                            kind == #kind
                        }
                        fn cast(syntax: SyntaxNode) -> Option<Self> {
                            if Self::can_cast(syntax.kind()) { Some(Self { syntax }) } else { None }
                        }
                        fn syntax(&self) -> &SyntaxNode { &self.syntax }
                    }
                },
            )
        })
        .unzip();

    let (enum_defs, enum_boilerplate_impls): (Vec<_>, Vec<_>) = grammar
        .enums
        .iter()
        .map(|en| {
            let variants: Vec<_> = en.variants.iter().map(|var| format_ident!("{}", var)).collect();
            let name = format_ident!("{}", en.name);
            let kinds: Vec<_> = variants
                .iter()
                .map(|name| format_ident!("{}", to_upper_snake_case(&name.to_string())))
                .collect();
            let traits = en.traits.iter().map(|trait_name| {
                let trait_name = format_ident!("{}", trait_name);
                quote!(impl ast::#trait_name for #name {})
            });

            let ast_node = if en.name == "Stmt" {
                quote! {}
            } else {
                quote! {
                    impl AstNode for #name {
                        fn can_cast(kind: SyntaxKind) -> bool {
                            match kind {
                                #(#kinds)|* => true,
                                _ => false,
                            }
                        }
                        fn cast(syntax: SyntaxNode) -> Option<Self> {
                            let res = match syntax.kind() {
                                #(
                                #kinds => #name::#variants(#variants { syntax }),
                                )*
                                _ => return None,
                            };
                            Some(res)
                        }
                        fn syntax(&self) -> &SyntaxNode {
                            match self {
                                #(
                                #name::#variants(it) => &it.syntax,
                                )*
                            }
                        }
                    }
                }
            };

            (
                quote! {
                    #[pretty_doc_comment_placeholder_workaround]
                    #[derive(Debug, Clone, PartialEq, Eq, Hash)]
                    pub enum #name {
                        #(#variants(#variants),)*
                    }

                    #(#traits)*
                },
                quote! {
                    #(
                        impl From<#variants> for #name {
                            fn from(node: #variants) -> #name {
                                #name::#variants(node)
                            }
                        }
                    )*
                    #ast_node
                },
            )
        })
        .unzip();

    let enum_names = grammar.enums.iter().map(|it| &it.name);
    let node_names = grammar.nodes.iter().map(|it| &it.name);

    let display_impls =
        enum_names.chain(node_names.clone()).map(|it| format_ident!("{}", it)).map(|name| {
            quote! {
                impl std::fmt::Display for #name {
                    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        std::fmt::Display::fmt(self.syntax(), f)
                    }
                }
            }
        });

    let defined_nodes: HashSet<_> = node_names.collect();

    for node in kinds
        .nodes
        .iter()
        .map(|kind| to_pascal_case(kind))
        .filter(|name| !defined_nodes.iter().any(|&it| it == name))
    {
        drop(node)
        // TODO: restore this
        // eprintln!("Warning: node {} not defined in ast source", node);
    }

    let ast = quote! {
        use crate::{
            SyntaxNode, SyntaxToken, SyntaxKind::{self, *},
            ast::{self, AstNode, AstChildren, support},
            T,
        };

        #(#node_defs)*
        #(#enum_defs)*
        #(#node_boilerplate_impls)*
        #(#enum_boilerplate_impls)*
        #(#display_impls)*
    };

    let ast = ast.to_string().replace("T ! [", "T![");

    let mut res = String::with_capacity(ast.len() * 2);

    let mut docs =
        grammar.nodes.iter().map(|it| &it.doc).chain(grammar.enums.iter().map(|it| &it.doc));

    for chunk in ast.split("# [pretty_doc_comment_placeholder_workaround] ") {
        res.push_str(chunk);
        if let Some(doc) = docs.next() {
            write_doc_comment(&doc, &mut res);
        }
    }

    let pretty = reformat(&res)?;
    Ok(pretty)
}

fn write_doc_comment(contents: &[String], dest: &mut String) {
    for line in contents {
        writeln!(dest, "///{}", line).unwrap();
    }
}

fn generate_syntax_kinds(grammar: KindsSrc<'_>) -> Result<String> {
    let (single_byte_tokens_values, single_byte_tokens): (Vec<_>, Vec<_>) = grammar
        .punct
        .iter()
        .filter(|(token, _name)| token.len() == 1)
        .map(|(token, name)| (token.chars().next().unwrap(), format_ident!("{}", name)))
        .unzip();

    let punctuation_values = grammar.punct.iter().map(|(token, _name)| {
        if "{}[]()".contains(token) {
            let c = token.chars().next().unwrap();
            quote! { #c }
        } else {
            let cs = token.chars().map(|c| Punct::new(c, Spacing::Joint));
            quote! { #(#cs)* }
        }
    });
    let punctuation =
        grammar.punct.iter().map(|(_token, name)| format_ident!("{}", name)).collect::<Vec<_>>();

    let full_keywords_values = &grammar.keywords;
    let full_keywords =
        full_keywords_values.iter().map(|kw| format_ident!("{}_KW", to_upper_snake_case(&kw)));

    let all_keywords_values =
        grammar.keywords.iter().chain(grammar.contextual_keywords.iter()).collect::<Vec<_>>();
    let all_keywords_idents = all_keywords_values.iter().map(|kw| format_ident!("{}", kw));
    let all_keywords = all_keywords_values
        .iter()
        .map(|name| format_ident!("{}_KW", to_upper_snake_case(&name)))
        .collect::<Vec<_>>();

    let literals =
        grammar.literals.iter().map(|name| format_ident!("{}", name)).collect::<Vec<_>>();

    let tokens = grammar.tokens.iter().map(|name| format_ident!("{}", name)).collect::<Vec<_>>();

    let nodes = grammar.nodes.iter().map(|name| format_ident!("{}", name)).collect::<Vec<_>>();

    let ast = quote! {
        #![allow(bad_style, missing_docs, unreachable_pub)]
        /// The kind of syntax node, e.g. `IDENT`, `USE_KW`, or `STRUCT`.
        #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
        #[repr(u16)]
        pub enum SyntaxKind {
            // Technical SyntaxKinds: they appear temporally during parsing,
            // but never end up in the final tree
            #[doc(hidden)]
            TOMBSTONE,
            #[doc(hidden)]
            EOF,
            #(#punctuation,)*
            #(#all_keywords,)*
            #(#literals,)*
            #(#tokens,)*
            #(#nodes,)*

            // Technical kind so that we can cast from u16 safely
            #[doc(hidden)]
            __LAST,
        }
        use self::SyntaxKind::*;

        impl SyntaxKind {
            pub fn is_keyword(self) -> bool {
                match self {
                    #(#all_keywords)|* => true,
                    _ => false,
                }
            }

            pub fn is_punct(self) -> bool {
                match self {
                    #(#punctuation)|* => true,
                    _ => false,
                }
            }

            pub fn is_literal(self) -> bool {
                match self {
                    #(#literals)|* => true,
                    _ => false,
                }
            }

            pub fn from_keyword(ident: &str) -> Option<SyntaxKind> {
                let kw = match ident {
                    #(#full_keywords_values => #full_keywords,)*
                    _ => return None,
                };
                Some(kw)
            }

            pub fn from_char(c: char) -> Option<SyntaxKind> {
                let tok = match c {
                    #(#single_byte_tokens_values => #single_byte_tokens,)*
                    _ => return None,
                };
                Some(tok)
            }
        }

        #[macro_export]
        macro_rules! T {
            #([#punctuation_values] => { $crate::SyntaxKind::#punctuation };)*
            #([#all_keywords_idents] => { $crate::SyntaxKind::#all_keywords };)*
            [lifetime] => { $crate::SyntaxKind::LIFETIME };
            [ident] => { $crate::SyntaxKind::IDENT };
            [string] => { $crate::SyntaxKind::STRING };
            [int_number] => { $crate::SyntaxKind::INT_NUMBER };
            [shebang] => { $crate::SyntaxKind::SHEBANG };
        }
    };

    reformat(&ast.to_string())
}

fn to_upper_snake_case(s: &str) -> String {
    let mut buf = String::with_capacity(s.len());
    let mut prev = false;
    for c in s.chars() {
        if c.is_ascii_uppercase() && prev {
            buf.push('_')
        }
        prev = true;

        buf.push(c.to_ascii_uppercase());
    }
    buf
}

fn to_lower_snake_case(s: &str) -> String {
    let mut buf = String::with_capacity(s.len());
    let mut prev = false;
    for c in s.chars() {
        if c.is_ascii_uppercase() && prev {
            buf.push('_')
        }
        prev = true;

        buf.push(c.to_ascii_lowercase());
    }
    buf
}

fn to_pascal_case(s: &str) -> String {
    let mut buf = String::with_capacity(s.len());
    let mut prev_is_underscore = true;
    for c in s.chars() {
        if c == '_' {
            prev_is_underscore = true;
        } else if prev_is_underscore {
            buf.push(c.to_ascii_uppercase());
            prev_is_underscore = false;
        } else {
            buf.push(c.to_ascii_lowercase());
        }
    }
    buf
}

fn pluralize(s: &str) -> String {
    format!("{}s", s)
}

impl Field {
    fn is_many(&self) -> bool {
        matches!(self, Field::Node { cardinality: Cardinality::Many, .. })
    }
    fn token_kind(&self) -> Option<proc_macro2::TokenStream> {
        match self {
            Field::Token(token) => {
                let token: proc_macro2::TokenStream = token.parse().unwrap();
                Some(quote! { T![#token] })
            }
            _ => None,
        }
    }
    fn method_name(&self) -> proc_macro2::Ident {
        match self {
            Field::Token(name) => {
                let name = match name.as_str() {
                    ";" => "semicolon",
                    "->" => "thin_arrow",
                    "'{'" => "l_curly",
                    "'}'" => "r_curly",
                    "'('" => "l_paren",
                    "')'" => "r_paren",
                    "'['" => "l_brack",
                    "']'" => "r_brack",
                    "<" => "l_angle",
                    ">" => "r_angle",
                    "=" => "eq",
                    "!" => "excl",
                    "*" => "star",
                    "&" => "amp",
                    "_" => "underscore",
                    "." => "dot",
                    ".." => "dotdot",
                    "..." => "dotdotdot",
                    "..=" => "dotdoteq",
                    "=>" => "fat_arrow",
                    "@" => "at",
                    ":" => "colon",
                    "::" => "coloncolon",
                    "#" => "pound",
                    "?" => "question_mark",
                    "," => "comma",
                    "|" => "pipe",
                    _ => name,
                };
                format_ident!("{}_token", name)
            }
            Field::Node { name, .. } => {
                if name == "type" {
                    format_ident!("ty")
                } else {
                    format_ident!("{}", name)
                }
            }
        }
    }
    fn ty(&self) -> proc_macro2::Ident {
        match self {
            Field::Token(_) => format_ident!("SyntaxToken"),
            Field::Node { ty, .. } => format_ident!("{}", ty),
        }
    }
}

fn lower(grammar: &Grammar) -> AstSrc {
    let mut res = AstSrc::default();

    res.tokens = "Whitespace Comment String ByteString IntNumber FloatNumber"
        .split_ascii_whitespace()
        .map(|it| it.to_string())
        .collect::<Vec<_>>();

    let nodes = grammar.iter().collect::<Vec<_>>();

    for &node in &nodes {
        let name = grammar[node].name.clone();
        let rule = &grammar[node].rule;
        match lower_enum(grammar, rule) {
            Some(variants) => {
                let enum_src = AstEnumSrc { doc: Vec::new(), name, traits: Vec::new(), variants };
                res.enums.push(enum_src);
            }
            None => {
                let mut fields = Vec::new();
                lower_rule(&mut fields, grammar, None, rule);
                res.nodes.push(AstNodeSrc { doc: Vec::new(), name, traits: Vec::new(), fields });
            }
        }
    }

    deduplicate_fields(&mut res);
    extract_enums(&mut res);
    extract_struct_traits(&mut res);
    extract_enum_traits(&mut res);
    res
}

fn lower_enum(grammar: &Grammar, rule: &Rule) -> Option<Vec<String>> {
    let alternatives = match rule {
        Rule::Alt(it) => it,
        _ => return None,
    };
    let mut variants = Vec::new();
    for alternative in alternatives {
        match alternative {
            Rule::Node(it) => variants.push(grammar[*it].name.clone()),
            Rule::Token(it) if grammar[*it].name == ";" => (),
            _ => return None,
        }
    }
    Some(variants)
}

fn lower_rule(acc: &mut Vec<Field>, grammar: &Grammar, label: Option<&String>, rule: &Rule) {
    if lower_comma_list(acc, grammar, label, rule) {
        return;
    }

    match rule {
        Rule::Node(node) => {
            let ty = grammar[*node].name.clone();
            let name = label.cloned().unwrap_or_else(|| to_lower_snake_case(&ty));
            let field = Field::Node { name, ty, cardinality: Cardinality::Optional };
            acc.push(field);
        }
        Rule::Token(token) => {
            assert!(label.is_none());
            let mut name = grammar[*token].name.clone();
            if name != "int_number" && name != "string" {
                if "[]{}()".contains(&name) {
                    name = format!("'{}'", name);
                }
                let field = Field::Token(name);
                acc.push(field);
            }
        }
        Rule::Rep(inner) => {
            if let Rule::Node(node) = &**inner {
                let ty = grammar[*node].name.clone();
                let name = label.cloned().unwrap_or_else(|| pluralize(&to_lower_snake_case(&ty)));
                let field = Field::Node { name, ty, cardinality: Cardinality::Many };
                acc.push(field);
                return;
            }
            todo!("{:?}", rule)
        }
        Rule::Labeled { label: l, rule } => {
            assert!(label.is_none());
            let manually_implemented = matches!(
                l.as_str(),
                "lhs"
                    | "rhs"
                    | "then_branch"
                    | "else_branch"
                    | "start"
                    | "end"
                    | "op"
                    | "index"
                    | "base"
                    | "value"
                    | "trait"
                    | "self_ty"
            );
            if manually_implemented {
                return;
            }
            lower_rule(acc, grammar, Some(l), rule);
        }
        Rule::Seq(rules) | Rule::Alt(rules) => {
            for rule in rules {
                lower_rule(acc, grammar, label, rule)
            }
        }
        Rule::Opt(rule) => lower_rule(acc, grammar, label, rule),
    }
}

// (T (',' T)* ','?)
fn lower_comma_list(
    acc: &mut Vec<Field>,
    grammar: &Grammar,
    label: Option<&String>,
    rule: &Rule,
) -> bool {
    let rule = match rule {
        Rule::Seq(it) => it,
        _ => return false,
    };
    let (node, repeat, trailing_comma) = match rule.as_slice() {
        [Rule::Node(node), Rule::Rep(repeat), Rule::Opt(trailing_comma)] => {
            (node, repeat, trailing_comma)
        }
        _ => return false,
    };
    let repeat = match &**repeat {
        Rule::Seq(it) => it,
        _ => return false,
    };
    match repeat.as_slice() {
        [comma, Rule::Node(n)] if comma == &**trailing_comma && n == node => (),
        _ => return false,
    }
    let ty = grammar[*node].name.clone();
    let name = label.cloned().unwrap_or_else(|| pluralize(&to_lower_snake_case(&ty)));
    let field = Field::Node { name, ty, cardinality: Cardinality::Many };
    acc.push(field);
    true
}

fn deduplicate_fields(ast: &mut AstSrc) {
    for node in &mut ast.nodes {
        let mut i = 0;
        'outer: while i < node.fields.len() {
            for j in 0..i {
                let f1 = &node.fields[i];
                let f2 = &node.fields[j];
                if f1 == f2 {
                    node.fields.remove(i);
                    continue 'outer;
                }
            }
            i += 1;
        }
    }
}

fn extract_enums(ast: &mut AstSrc) {
    for node in &mut ast.nodes {
        for enm in &ast.enums {
            let mut to_remove = Vec::new();
            for (i, field) in node.fields.iter().enumerate() {
                let ty = field.ty().to_string();
                if enm.variants.iter().any(|it| it == &ty) {
                    to_remove.push(i);
                }
            }
            if to_remove.len() == enm.variants.len() {
                node.remove_field(to_remove);
                let ty = enm.name.clone();
                let name = to_lower_snake_case(&ty);
                node.fields.push(Field::Node { name, ty, cardinality: Cardinality::Optional });
            }
        }
    }
}

fn extract_struct_traits(ast: &mut AstSrc) {
    let traits: &[(&str, &[&str])] = &[
        ("AttrsOwner", &["attrs"]),
        ("NameOwner", &["name"]),
        ("VisibilityOwner", &["visibility"]),
        ("GenericParamsOwner", &["generic_param_list", "where_clause"]),
        ("TypeBoundsOwner", &["type_bound_list", "colon_token"]),
        ("ModuleItemOwner", &["items"]),
        ("LoopBodyOwner", &["label", "loop_body"]),
        ("ArgListOwner", &["arg_list"]),
    ];

    for node in &mut ast.nodes {
        for (name, methods) in traits {
            extract_struct_trait(node, name, methods);
        }
    }
}

fn extract_struct_trait(node: &mut AstNodeSrc, trait_name: &str, methods: &[&str]) {
    let mut to_remove = Vec::new();
    for (i, field) in node.fields.iter().enumerate() {
        let method_name = field.method_name().to_string();
        if methods.iter().any(|&it| it == &method_name) {
            to_remove.push(i);
        }
    }
    if to_remove.len() == methods.len() {
        node.traits.push(trait_name.to_string());
        node.remove_field(to_remove);
    }
}

fn extract_enum_traits(ast: &mut AstSrc) {
    for enm in &mut ast.enums {
        if enm.name == "Stmt" {
            continue;
        }
        let nodes = &ast.nodes;
        let mut variant_traits = enm
            .variants
            .iter()
            .map(|var| nodes.iter().find(|it| &it.name == var).unwrap())
            .map(|node| node.traits.iter().cloned().collect::<BTreeSet<_>>());

        let mut enum_traits = match variant_traits.next() {
            Some(it) => it,
            None => continue,
        };
        for traits in variant_traits {
            enum_traits = enum_traits.intersection(&traits).cloned().collect();
        }
        enm.traits = enum_traits.into_iter().collect();
    }
}

impl AstNodeSrc {
    fn remove_field(&mut self, to_remove: Vec<usize>) {
        to_remove.into_iter().rev().for_each(|idx| {
            self.fields.remove(idx);
        });
    }
}

// -- Make generation --

fn generate_make(grammar: &Grammar) -> Result<String> {
    let nodes = grammar.iter().collect::<Vec<_>>();

    let mut funcs = vec![];

    // use nodelabel names for param names if possible?
    nodes
        .into_iter()
        .filter(|node| {
            // FIXME: Module and Rename for example emit to functions which ideally could be collapsed into one impl

            // things that have to be manual implementations as these are not supported/too complex
            // to autogenerate
            !matches!(
                grammar[*node].name.as_ref(),
                "Literal" // <-- TODO
                    | "MacroCall" // opt punct
                    | "UseTree" // nested opt
                    | "Fn" // opt lit
                    | "SelfParam" // opt lit, nested opt
                    | "Const" // opt lit
                    | "Static" // opt lit
                    | "Trait" // nested opt
                    | "TypeAlias" // nested opt
                    | "LifetimeParam" // nested opt
                    | "Impl" // opt everything
                    | "Attr" // opt punct
                    | "ExprStmt" // opt semi
                    | "MatchArm" // opt comma
                    | "RefType" // 'mut'?
                    | "FnPtrType" // opt lit
                    | "TypeBound" // opt punctuation
                    | "IdentPat" // opt lit
                    | "RefPat" // opt lit
                    | "RecordPatFieldList" // opt punct
                    | "ClosureExpr" // opt lit
                    | "Visibility" // has opt literals after stripped alternating paths
                    | "TokenTree" // the rule doesnt respect the inner tokens
            )
        })
        .for_each(|node| {
            let name = grammar[node].name.clone();
            let rule = &grammar[node].rule;
            if lower_enum(grammar, rule).is_some() {
                return;
            }
            let ctors = rule_to_ctors(grammar, &name, rule);
            funcs.push(quote! {
                #( #ctors )*
            });
        });

    let res = quote! {
        #![allow(unused_mut)]
        use crate::{ast, AstNode, SyntaxKind, SyntaxNode, T};
        use itertools::Itertools;
        use rowan::{GreenNode, GreenToken, NodeOrToken, SmolStr, SyntaxKind as RSyntaxKind};


        #(#funcs)*
    };

    let pretty = reformat(&res.to_string()).unwrap_or(res.to_string());
    Ok(pretty)
}

#[derive(Copy, Clone, PartialEq, Debug)]
enum ParamKind {
    Required,
    Optional,
    Many,
}

enum Node {
    Str(String),
    Typed(String, String),
}

impl Node {
    #[allow(dead_code)]
    fn name(&mut self) -> &str {
        match self {
            Node::Typed(name, _) | Node::Str(name) => name,
        }
    }

    fn name_mut(&mut self) -> &mut String {
        match self {
            Node::Typed(name, _) | Node::Str(name) => name,
        }
    }

    // input param type
    fn name_and_ty(&self) -> (&str, &str) {
        match self {
            Node::Str(name) => (name, "&'a str"),
            Node::Typed(name, ty) => (name, ty),
        }
    }
}
enum NodeOrToken {
    Node(Node),
    Token(String),
}

impl NodeOrToken {
    fn name_mut(&mut self) -> Option<&mut String> {
        match self {
            NodeOrToken::Node(node) => Some(node.name_mut()),
            NodeOrToken::Token(_) => None,
        }
    }
}

enum MakeFnAction {
    Always(NodeOrToken),
    Optional(Vec<String>, Node, Vec<String>),
    Repeat(Node, Option<String>),
}

struct MakeFnRecorder {
    actions: Vec<MakeFnAction>,
}

impl MakeFnRecorder {
    fn new() -> Self {
        MakeFnRecorder { actions: vec![] }
    }

    fn push_rep_node(&mut self, name: String, sep: Option<String>) {
        let ty = format!("ast::{}", &name);
        let name = to_lower_snake_case(&name);
        self.actions.push(MakeFnAction::Repeat(Node::Typed(name, ty), sep));
    }
    fn push_opt_node(&mut self, name: String) {
        let ty = format!("ast::{}", &name);
        let name = to_lower_snake_case(&name);
        self.actions.push(MakeFnAction::Optional(vec![], Node::Typed(name, ty), vec![]))
    }
    fn push_opt_node_with_tokens(&mut self, left: Vec<String>, name: String, right: Vec<String>) {
        let ty = format!("ast::{}", &name);
        let name = to_lower_snake_case(&name);
        self.actions.push(MakeFnAction::Optional(left, Node::Typed(name, ty), right))
    }

    fn push_token(&mut self, token: String) {
        self.actions.push(MakeFnAction::Always(NodeOrToken::Token(token)));
    }

    fn push_node(&mut self, name: String) {
        let ty = format!("ast::{}", &name);
        let name = to_lower_snake_case(&name);
        self.actions.push(MakeFnAction::Always(NodeOrToken::Node(Node::Typed(name, ty))));
    }

    fn push_string_node(&mut self, name: String) {
        self.actions.push(MakeFnAction::Always(NodeOrToken::Node(Node::Str(name))));
    }

    fn push_opt_string_node_with_tokens(
        &mut self,
        left: Vec<String>,
        name: String,
        right: Vec<String>,
    ) {
        self.actions.push(MakeFnAction::Optional(left, Node::Str(name), right));
    }

    fn params(&self) -> impl Iterator<Item = Param> + '_ {
        self.actions.iter().flat_map(|action| match action {
            MakeFnAction::Always(NodeOrToken::Node(node)) => {
                let (name, ty) = node.name_and_ty();
                Some(Param { name: name.to_owned(), ty: ty.to_owned(), kind: ParamKind::Required })
            }
            MakeFnAction::Optional(_, node, _) => {
                let (name, ty) = node.name_and_ty();
                Some(Param { name: name.to_owned(), ty: ty.to_owned(), kind: ParamKind::Optional })
            }
            MakeFnAction::Repeat(node, _) => {
                let (name, ty) = node.name_and_ty();
                Some(Param { name: name.to_owned(), ty: ty.to_owned(), kind: ParamKind::Many })
            }
            _ => None,
        })
    }

    fn param_names(&mut self) -> impl Iterator<Item = &'_ mut String> + '_ {
        self.actions.iter_mut().flat_map(|action| match action {
            MakeFnAction::Always(nt) => nt.name_mut(),
            MakeFnAction::Optional(_, node, _) | MakeFnAction::Repeat(node, _) => {
                Some(node.name_mut())
            }
        })
    }

    fn gen_body(&self) -> proc_macro2::TokenStream {
        self.actions
            .iter()
            .map(|action| match action {
                MakeFnAction::Always(NodeOrToken::Node(Node::Typed(name, _))) => {
                    let name = name_to_ident(name);
                    quote! {
                        children.push(NodeOrToken::Node(#name.syntax().green().clone()));
                    }
                }
                MakeFnAction::Always(NodeOrToken::Node(Node::Str(name))) => {
                    let kind = str_node_to_kind(name);
                    let name = name_to_ident(name);
                    quote! { children.push(NodeOrToken::Token(GreenToken::new(RSyntaxKind(T![#kind] as u16), SmolStr::from(#name)))); }
                },
                MakeFnAction::Always(NodeOrToken::Token(token)) => quote_push_token(token),
                MakeFnAction::Optional(left, Node::Typed(name, _), right) => {
                    let left = left.iter().map(|s| quote_push_token(s));
                    let right = right.iter().map(|s| quote_push_token(s));
                    let name = name_to_ident(name);
                    quote! {
                        if let Some(#name) = #name.into() {
                            #(#left)*
                            children.push(NodeOrToken::Node(#name.syntax().green().clone()));
                            #(#right)*
                        }
                    }
                }
                MakeFnAction::Optional(left, Node::Str(name), right) => {
                    let kind = str_node_to_kind(name);
                    let left = left.iter().map(|s| quote_push_token(s));
                    let right = right.iter().map(|s| quote_push_token(s));
                    let name = name_to_ident(name);
                    quote! {
                        if let Some(#name) = #name.into() {
                            #(#left)*
                            children.push(NodeOrToken::Token(GreenToken::new(RSyntaxKind(T![#kind] as u16), SmolStr::from(#name))));
                            #(#right)*
                        }
                    }
                }
                MakeFnAction::Repeat(Node::Typed(name, _), sep) => {
                    let sep = sep.as_deref().map(quote_token).into_iter();
                    let name = name_to_ident(name);
                    quote! {
                        children.extend(#name.into_iter().map(|item| NodeOrToken::Node(item.syntax().green().clone())) #(.intersperse(#sep))* );
                    }
                }
                MakeFnAction::Repeat(Node::Str(_), ..) => {panic!()}
            })
            .flatten()
            .collect()
    }
}

fn str_node_to_kind(name: &str) -> proc_macro2::Ident {
    match name {
        "lifetime" | "string" | "shebang" | "int_number" | "ident" => format_ident!("{}", name),
        name => panic!("unexpected string node {:?}", name),
    }
}

fn quote_push_token(token: &str) -> proc_macro2::TokenStream {
    let token = quote_token(token);
    quote! { children.push(#token); }
}

fn quote_token(token: &str) -> proc_macro2::TokenStream {
    let macro_token = match token {
        "(" => "'('",
        ")" => "')'",
        "{" => "'{'",
        "}" => "'}'",
        "[" => "'['",
        "]" => "']'",
        token => token,
    };
    let macro_token: proc_macro2::TokenStream = macro_token.parse().unwrap();
    quote! { NodeOrToken::Token(GreenToken::new(RSyntaxKind(T![#macro_token] as u16), SmolStr::from(#token))) }
}

#[derive(Clone, Debug)]
struct Param {
    name: String,
    ty: String,
    kind: ParamKind,
}

impl quote::ToTokens for Param {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let name = name_to_ident(&self.name);
        let ty: proc_macro2::TokenStream = self.ty.parse().unwrap();
        tokens.extend(match self.kind {
            ParamKind::Required => quote!(#name : #ty),
            ParamKind::Optional => quote!(#name : impl Into<Option<#ty>>),
            ParamKind::Many => quote!(#name : impl IntoIterator<Item = #ty>),
        });
    }
}
struct MakeConstructor {
    name: String,
    rule_name: String,
    params: Vec<Param>,
    body: proc_macro2::TokenStream,
}

impl quote::ToTokens for MakeConstructor {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let name = name_to_ident(&self.name);
        let ty = format_ident!("{}", &self.rule_name);
        let syntax_kind = format_ident!("{}", to_upper_snake_case(&self.rule_name));
        let params = self.params.iter();
        let lifetime = self.params.iter().find_map(|param| {
            if param.ty == "&'a str" {
                Some(quote!(<'a>))
            } else {
                None
            }
        });
        let body = &self.body;
        tokens.extend(quote! {
            pub fn #name #lifetime (#(#params),*) -> ast::#ty {
                let mut children = vec![];
                #body
                let green_node = GreenNode::new(RSyntaxKind(SyntaxKind::#syntax_kind as u16), children);
                ast::#ty::cast(SyntaxNode::new_root(green_node)).unwrap()
            }
        });
    }
}

fn lower_rule_to_ctor(recorder: &mut MakeFnRecorder, rule: NonAltRule) {
    match rule {
        NonAltRule::Token(token) => {
            if let Some(name) = token_as_param(&token) {
                recorder.push_string_node(name);
            } else {
                recorder.push_token(token);
            };
        }
        // node alias
        NonAltRule::Node(name) => recorder.push_node(name),
        NonAltRule::Opt(_) => unimplemented!("direct optional rules"),
        NonAltRule::Rep(r) => lower_rep_rule(recorder, *r),
        NonAltRule::Seq(r) => lower_seq_rule(recorder, r),
    }
}

fn lower_seq_rule(recorder: &mut MakeFnRecorder, sequence: Vec<NonAltRule>) {
    if try_lower_seperated_rep(recorder, &sequence) {
        // TypeBoundList/ OrPat
        // FIXME: The way this works atm allows these to be constructed completely empty
        return;
    }
    for rule in sequence {
        match rule {
            NonAltRule::Token(token) => recorder.push_token(token),
            NonAltRule::Node(name) => recorder.push_node(name),
            NonAltRule::Opt(r) => lower_opt_rule(recorder, *r),
            NonAltRule::Rep(r) => lower_rep_rule(recorder, *r),
            NonAltRule::Seq(sequence) => lower_seq_rule(recorder, sequence),
        }
    }
}

fn lower_rep_rule(recorder: &mut MakeFnRecorder, rule: NonAltRule) {
    match rule {
        NonAltRule::Node(name) => recorder.push_rep_node(name, None),
        NonAltRule::Token(it) => panic!("{:?}", it),
        NonAltRule::Opt(it) => panic!("{:?}", it),
        NonAltRule::Rep(it) => panic!("{:?}", it),
        NonAltRule::Seq(it) => panic!("{:?}", it),
    }
}

fn lower_opt_rule(recorder: &mut MakeFnRecorder, rule: NonAltRule) {
    match rule {
        NonAltRule::Token(token) => {
            if let Some(name) = token_as_param(&token) {
                // Abi/SelfParam
                recorder.push_opt_string_node_with_tokens(vec![], name, vec![]);
            } else {
                if token == "::" {
                    // FIXME prefix ::, triggered by GenericArgList and one case of PathSegment
                } else if ["super", "self", "crate"].contains(&&*token) {
                    // Visibillity
                } else {
                    // other optional tokens
                    panic!();
                }
            }
        }
        NonAltRule::Node(name) => recorder.push_opt_node(name),
        NonAltRule::Opt(it) => panic!("{:?}", it),
        NonAltRule::Rep(it) => panic!("{:?}", it),
        NonAltRule::Seq(it) => {
            if !try_lower_seperated_rep(recorder, &it) {
                // not of the form (Node ( 'sep' Node) 'sep'?)?
                // so expect the form (sep1 Node sep2)? where sep1 and sep2 are arbitrarily many non optional tokens
                if let Some((node_pos, node)) = it
                    .iter()
                    .enumerate()
                    .find(|(_, rule)| matches!(rule, NonAltRule::Node(_) | NonAltRule::Opt(_)))
                {
                    let name = match node {
                        NonAltRule::Node(name) => Cow::Borrowed(name),
                        NonAltRule::Opt(inner) => match inner.as_ref() {
                            NonAltRule::Node(name) => Cow::Borrowed(name),
                            // NonAltRule::Token(token) => Cow::Owned(token_as_param(token).unwrap()),
                            _ => panic!("expected Node: {:?}", node),
                        },
                        _ => panic!("expected (optional) Node: {:?}", node),
                    };
                    let (left, right) = it.split_at(node_pos);

                    let get_token_strings = |tok| match tok {
                        &NonAltRule::Token(ref token) => token.clone(),
                        _ => panic!("expected token: {:?}", tok),
                    };
                    match name {
                        Cow::Borrowed(name) => recorder.push_opt_node_with_tokens(
                            left.iter().map(get_token_strings).collect(),
                            name.to_owned(),
                            right[1..].iter().map(get_token_strings).collect(),
                        ),
                        Cow::Owned(name) => recorder.push_opt_string_node_with_tokens(
                            left.iter().map(get_token_strings).collect(),
                            name,
                            right[1..].iter().map(get_token_strings).collect(),
                        ),
                    }
                }
            }
        }
    }
}

// box patterns would be nice
/// This tries to parse the pattern (Node ('sep' Node)* sep?) which appears repeatedly in the grammar
fn try_lower_seperated_rep(recorder: &mut MakeFnRecorder, rule: &[NonAltRule]) -> bool {
    let (first_node, rep, sep) = match rule {
        [NonAltRule::Node(first_node), NonAltRule::Rep(rep), NonAltRule::Opt(sep)] => {
            (first_node, rep, sep)
        }
        _ => return false,
    };
    let seperator = match sep.as_ref() {
        NonAltRule::Token(token) => token,
        sep => panic!("sep token expected: {:?}", sep),
    };

    if let NonAltRule::Seq(seq) = rep.as_ref() {
        match seq.as_slice() {
            [NonAltRule::Token(token), NonAltRule::Node(node)] => {
                assert_eq!(seperator, token, "sep and repeat token are unequal");
                assert_eq!(first_node, node, "start and repeat node are unequal");
                recorder.push_rep_node(first_node.clone(), Some(seperator.clone()));
                return true;
            }
            seq => panic!("token followed by node expected: {:?}", seq),
        }
    }
    panic!("sequence expected: {:?}", rep.as_ref());
}

fn rename_dupe_params<'a>(param_names: impl Iterator<Item = &'a mut String>) {
    let mut c = 0;
    let mut last_param_name = String::new();
    for param in param_names {
        if &last_param_name == param {
            c += 1;
            *param = format!("{}{}", param, c);
        } else {
            c = 0;
            last_param_name = param.clone();
        }
    }
}

fn rule_to_ctors(grammar: &Grammar, node_name: &str, rule: &Rule) -> Vec<MakeConstructor> {
    // dbg!(node_name); // uncomment this if something panics to see what rule it is
    let mut rules = strip_alt(grammar, vec![], rule, None);
    if node_name == "PathSegment" {
        rules.remove(3); // '::'? NameRef
    }

    let mut constructors: Vec<_> = rules
        .into_iter()
        .map(|rule| {
            let mut recorder = MakeFnRecorder::new();
            lower_rule_to_ctor(&mut recorder, rule);

            rename_dupe_params(recorder.param_names());

            MakeConstructor {
                name: node_name.to_owned(),
                rule_name: node_name.to_owned(),
                params: recorder.params().collect(),
                body: recorder.gen_body(),
            }
        })
        .collect();

    if constructors.len() > 1 {
        constructors
            .iter_mut()
            .enumerate()
            .for_each(|(idx, ctor)| ctor.name = format!("{}{}", &ctor.name, idx))
    }
    constructors
}

/// Basically like ungrammar::Rule but without alternations
#[derive(Clone, Debug)]
enum NonAltRule {
    Token(String),
    Node(String),
    Opt(Box<NonAltRule>),
    Rep(Box<NonAltRule>),
    Seq(Vec<NonAltRule>),
}

// Removes alternating rules from a rule, therefor creating a new rule for each alternating path
// simplifies generating make constructors
fn strip_alt(
    grammar: &Grammar,
    mut collect: Vec<NonAltRule>,
    rule: &Rule,
    renamed: Option<&str>,
) -> Vec<NonAltRule> {
    match rule {
        Rule::Labeled { label, rule } => return strip_alt(grammar, collect, rule, Some(label)),
        &Rule::Node(node) => collect.push(NonAltRule::Node(grammar[node].name.clone())),
        &Rule::Token(token) => collect.push(NonAltRule::Token(grammar[token].name.clone())),
        Rule::Seq(rules) => {
            let mut sequences = vec![collect];
            for rule in rules {
                sequences = sequences
                    .into_iter()
                    .flat_map(|sequence| {
                        strip_alt(grammar, vec![], rule, renamed).into_iter().map(move |next| {
                            let mut sequence = sequence.clone();
                            sequence.push(next);
                            sequence
                        })
                    })
                    .collect();
            }
            collect = sequences.into_iter().map(NonAltRule::Seq).collect();
        }
        Rule::Alt(rules) => {
            collect = rules
                .iter()
                .map(|rule| strip_alt(grammar, collect.clone(), rule, renamed))
                .flat_map(IntoIterator::into_iter)
                .collect();
        }
        Rule::Opt(rule) => {
            // if let Rule::Token(token) = **rule { // check for ascii only
            //     FIXME, split this as if it was alternating?
            // } else {
            collect = strip_alt(grammar, collect, rule, renamed)
                .into_iter()
                .map(|tnr| NonAltRule::Opt(Box::new(tnr)))
                .collect()
            // }
        }
        Rule::Rep(rule) => {
            collect = strip_alt(grammar, collect, rule, renamed)
                .into_iter()
                .map(|tnr| NonAltRule::Rep(Box::new(tnr)))
                .collect()
        }
    }
    collect
}

fn token_as_param(token: &str) -> Option<String> {
    match token {
        "ident" | "int_number" | "lifetime" | "string" | "shebang" => Some(token.to_owned()),
        _ => None,
    }
}

fn name_to_ident(name: &str) -> proc_macro2::Ident {
    match to_lower_snake_case(name).as_ref() {
        "type" => format_ident!("ty"),
        "const" => format_ident!("konst"),
        "enum" => format_ident!("enum_"),
        "impl" => format_ident!("impl_"),
        "fn" => format_ident!("func"),
        "static" => format_ident!("statik"),
        "struct" => format_ident!("strukt"),
        "trait" => format_ident!("trait_"),
        "use" => format_ident!("use_"),
        name => format_ident!("{}", name),
    }
}
