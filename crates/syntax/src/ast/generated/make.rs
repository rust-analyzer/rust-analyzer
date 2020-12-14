//! Generated file, do not edit by hand, see `xtask/src/codegen`

#![allow(unused_mut)]
use crate::{ast, AstNode, SyntaxKind, SyntaxNode, T};
use itertools::Itertools;
use rowan::{GreenNode, GreenToken, NodeOrToken, SmolStr, SyntaxKind as RSyntaxKind};
pub fn name<'a>(ident: &'a str) -> ast::Name {
    let mut children = vec![];
    children.push(NodeOrToken::Token(GreenToken::new(
        RSyntaxKind(T![ident] as u16),
        SmolStr::from(ident),
    )));
    let green_node = GreenNode::new(RSyntaxKind(SyntaxKind::NAME as u16), children);
    ast::Name::cast(SyntaxNode::new_root(green_node)).unwrap()
}
pub fn name_ref0<'a>(ident: &'a str) -> ast::NameRef {
    let mut children = vec![];
    children.push(NodeOrToken::Token(GreenToken::new(
        RSyntaxKind(T![ident] as u16),
        SmolStr::from(ident),
    )));
    let green_node = GreenNode::new(RSyntaxKind(SyntaxKind::NAME_REF as u16), children);
    ast::NameRef::cast(SyntaxNode::new_root(green_node)).unwrap()
}
pub fn name_ref1<'a>(int_number: &'a str) -> ast::NameRef {
    let mut children = vec![];
    children.push(NodeOrToken::Token(GreenToken::new(
        RSyntaxKind(T![int_number] as u16),
        SmolStr::from(int_number),
    )));
    let green_node = GreenNode::new(RSyntaxKind(SyntaxKind::NAME_REF as u16), children);
    ast::NameRef::cast(SyntaxNode::new_root(green_node)).unwrap()
}
pub fn path(path: impl Into<Option<ast::Path>>, path_segment: ast::PathSegment) -> ast::Path {
    let mut children = vec![];
    if let Some(path) = path.into() {
        children.push(NodeOrToken::Node(path.syntax().green().clone()));
        children.push(NodeOrToken::Token(GreenToken::new(
            RSyntaxKind(T ! [::] as u16),
            SmolStr::from("::"),
        )));
    }
    children.push(NodeOrToken::Node(path_segment.syntax().green().clone()));
    let green_node = GreenNode::new(RSyntaxKind(SyntaxKind::PATH as u16), children);
    ast::Path::cast(SyntaxNode::new_root(green_node)).unwrap()
}
pub fn path_segment0() -> ast::PathSegment {
    let mut children = vec![];
    children.push(NodeOrToken::Token(GreenToken::new(
        RSyntaxKind(T![crate] as u16),
        SmolStr::from("crate"),
    )));
    let green_node = GreenNode::new(RSyntaxKind(SyntaxKind::PATH_SEGMENT as u16), children);
    ast::PathSegment::cast(SyntaxNode::new_root(green_node)).unwrap()
}
pub fn path_segment1() -> ast::PathSegment {
    let mut children = vec![];
    children.push(NodeOrToken::Token(GreenToken::new(
        RSyntaxKind(T![self] as u16),
        SmolStr::from("self"),
    )));
    let green_node = GreenNode::new(RSyntaxKind(SyntaxKind::PATH_SEGMENT as u16), children);
    ast::PathSegment::cast(SyntaxNode::new_root(green_node)).unwrap()
}
pub fn path_segment2() -> ast::PathSegment {
    let mut children = vec![];
    children.push(NodeOrToken::Token(GreenToken::new(
        RSyntaxKind(T![super] as u16),
        SmolStr::from("super"),
    )));
    let green_node = GreenNode::new(RSyntaxKind(SyntaxKind::PATH_SEGMENT as u16), children);
    ast::PathSegment::cast(SyntaxNode::new_root(green_node)).unwrap()
}
pub fn path_segment3(
    name_ref: ast::NameRef,
    generic_arg_list: impl Into<Option<ast::GenericArgList>>,
) -> ast::PathSegment {
    let mut children = vec![];
    children.push(NodeOrToken::Node(name_ref.syntax().green().clone()));
    if let Some(generic_arg_list) = generic_arg_list.into() {
        children.push(NodeOrToken::Node(generic_arg_list.syntax().green().clone()));
    }
    let green_node = GreenNode::new(RSyntaxKind(SyntaxKind::PATH_SEGMENT as u16), children);
    ast::PathSegment::cast(SyntaxNode::new_root(green_node)).unwrap()
}
pub fn path_segment4(
    name_ref: ast::NameRef,
    param_list: ast::ParamList,
    ret_type: impl Into<Option<ast::RetType>>,
) -> ast::PathSegment {
    let mut children = vec![];
    children.push(NodeOrToken::Node(name_ref.syntax().green().clone()));
    children.push(NodeOrToken::Node(param_list.syntax().green().clone()));
    if let Some(ret_type) = ret_type.into() {
        children.push(NodeOrToken::Node(ret_type.syntax().green().clone()));
    }
    let green_node = GreenNode::new(RSyntaxKind(SyntaxKind::PATH_SEGMENT as u16), children);
    ast::PathSegment::cast(SyntaxNode::new_root(green_node)).unwrap()
}
pub fn path_segment5(
    path_type: ast::PathType,
    path_type1: impl Into<Option<ast::PathType>>,
) -> ast::PathSegment {
    let mut children = vec![];
    children
        .push(NodeOrToken::Token(GreenToken::new(RSyntaxKind(T ! [<] as u16), SmolStr::from("<"))));
    children.push(NodeOrToken::Node(path_type.syntax().green().clone()));
    if let Some(path_type1) = path_type1.into() {
        children.push(NodeOrToken::Token(GreenToken::new(
            RSyntaxKind(T![as] as u16),
            SmolStr::from("as"),
        )));
        children.push(NodeOrToken::Node(path_type1.syntax().green().clone()));
    }
    children
        .push(NodeOrToken::Token(GreenToken::new(RSyntaxKind(T ! [>] as u16), SmolStr::from(">"))));
    let green_node = GreenNode::new(RSyntaxKind(SyntaxKind::PATH_SEGMENT as u16), children);
    ast::PathSegment::cast(SyntaxNode::new_root(green_node)).unwrap()
}
pub fn generic_arg_list(
    generic_arg: impl IntoIterator<Item = ast::GenericArg>,
) -> ast::GenericArgList {
    let mut children = vec![];
    children
        .push(NodeOrToken::Token(GreenToken::new(RSyntaxKind(T ! [<] as u16), SmolStr::from("<"))));
    children.extend(
        generic_arg
            .into_iter()
            .map(|item| NodeOrToken::Node(item.syntax().green().clone()))
            .intersperse(NodeOrToken::Token(GreenToken::new(
                RSyntaxKind(T ! [,] as u16),
                SmolStr::from(","),
            ))),
    );
    children
        .push(NodeOrToken::Token(GreenToken::new(RSyntaxKind(T ! [>] as u16), SmolStr::from(">"))));
    let green_node = GreenNode::new(RSyntaxKind(SyntaxKind::GENERIC_ARG_LIST as u16), children);
    ast::GenericArgList::cast(SyntaxNode::new_root(green_node)).unwrap()
}
pub fn param_list0(self_param: ast::SelfParam) -> ast::ParamList {
    let mut children = vec![];
    children
        .push(NodeOrToken::Token(GreenToken::new(RSyntaxKind(T!['('] as u16), SmolStr::from("("))));
    children.push(NodeOrToken::Node(self_param.syntax().green().clone()));
    children
        .push(NodeOrToken::Token(GreenToken::new(RSyntaxKind(T![')'] as u16), SmolStr::from(")"))));
    let green_node = GreenNode::new(RSyntaxKind(SyntaxKind::PARAM_LIST as u16), children);
    ast::ParamList::cast(SyntaxNode::new_root(green_node)).unwrap()
}
pub fn param_list1(
    self_param: impl Into<Option<ast::SelfParam>>,
    param: impl IntoIterator<Item = ast::Param>,
) -> ast::ParamList {
    let mut children = vec![];
    children
        .push(NodeOrToken::Token(GreenToken::new(RSyntaxKind(T!['('] as u16), SmolStr::from("("))));
    if let Some(self_param) = self_param.into() {
        children.push(NodeOrToken::Node(self_param.syntax().green().clone()));
        children.push(NodeOrToken::Token(GreenToken::new(
            RSyntaxKind(T ! [,] as u16),
            SmolStr::from(","),
        )));
    }
    children.extend(
        param.into_iter().map(|item| NodeOrToken::Node(item.syntax().green().clone())).intersperse(
            NodeOrToken::Token(GreenToken::new(RSyntaxKind(T ! [,] as u16), SmolStr::from(","))),
        ),
    );
    children
        .push(NodeOrToken::Token(GreenToken::new(RSyntaxKind(T![')'] as u16), SmolStr::from(")"))));
    let green_node = GreenNode::new(RSyntaxKind(SyntaxKind::PARAM_LIST as u16), children);
    ast::ParamList::cast(SyntaxNode::new_root(green_node)).unwrap()
}
pub fn param_list2(param: impl IntoIterator<Item = ast::Param>) -> ast::ParamList {
    let mut children = vec![];
    children
        .push(NodeOrToken::Token(GreenToken::new(RSyntaxKind(T ! [|] as u16), SmolStr::from("|"))));
    children.extend(
        param.into_iter().map(|item| NodeOrToken::Node(item.syntax().green().clone())).intersperse(
            NodeOrToken::Token(GreenToken::new(RSyntaxKind(T ! [,] as u16), SmolStr::from(","))),
        ),
    );
    children
        .push(NodeOrToken::Token(GreenToken::new(RSyntaxKind(T ! [|] as u16), SmolStr::from("|"))));
    let green_node = GreenNode::new(RSyntaxKind(SyntaxKind::PARAM_LIST as u16), children);
    ast::ParamList::cast(SyntaxNode::new_root(green_node)).unwrap()
}
pub fn ret_type(ty: ast::Type) -> ast::RetType {
    let mut children = vec![];
    children.push(NodeOrToken::Token(GreenToken::new(
        RSyntaxKind(T ! [->] as u16),
        SmolStr::from("->"),
    )));
    children.push(NodeOrToken::Node(ty.syntax().green().clone()));
    let green_node = GreenNode::new(RSyntaxKind(SyntaxKind::RET_TYPE as u16), children);
    ast::RetType::cast(SyntaxNode::new_root(green_node)).unwrap()
}
pub fn path_type(path: ast::Path) -> ast::PathType {
    let mut children = vec![];
    children.push(NodeOrToken::Node(path.syntax().green().clone()));
    let green_node = GreenNode::new(RSyntaxKind(SyntaxKind::PATH_TYPE as u16), children);
    ast::PathType::cast(SyntaxNode::new_root(green_node)).unwrap()
}
pub fn type_arg(ty: ast::Type) -> ast::TypeArg {
    let mut children = vec![];
    children.push(NodeOrToken::Node(ty.syntax().green().clone()));
    let green_node = GreenNode::new(RSyntaxKind(SyntaxKind::TYPE_ARG as u16), children);
    ast::TypeArg::cast(SyntaxNode::new_root(green_node)).unwrap()
}
pub fn assoc_type_arg0(
    name_ref: ast::NameRef,
    type_bound_list: ast::TypeBoundList,
) -> ast::AssocTypeArg {
    let mut children = vec![];
    children.push(NodeOrToken::Node(name_ref.syntax().green().clone()));
    children
        .push(NodeOrToken::Token(GreenToken::new(RSyntaxKind(T ! [:] as u16), SmolStr::from(":"))));
    children.push(NodeOrToken::Node(type_bound_list.syntax().green().clone()));
    let green_node = GreenNode::new(RSyntaxKind(SyntaxKind::ASSOC_TYPE_ARG as u16), children);
    ast::AssocTypeArg::cast(SyntaxNode::new_root(green_node)).unwrap()
}
pub fn assoc_type_arg1(name_ref: ast::NameRef, ty: ast::Type) -> ast::AssocTypeArg {
    let mut children = vec![];
    children.push(NodeOrToken::Node(name_ref.syntax().green().clone()));
    children
        .push(NodeOrToken::Token(GreenToken::new(RSyntaxKind(T ! [=] as u16), SmolStr::from("="))));
    children.push(NodeOrToken::Node(ty.syntax().green().clone()));
    let green_node = GreenNode::new(RSyntaxKind(SyntaxKind::ASSOC_TYPE_ARG as u16), children);
    ast::AssocTypeArg::cast(SyntaxNode::new_root(green_node)).unwrap()
}
pub fn lifetime_arg<'a>(lifetime: &'a str) -> ast::LifetimeArg {
    let mut children = vec![];
    children.push(NodeOrToken::Token(GreenToken::new(
        RSyntaxKind(T![lifetime] as u16),
        SmolStr::from(lifetime),
    )));
    let green_node = GreenNode::new(RSyntaxKind(SyntaxKind::LIFETIME_ARG as u16), children);
    ast::LifetimeArg::cast(SyntaxNode::new_root(green_node)).unwrap()
}
pub fn const_arg(expr: ast::Expr) -> ast::ConstArg {
    let mut children = vec![];
    children.push(NodeOrToken::Node(expr.syntax().green().clone()));
    let green_node = GreenNode::new(RSyntaxKind(SyntaxKind::CONST_ARG as u16), children);
    ast::ConstArg::cast(SyntaxNode::new_root(green_node)).unwrap()
}
pub fn type_bound_list(type_bound: impl IntoIterator<Item = ast::TypeBound>) -> ast::TypeBoundList {
    let mut children = vec![];
    children.extend(
        type_bound
            .into_iter()
            .map(|item| NodeOrToken::Node(item.syntax().green().clone()))
            .intersperse(NodeOrToken::Token(GreenToken::new(
                RSyntaxKind(T ! [+] as u16),
                SmolStr::from("+"),
            ))),
    );
    let green_node = GreenNode::new(RSyntaxKind(SyntaxKind::TYPE_BOUND_LIST as u16), children);
    ast::TypeBoundList::cast(SyntaxNode::new_root(green_node)).unwrap()
}
pub fn macro_items(item: impl IntoIterator<Item = ast::Item>) -> ast::MacroItems {
    let mut children = vec![];
    children.extend(item.into_iter().map(|item| NodeOrToken::Node(item.syntax().green().clone())));
    let green_node = GreenNode::new(RSyntaxKind(SyntaxKind::MACRO_ITEMS as u16), children);
    ast::MacroItems::cast(SyntaxNode::new_root(green_node)).unwrap()
}
pub fn macro_stmts(
    stmt: impl IntoIterator<Item = ast::Stmt>,
    expr: impl Into<Option<ast::Expr>>,
) -> ast::MacroStmts {
    let mut children = vec![];
    children.extend(stmt.into_iter().map(|item| NodeOrToken::Node(item.syntax().green().clone())));
    if let Some(expr) = expr.into() {
        children.push(NodeOrToken::Node(expr.syntax().green().clone()));
    }
    let green_node = GreenNode::new(RSyntaxKind(SyntaxKind::MACRO_STMTS as u16), children);
    ast::MacroStmts::cast(SyntaxNode::new_root(green_node)).unwrap()
}
pub fn source_file<'a>(
    shebang: impl Into<Option<&'a str>>,
    attr: impl IntoIterator<Item = ast::Attr>,
    item: impl IntoIterator<Item = ast::Item>,
) -> ast::SourceFile {
    let mut children = vec![];
    if let Some(shebang) = shebang.into() {
        children.push(NodeOrToken::Token(GreenToken::new(
            RSyntaxKind(T![shebang] as u16),
            SmolStr::from(shebang),
        )));
    }
    children.extend(attr.into_iter().map(|item| NodeOrToken::Node(item.syntax().green().clone())));
    children.extend(item.into_iter().map(|item| NodeOrToken::Node(item.syntax().green().clone())));
    let green_node = GreenNode::new(RSyntaxKind(SyntaxKind::SOURCE_FILE as u16), children);
    ast::SourceFile::cast(SyntaxNode::new_root(green_node)).unwrap()
}
pub fn enum_(
    attr: impl IntoIterator<Item = ast::Attr>,
    visibility: impl Into<Option<ast::Visibility>>,
    name: ast::Name,
    generic_param_list: impl Into<Option<ast::GenericParamList>>,
    where_clause: impl Into<Option<ast::WhereClause>>,
    variant_list: ast::VariantList,
) -> ast::Enum {
    let mut children = vec![];
    children.extend(attr.into_iter().map(|item| NodeOrToken::Node(item.syntax().green().clone())));
    if let Some(visibility) = visibility.into() {
        children.push(NodeOrToken::Node(visibility.syntax().green().clone()));
    }
    children.push(NodeOrToken::Token(GreenToken::new(
        RSyntaxKind(T![enum] as u16),
        SmolStr::from("enum"),
    )));
    children.push(NodeOrToken::Node(name.syntax().green().clone()));
    if let Some(generic_param_list) = generic_param_list.into() {
        children.push(NodeOrToken::Node(generic_param_list.syntax().green().clone()));
    }
    if let Some(where_clause) = where_clause.into() {
        children.push(NodeOrToken::Node(where_clause.syntax().green().clone()));
    }
    children.push(NodeOrToken::Node(variant_list.syntax().green().clone()));
    let green_node = GreenNode::new(RSyntaxKind(SyntaxKind::ENUM as u16), children);
    ast::Enum::cast(SyntaxNode::new_root(green_node)).unwrap()
}
pub fn extern_block(
    attr: impl IntoIterator<Item = ast::Attr>,
    abi: ast::Abi,
    extern_item_list: ast::ExternItemList,
) -> ast::ExternBlock {
    let mut children = vec![];
    children.extend(attr.into_iter().map(|item| NodeOrToken::Node(item.syntax().green().clone())));
    children.push(NodeOrToken::Node(abi.syntax().green().clone()));
    children.push(NodeOrToken::Node(extern_item_list.syntax().green().clone()));
    let green_node = GreenNode::new(RSyntaxKind(SyntaxKind::EXTERN_BLOCK as u16), children);
    ast::ExternBlock::cast(SyntaxNode::new_root(green_node)).unwrap()
}
pub fn extern_crate0(
    attr: impl IntoIterator<Item = ast::Attr>,
    visibility: impl Into<Option<ast::Visibility>>,
    name_ref: ast::NameRef,
    rename: impl Into<Option<ast::Rename>>,
) -> ast::ExternCrate {
    let mut children = vec![];
    children.extend(attr.into_iter().map(|item| NodeOrToken::Node(item.syntax().green().clone())));
    if let Some(visibility) = visibility.into() {
        children.push(NodeOrToken::Node(visibility.syntax().green().clone()));
    }
    children.push(NodeOrToken::Token(GreenToken::new(
        RSyntaxKind(T![extern] as u16),
        SmolStr::from("extern"),
    )));
    children.push(NodeOrToken::Token(GreenToken::new(
        RSyntaxKind(T![crate] as u16),
        SmolStr::from("crate"),
    )));
    children.push(NodeOrToken::Node(name_ref.syntax().green().clone()));
    if let Some(rename) = rename.into() {
        children.push(NodeOrToken::Node(rename.syntax().green().clone()));
    }
    children
        .push(NodeOrToken::Token(GreenToken::new(RSyntaxKind(T ! [;] as u16), SmolStr::from(";"))));
    let green_node = GreenNode::new(RSyntaxKind(SyntaxKind::EXTERN_CRATE as u16), children);
    ast::ExternCrate::cast(SyntaxNode::new_root(green_node)).unwrap()
}
pub fn extern_crate1(
    attr: impl IntoIterator<Item = ast::Attr>,
    visibility: impl Into<Option<ast::Visibility>>,
    rename: impl Into<Option<ast::Rename>>,
) -> ast::ExternCrate {
    let mut children = vec![];
    children.extend(attr.into_iter().map(|item| NodeOrToken::Node(item.syntax().green().clone())));
    if let Some(visibility) = visibility.into() {
        children.push(NodeOrToken::Node(visibility.syntax().green().clone()));
    }
    children.push(NodeOrToken::Token(GreenToken::new(
        RSyntaxKind(T![extern] as u16),
        SmolStr::from("extern"),
    )));
    children.push(NodeOrToken::Token(GreenToken::new(
        RSyntaxKind(T![crate] as u16),
        SmolStr::from("crate"),
    )));
    children.push(NodeOrToken::Token(GreenToken::new(
        RSyntaxKind(T![self] as u16),
        SmolStr::from("self"),
    )));
    if let Some(rename) = rename.into() {
        children.push(NodeOrToken::Node(rename.syntax().green().clone()));
    }
    children
        .push(NodeOrToken::Token(GreenToken::new(RSyntaxKind(T ! [;] as u16), SmolStr::from(";"))));
    let green_node = GreenNode::new(RSyntaxKind(SyntaxKind::EXTERN_CRATE as u16), children);
    ast::ExternCrate::cast(SyntaxNode::new_root(green_node)).unwrap()
}
pub fn module0(
    attr: impl IntoIterator<Item = ast::Attr>,
    visibility: impl Into<Option<ast::Visibility>>,
    name: ast::Name,
    item_list: ast::ItemList,
) -> ast::Module {
    let mut children = vec![];
    children.extend(attr.into_iter().map(|item| NodeOrToken::Node(item.syntax().green().clone())));
    if let Some(visibility) = visibility.into() {
        children.push(NodeOrToken::Node(visibility.syntax().green().clone()));
    }
    children.push(NodeOrToken::Token(GreenToken::new(
        RSyntaxKind(T![mod] as u16),
        SmolStr::from("mod"),
    )));
    children.push(NodeOrToken::Node(name.syntax().green().clone()));
    children.push(NodeOrToken::Node(item_list.syntax().green().clone()));
    let green_node = GreenNode::new(RSyntaxKind(SyntaxKind::MODULE as u16), children);
    ast::Module::cast(SyntaxNode::new_root(green_node)).unwrap()
}
pub fn module1(
    attr: impl IntoIterator<Item = ast::Attr>,
    visibility: impl Into<Option<ast::Visibility>>,
    name: ast::Name,
) -> ast::Module {
    let mut children = vec![];
    children.extend(attr.into_iter().map(|item| NodeOrToken::Node(item.syntax().green().clone())));
    if let Some(visibility) = visibility.into() {
        children.push(NodeOrToken::Node(visibility.syntax().green().clone()));
    }
    children.push(NodeOrToken::Token(GreenToken::new(
        RSyntaxKind(T![mod] as u16),
        SmolStr::from("mod"),
    )));
    children.push(NodeOrToken::Node(name.syntax().green().clone()));
    children
        .push(NodeOrToken::Token(GreenToken::new(RSyntaxKind(T ! [;] as u16), SmolStr::from(";"))));
    let green_node = GreenNode::new(RSyntaxKind(SyntaxKind::MODULE as u16), children);
    ast::Module::cast(SyntaxNode::new_root(green_node)).unwrap()
}
pub fn struct0(
    attr: impl IntoIterator<Item = ast::Attr>,
    visibility: impl Into<Option<ast::Visibility>>,
    name: ast::Name,
    generic_param_list: impl Into<Option<ast::GenericParamList>>,
    where_clause: impl Into<Option<ast::WhereClause>>,
    record_field_list: ast::RecordFieldList,
) -> ast::Struct {
    let mut children = vec![];
    children.extend(attr.into_iter().map(|item| NodeOrToken::Node(item.syntax().green().clone())));
    if let Some(visibility) = visibility.into() {
        children.push(NodeOrToken::Node(visibility.syntax().green().clone()));
    }
    children.push(NodeOrToken::Token(GreenToken::new(
        RSyntaxKind(T![struct] as u16),
        SmolStr::from("struct"),
    )));
    children.push(NodeOrToken::Node(name.syntax().green().clone()));
    if let Some(generic_param_list) = generic_param_list.into() {
        children.push(NodeOrToken::Node(generic_param_list.syntax().green().clone()));
    }
    if let Some(where_clause) = where_clause.into() {
        children.push(NodeOrToken::Node(where_clause.syntax().green().clone()));
    }
    children.push(NodeOrToken::Node(record_field_list.syntax().green().clone()));
    let green_node = GreenNode::new(RSyntaxKind(SyntaxKind::STRUCT as u16), children);
    ast::Struct::cast(SyntaxNode::new_root(green_node)).unwrap()
}
pub fn struct1(
    attr: impl IntoIterator<Item = ast::Attr>,
    visibility: impl Into<Option<ast::Visibility>>,
    name: ast::Name,
    generic_param_list: impl Into<Option<ast::GenericParamList>>,
    where_clause: impl Into<Option<ast::WhereClause>>,
) -> ast::Struct {
    let mut children = vec![];
    children.extend(attr.into_iter().map(|item| NodeOrToken::Node(item.syntax().green().clone())));
    if let Some(visibility) = visibility.into() {
        children.push(NodeOrToken::Node(visibility.syntax().green().clone()));
    }
    children.push(NodeOrToken::Token(GreenToken::new(
        RSyntaxKind(T![struct] as u16),
        SmolStr::from("struct"),
    )));
    children.push(NodeOrToken::Node(name.syntax().green().clone()));
    if let Some(generic_param_list) = generic_param_list.into() {
        children.push(NodeOrToken::Node(generic_param_list.syntax().green().clone()));
    }
    if let Some(where_clause) = where_clause.into() {
        children.push(NodeOrToken::Node(where_clause.syntax().green().clone()));
    }
    children
        .push(NodeOrToken::Token(GreenToken::new(RSyntaxKind(T ! [;] as u16), SmolStr::from(";"))));
    let green_node = GreenNode::new(RSyntaxKind(SyntaxKind::STRUCT as u16), children);
    ast::Struct::cast(SyntaxNode::new_root(green_node)).unwrap()
}
pub fn struct2(
    attr: impl IntoIterator<Item = ast::Attr>,
    visibility: impl Into<Option<ast::Visibility>>,
    name: ast::Name,
    generic_param_list: impl Into<Option<ast::GenericParamList>>,
    tuple_field_list: ast::TupleFieldList,
    where_clause: impl Into<Option<ast::WhereClause>>,
) -> ast::Struct {
    let mut children = vec![];
    children.extend(attr.into_iter().map(|item| NodeOrToken::Node(item.syntax().green().clone())));
    if let Some(visibility) = visibility.into() {
        children.push(NodeOrToken::Node(visibility.syntax().green().clone()));
    }
    children.push(NodeOrToken::Token(GreenToken::new(
        RSyntaxKind(T![struct] as u16),
        SmolStr::from("struct"),
    )));
    children.push(NodeOrToken::Node(name.syntax().green().clone()));
    if let Some(generic_param_list) = generic_param_list.into() {
        children.push(NodeOrToken::Node(generic_param_list.syntax().green().clone()));
    }
    children.push(NodeOrToken::Node(tuple_field_list.syntax().green().clone()));
    if let Some(where_clause) = where_clause.into() {
        children.push(NodeOrToken::Node(where_clause.syntax().green().clone()));
    }
    children
        .push(NodeOrToken::Token(GreenToken::new(RSyntaxKind(T ! [;] as u16), SmolStr::from(";"))));
    let green_node = GreenNode::new(RSyntaxKind(SyntaxKind::STRUCT as u16), children);
    ast::Struct::cast(SyntaxNode::new_root(green_node)).unwrap()
}
pub fn union(
    attr: impl IntoIterator<Item = ast::Attr>,
    visibility: impl Into<Option<ast::Visibility>>,
    name: ast::Name,
    generic_param_list: impl Into<Option<ast::GenericParamList>>,
    where_clause: impl Into<Option<ast::WhereClause>>,
    record_field_list: ast::RecordFieldList,
) -> ast::Union {
    let mut children = vec![];
    children.extend(attr.into_iter().map(|item| NodeOrToken::Node(item.syntax().green().clone())));
    if let Some(visibility) = visibility.into() {
        children.push(NodeOrToken::Node(visibility.syntax().green().clone()));
    }
    children.push(NodeOrToken::Token(GreenToken::new(
        RSyntaxKind(T![union] as u16),
        SmolStr::from("union"),
    )));
    children.push(NodeOrToken::Node(name.syntax().green().clone()));
    if let Some(generic_param_list) = generic_param_list.into() {
        children.push(NodeOrToken::Node(generic_param_list.syntax().green().clone()));
    }
    if let Some(where_clause) = where_clause.into() {
        children.push(NodeOrToken::Node(where_clause.syntax().green().clone()));
    }
    children.push(NodeOrToken::Node(record_field_list.syntax().green().clone()));
    let green_node = GreenNode::new(RSyntaxKind(SyntaxKind::UNION as u16), children);
    ast::Union::cast(SyntaxNode::new_root(green_node)).unwrap()
}
pub fn use_(
    attr: impl IntoIterator<Item = ast::Attr>,
    visibility: impl Into<Option<ast::Visibility>>,
    use_tree: ast::UseTree,
) -> ast::Use {
    let mut children = vec![];
    children.extend(attr.into_iter().map(|item| NodeOrToken::Node(item.syntax().green().clone())));
    if let Some(visibility) = visibility.into() {
        children.push(NodeOrToken::Node(visibility.syntax().green().clone()));
    }
    children.push(NodeOrToken::Token(GreenToken::new(
        RSyntaxKind(T![use] as u16),
        SmolStr::from("use"),
    )));
    children.push(NodeOrToken::Node(use_tree.syntax().green().clone()));
    children
        .push(NodeOrToken::Token(GreenToken::new(RSyntaxKind(T ! [;] as u16), SmolStr::from(";"))));
    let green_node = GreenNode::new(RSyntaxKind(SyntaxKind::USE as u16), children);
    ast::Use::cast(SyntaxNode::new_root(green_node)).unwrap()
}
pub fn item_list(
    attr: impl IntoIterator<Item = ast::Attr>,
    item: impl IntoIterator<Item = ast::Item>,
) -> ast::ItemList {
    let mut children = vec![];
    children
        .push(NodeOrToken::Token(GreenToken::new(RSyntaxKind(T!['{'] as u16), SmolStr::from("{"))));
    children.extend(attr.into_iter().map(|item| NodeOrToken::Node(item.syntax().green().clone())));
    children.extend(item.into_iter().map(|item| NodeOrToken::Node(item.syntax().green().clone())));
    children
        .push(NodeOrToken::Token(GreenToken::new(RSyntaxKind(T!['}'] as u16), SmolStr::from("}"))));
    let green_node = GreenNode::new(RSyntaxKind(SyntaxKind::ITEM_LIST as u16), children);
    ast::ItemList::cast(SyntaxNode::new_root(green_node)).unwrap()
}
pub fn rename0(name: ast::Name) -> ast::Rename {
    let mut children = vec![];
    children
        .push(NodeOrToken::Token(GreenToken::new(RSyntaxKind(T![as] as u16), SmolStr::from("as"))));
    children.push(NodeOrToken::Node(name.syntax().green().clone()));
    let green_node = GreenNode::new(RSyntaxKind(SyntaxKind::RENAME as u16), children);
    ast::Rename::cast(SyntaxNode::new_root(green_node)).unwrap()
}
pub fn rename1() -> ast::Rename {
    let mut children = vec![];
    children
        .push(NodeOrToken::Token(GreenToken::new(RSyntaxKind(T![as] as u16), SmolStr::from("as"))));
    children
        .push(NodeOrToken::Token(GreenToken::new(RSyntaxKind(T![_] as u16), SmolStr::from("_"))));
    let green_node = GreenNode::new(RSyntaxKind(SyntaxKind::RENAME as u16), children);
    ast::Rename::cast(SyntaxNode::new_root(green_node)).unwrap()
}
pub fn use_tree_list(use_tree: impl IntoIterator<Item = ast::UseTree>) -> ast::UseTreeList {
    let mut children = vec![];
    children
        .push(NodeOrToken::Token(GreenToken::new(RSyntaxKind(T!['{'] as u16), SmolStr::from("{"))));
    children.extend(
        use_tree
            .into_iter()
            .map(|item| NodeOrToken::Node(item.syntax().green().clone()))
            .intersperse(NodeOrToken::Token(GreenToken::new(
                RSyntaxKind(T ! [,] as u16),
                SmolStr::from(","),
            ))),
    );
    children
        .push(NodeOrToken::Token(GreenToken::new(RSyntaxKind(T!['}'] as u16), SmolStr::from("}"))));
    let green_node = GreenNode::new(RSyntaxKind(SyntaxKind::USE_TREE_LIST as u16), children);
    ast::UseTreeList::cast(SyntaxNode::new_root(green_node)).unwrap()
}
pub fn abi<'a>(string: impl Into<Option<&'a str>>) -> ast::Abi {
    let mut children = vec![];
    children.push(NodeOrToken::Token(GreenToken::new(
        RSyntaxKind(T![extern] as u16),
        SmolStr::from("extern"),
    )));
    if let Some(string) = string.into() {
        children.push(NodeOrToken::Token(GreenToken::new(
            RSyntaxKind(T![string] as u16),
            SmolStr::from(string),
        )));
    }
    let green_node = GreenNode::new(RSyntaxKind(SyntaxKind::ABI as u16), children);
    ast::Abi::cast(SyntaxNode::new_root(green_node)).unwrap()
}
pub fn generic_param_list(
    generic_param: impl IntoIterator<Item = ast::GenericParam>,
) -> ast::GenericParamList {
    let mut children = vec![];
    children
        .push(NodeOrToken::Token(GreenToken::new(RSyntaxKind(T ! [<] as u16), SmolStr::from("<"))));
    children.extend(
        generic_param
            .into_iter()
            .map(|item| NodeOrToken::Node(item.syntax().green().clone()))
            .intersperse(NodeOrToken::Token(GreenToken::new(
                RSyntaxKind(T ! [,] as u16),
                SmolStr::from(","),
            ))),
    );
    children
        .push(NodeOrToken::Token(GreenToken::new(RSyntaxKind(T ! [>] as u16), SmolStr::from(">"))));
    let green_node = GreenNode::new(RSyntaxKind(SyntaxKind::GENERIC_PARAM_LIST as u16), children);
    ast::GenericParamList::cast(SyntaxNode::new_root(green_node)).unwrap()
}
pub fn where_clause(where_pred: impl IntoIterator<Item = ast::WherePred>) -> ast::WhereClause {
    let mut children = vec![];
    children.push(NodeOrToken::Token(GreenToken::new(
        RSyntaxKind(T![where] as u16),
        SmolStr::from("where"),
    )));
    children.extend(
        where_pred
            .into_iter()
            .map(|item| NodeOrToken::Node(item.syntax().green().clone()))
            .intersperse(NodeOrToken::Token(GreenToken::new(
                RSyntaxKind(T ! [,] as u16),
                SmolStr::from(","),
            ))),
    );
    let green_node = GreenNode::new(RSyntaxKind(SyntaxKind::WHERE_CLAUSE as u16), children);
    ast::WhereClause::cast(SyntaxNode::new_root(green_node)).unwrap()
}
pub fn block_expr(
    attr: impl IntoIterator<Item = ast::Attr>,
    stmt: impl IntoIterator<Item = ast::Stmt>,
    expr: impl Into<Option<ast::Expr>>,
) -> ast::BlockExpr {
    let mut children = vec![];
    children
        .push(NodeOrToken::Token(GreenToken::new(RSyntaxKind(T!['{'] as u16), SmolStr::from("{"))));
    children.extend(attr.into_iter().map(|item| NodeOrToken::Node(item.syntax().green().clone())));
    children.extend(stmt.into_iter().map(|item| NodeOrToken::Node(item.syntax().green().clone())));
    if let Some(expr) = expr.into() {
        children.push(NodeOrToken::Node(expr.syntax().green().clone()));
    }
    children
        .push(NodeOrToken::Token(GreenToken::new(RSyntaxKind(T!['}'] as u16), SmolStr::from("}"))));
    let green_node = GreenNode::new(RSyntaxKind(SyntaxKind::BLOCK_EXPR as u16), children);
    ast::BlockExpr::cast(SyntaxNode::new_root(green_node)).unwrap()
}
pub fn param0(
    attr: impl IntoIterator<Item = ast::Attr>,
    pat: ast::Pat,
    ty: impl Into<Option<ast::Type>>,
) -> ast::Param {
    let mut children = vec![];
    children.extend(attr.into_iter().map(|item| NodeOrToken::Node(item.syntax().green().clone())));
    children.push(NodeOrToken::Node(pat.syntax().green().clone()));
    if let Some(ty) = ty.into() {
        children.push(NodeOrToken::Token(GreenToken::new(
            RSyntaxKind(T ! [:] as u16),
            SmolStr::from(":"),
        )));
        children.push(NodeOrToken::Node(ty.syntax().green().clone()));
    }
    let green_node = GreenNode::new(RSyntaxKind(SyntaxKind::PARAM as u16), children);
    ast::Param::cast(SyntaxNode::new_root(green_node)).unwrap()
}
pub fn param1(attr: impl IntoIterator<Item = ast::Attr>, ty: ast::Type) -> ast::Param {
    let mut children = vec![];
    children.extend(attr.into_iter().map(|item| NodeOrToken::Node(item.syntax().green().clone())));
    children.push(NodeOrToken::Node(ty.syntax().green().clone()));
    let green_node = GreenNode::new(RSyntaxKind(SyntaxKind::PARAM as u16), children);
    ast::Param::cast(SyntaxNode::new_root(green_node)).unwrap()
}
pub fn param2(attr: impl IntoIterator<Item = ast::Attr>) -> ast::Param {
    let mut children = vec![];
    children.extend(attr.into_iter().map(|item| NodeOrToken::Node(item.syntax().green().clone())));
    children.push(NodeOrToken::Token(GreenToken::new(
        RSyntaxKind(T ! [...] as u16),
        SmolStr::from("..."),
    )));
    let green_node = GreenNode::new(RSyntaxKind(SyntaxKind::PARAM as u16), children);
    ast::Param::cast(SyntaxNode::new_root(green_node)).unwrap()
}
pub fn record_field_list(
    record_field: impl IntoIterator<Item = ast::RecordField>,
) -> ast::RecordFieldList {
    let mut children = vec![];
    children
        .push(NodeOrToken::Token(GreenToken::new(RSyntaxKind(T!['{'] as u16), SmolStr::from("{"))));
    children.extend(
        record_field
            .into_iter()
            .map(|item| NodeOrToken::Node(item.syntax().green().clone()))
            .intersperse(NodeOrToken::Token(GreenToken::new(
                RSyntaxKind(T ! [,] as u16),
                SmolStr::from(","),
            ))),
    );
    children
        .push(NodeOrToken::Token(GreenToken::new(RSyntaxKind(T!['}'] as u16), SmolStr::from("}"))));
    let green_node = GreenNode::new(RSyntaxKind(SyntaxKind::RECORD_FIELD_LIST as u16), children);
    ast::RecordFieldList::cast(SyntaxNode::new_root(green_node)).unwrap()
}
pub fn tuple_field_list(
    tuple_field: impl IntoIterator<Item = ast::TupleField>,
) -> ast::TupleFieldList {
    let mut children = vec![];
    children
        .push(NodeOrToken::Token(GreenToken::new(RSyntaxKind(T!['('] as u16), SmolStr::from("("))));
    children.extend(
        tuple_field
            .into_iter()
            .map(|item| NodeOrToken::Node(item.syntax().green().clone()))
            .intersperse(NodeOrToken::Token(GreenToken::new(
                RSyntaxKind(T ! [,] as u16),
                SmolStr::from(","),
            ))),
    );
    children
        .push(NodeOrToken::Token(GreenToken::new(RSyntaxKind(T![')'] as u16), SmolStr::from(")"))));
    let green_node = GreenNode::new(RSyntaxKind(SyntaxKind::TUPLE_FIELD_LIST as u16), children);
    ast::TupleFieldList::cast(SyntaxNode::new_root(green_node)).unwrap()
}
pub fn record_field(
    attr: impl IntoIterator<Item = ast::Attr>,
    visibility: impl Into<Option<ast::Visibility>>,
    name: ast::Name,
    ty: ast::Type,
) -> ast::RecordField {
    let mut children = vec![];
    children.extend(attr.into_iter().map(|item| NodeOrToken::Node(item.syntax().green().clone())));
    if let Some(visibility) = visibility.into() {
        children.push(NodeOrToken::Node(visibility.syntax().green().clone()));
    }
    children.push(NodeOrToken::Node(name.syntax().green().clone()));
    children
        .push(NodeOrToken::Token(GreenToken::new(RSyntaxKind(T ! [:] as u16), SmolStr::from(":"))));
    children.push(NodeOrToken::Node(ty.syntax().green().clone()));
    let green_node = GreenNode::new(RSyntaxKind(SyntaxKind::RECORD_FIELD as u16), children);
    ast::RecordField::cast(SyntaxNode::new_root(green_node)).unwrap()
}
pub fn tuple_field(
    attr: impl IntoIterator<Item = ast::Attr>,
    visibility: impl Into<Option<ast::Visibility>>,
    ty: ast::Type,
) -> ast::TupleField {
    let mut children = vec![];
    children.extend(attr.into_iter().map(|item| NodeOrToken::Node(item.syntax().green().clone())));
    if let Some(visibility) = visibility.into() {
        children.push(NodeOrToken::Node(visibility.syntax().green().clone()));
    }
    children.push(NodeOrToken::Node(ty.syntax().green().clone()));
    let green_node = GreenNode::new(RSyntaxKind(SyntaxKind::TUPLE_FIELD as u16), children);
    ast::TupleField::cast(SyntaxNode::new_root(green_node)).unwrap()
}
pub fn variant_list(variant: impl IntoIterator<Item = ast::Variant>) -> ast::VariantList {
    let mut children = vec![];
    children
        .push(NodeOrToken::Token(GreenToken::new(RSyntaxKind(T!['{'] as u16), SmolStr::from("{"))));
    children.extend(
        variant
            .into_iter()
            .map(|item| NodeOrToken::Node(item.syntax().green().clone()))
            .intersperse(NodeOrToken::Token(GreenToken::new(
                RSyntaxKind(T ! [,] as u16),
                SmolStr::from(","),
            ))),
    );
    children
        .push(NodeOrToken::Token(GreenToken::new(RSyntaxKind(T!['}'] as u16), SmolStr::from("}"))));
    let green_node = GreenNode::new(RSyntaxKind(SyntaxKind::VARIANT_LIST as u16), children);
    ast::VariantList::cast(SyntaxNode::new_root(green_node)).unwrap()
}
pub fn variant(
    attr: impl IntoIterator<Item = ast::Attr>,
    visibility: impl Into<Option<ast::Visibility>>,
    name: ast::Name,
    field_list: ast::FieldList,
    expr: impl Into<Option<ast::Expr>>,
) -> ast::Variant {
    let mut children = vec![];
    children.extend(attr.into_iter().map(|item| NodeOrToken::Node(item.syntax().green().clone())));
    if let Some(visibility) = visibility.into() {
        children.push(NodeOrToken::Node(visibility.syntax().green().clone()));
    }
    children.push(NodeOrToken::Node(name.syntax().green().clone()));
    children.push(NodeOrToken::Node(field_list.syntax().green().clone()));
    if let Some(expr) = expr.into() {
        children.push(NodeOrToken::Token(GreenToken::new(
            RSyntaxKind(T ! [=] as u16),
            SmolStr::from("="),
        )));
        children.push(NodeOrToken::Node(expr.syntax().green().clone()));
    }
    let green_node = GreenNode::new(RSyntaxKind(SyntaxKind::VARIANT as u16), children);
    ast::Variant::cast(SyntaxNode::new_root(green_node)).unwrap()
}
pub fn assoc_item_list(
    attr: impl IntoIterator<Item = ast::Attr>,
    assoc_item: impl IntoIterator<Item = ast::AssocItem>,
) -> ast::AssocItemList {
    let mut children = vec![];
    children
        .push(NodeOrToken::Token(GreenToken::new(RSyntaxKind(T!['{'] as u16), SmolStr::from("{"))));
    children.extend(attr.into_iter().map(|item| NodeOrToken::Node(item.syntax().green().clone())));
    children.extend(
        assoc_item.into_iter().map(|item| NodeOrToken::Node(item.syntax().green().clone())),
    );
    children
        .push(NodeOrToken::Token(GreenToken::new(RSyntaxKind(T!['}'] as u16), SmolStr::from("}"))));
    let green_node = GreenNode::new(RSyntaxKind(SyntaxKind::ASSOC_ITEM_LIST as u16), children);
    ast::AssocItemList::cast(SyntaxNode::new_root(green_node)).unwrap()
}
pub fn extern_item_list(
    attr: impl IntoIterator<Item = ast::Attr>,
    extern_item: impl IntoIterator<Item = ast::ExternItem>,
) -> ast::ExternItemList {
    let mut children = vec![];
    children
        .push(NodeOrToken::Token(GreenToken::new(RSyntaxKind(T!['{'] as u16), SmolStr::from("{"))));
    children.extend(attr.into_iter().map(|item| NodeOrToken::Node(item.syntax().green().clone())));
    children.extend(
        extern_item.into_iter().map(|item| NodeOrToken::Node(item.syntax().green().clone())),
    );
    children
        .push(NodeOrToken::Token(GreenToken::new(RSyntaxKind(T!['}'] as u16), SmolStr::from("}"))));
    let green_node = GreenNode::new(RSyntaxKind(SyntaxKind::EXTERN_ITEM_LIST as u16), children);
    ast::ExternItemList::cast(SyntaxNode::new_root(green_node)).unwrap()
}
pub fn const_param(
    attr: impl IntoIterator<Item = ast::Attr>,
    name: ast::Name,
    ty: ast::Type,
    expr: impl Into<Option<ast::Expr>>,
) -> ast::ConstParam {
    let mut children = vec![];
    children.extend(attr.into_iter().map(|item| NodeOrToken::Node(item.syntax().green().clone())));
    children.push(NodeOrToken::Token(GreenToken::new(
        RSyntaxKind(T![const] as u16),
        SmolStr::from("const"),
    )));
    children.push(NodeOrToken::Node(name.syntax().green().clone()));
    children
        .push(NodeOrToken::Token(GreenToken::new(RSyntaxKind(T ! [:] as u16), SmolStr::from(":"))));
    children.push(NodeOrToken::Node(ty.syntax().green().clone()));
    if let Some(expr) = expr.into() {
        children.push(NodeOrToken::Token(GreenToken::new(
            RSyntaxKind(T ! [=] as u16),
            SmolStr::from("="),
        )));
        children.push(NodeOrToken::Node(expr.syntax().green().clone()));
    }
    let green_node = GreenNode::new(RSyntaxKind(SyntaxKind::CONST_PARAM as u16), children);
    ast::ConstParam::cast(SyntaxNode::new_root(green_node)).unwrap()
}
pub fn type_param(
    attr: impl IntoIterator<Item = ast::Attr>,
    name: ast::Name,
    type_bound_list: impl Into<Option<ast::TypeBoundList>>,
    ty: impl Into<Option<ast::Type>>,
) -> ast::TypeParam {
    let mut children = vec![];
    children.extend(attr.into_iter().map(|item| NodeOrToken::Node(item.syntax().green().clone())));
    children.push(NodeOrToken::Node(name.syntax().green().clone()));
    if let Some(type_bound_list) = type_bound_list.into() {
        children.push(NodeOrToken::Token(GreenToken::new(
            RSyntaxKind(T ! [:] as u16),
            SmolStr::from(":"),
        )));
        children.push(NodeOrToken::Node(type_bound_list.syntax().green().clone()));
    }
    if let Some(ty) = ty.into() {
        children.push(NodeOrToken::Token(GreenToken::new(
            RSyntaxKind(T ! [=] as u16),
            SmolStr::from("="),
        )));
        children.push(NodeOrToken::Node(ty.syntax().green().clone()));
    }
    let green_node = GreenNode::new(RSyntaxKind(SyntaxKind::TYPE_PARAM as u16), children);
    ast::TypeParam::cast(SyntaxNode::new_root(green_node)).unwrap()
}
pub fn where_pred0(
    generic_param_list: impl Into<Option<ast::GenericParamList>>,
    type_bound_list: ast::TypeBoundList,
) -> ast::WherePred {
    let mut children = vec![];
    if let Some(generic_param_list) = generic_param_list.into() {
        children.push(NodeOrToken::Token(GreenToken::new(
            RSyntaxKind(T![for] as u16),
            SmolStr::from("for"),
        )));
        children.push(NodeOrToken::Node(generic_param_list.syntax().green().clone()));
    }
    children.push(NodeOrToken::Token(GreenToken::new(
        RSyntaxKind(T![lifetime] as u16),
        SmolStr::from("lifetime"),
    )));
    children
        .push(NodeOrToken::Token(GreenToken::new(RSyntaxKind(T ! [:] as u16), SmolStr::from(":"))));
    children.push(NodeOrToken::Node(type_bound_list.syntax().green().clone()));
    let green_node = GreenNode::new(RSyntaxKind(SyntaxKind::WHERE_PRED as u16), children);
    ast::WherePred::cast(SyntaxNode::new_root(green_node)).unwrap()
}
pub fn where_pred1(
    generic_param_list: impl Into<Option<ast::GenericParamList>>,
    ty: ast::Type,
    type_bound_list: ast::TypeBoundList,
) -> ast::WherePred {
    let mut children = vec![];
    if let Some(generic_param_list) = generic_param_list.into() {
        children.push(NodeOrToken::Token(GreenToken::new(
            RSyntaxKind(T![for] as u16),
            SmolStr::from("for"),
        )));
        children.push(NodeOrToken::Node(generic_param_list.syntax().green().clone()));
    }
    children.push(NodeOrToken::Node(ty.syntax().green().clone()));
    children
        .push(NodeOrToken::Token(GreenToken::new(RSyntaxKind(T ! [:] as u16), SmolStr::from(":"))));
    children.push(NodeOrToken::Node(type_bound_list.syntax().green().clone()));
    let green_node = GreenNode::new(RSyntaxKind(SyntaxKind::WHERE_PRED as u16), children);
    ast::WherePred::cast(SyntaxNode::new_root(green_node)).unwrap()
}
pub fn let_stmt(
    attr: impl IntoIterator<Item = ast::Attr>,
    pat: ast::Pat,
    ty: impl Into<Option<ast::Type>>,
    expr: ast::Expr,
) -> ast::LetStmt {
    let mut children = vec![];
    children.extend(attr.into_iter().map(|item| NodeOrToken::Node(item.syntax().green().clone())));
    children.push(NodeOrToken::Token(GreenToken::new(
        RSyntaxKind(T![let] as u16),
        SmolStr::from("let"),
    )));
    children.push(NodeOrToken::Node(pat.syntax().green().clone()));
    if let Some(ty) = ty.into() {
        children.push(NodeOrToken::Token(GreenToken::new(
            RSyntaxKind(T ! [:] as u16),
            SmolStr::from(":"),
        )));
        children.push(NodeOrToken::Node(ty.syntax().green().clone()));
    }
    children
        .push(NodeOrToken::Token(GreenToken::new(RSyntaxKind(T ! [=] as u16), SmolStr::from("="))));
    children.push(NodeOrToken::Node(expr.syntax().green().clone()));
    children
        .push(NodeOrToken::Token(GreenToken::new(RSyntaxKind(T ! [;] as u16), SmolStr::from(";"))));
    let green_node = GreenNode::new(RSyntaxKind(SyntaxKind::LET_STMT as u16), children);
    ast::LetStmt::cast(SyntaxNode::new_root(green_node)).unwrap()
}
pub fn array_expr0(
    attr: impl IntoIterator<Item = ast::Attr>,
    attr1: impl IntoIterator<Item = ast::Attr>,
    expr: impl IntoIterator<Item = ast::Expr>,
) -> ast::ArrayExpr {
    let mut children = vec![];
    children.extend(attr.into_iter().map(|item| NodeOrToken::Node(item.syntax().green().clone())));
    children
        .push(NodeOrToken::Token(GreenToken::new(RSyntaxKind(T!['['] as u16), SmolStr::from("["))));
    children.extend(attr1.into_iter().map(|item| NodeOrToken::Node(item.syntax().green().clone())));
    children.extend(
        expr.into_iter().map(|item| NodeOrToken::Node(item.syntax().green().clone())).intersperse(
            NodeOrToken::Token(GreenToken::new(RSyntaxKind(T ! [,] as u16), SmolStr::from(","))),
        ),
    );
    children
        .push(NodeOrToken::Token(GreenToken::new(RSyntaxKind(T![']'] as u16), SmolStr::from("]"))));
    let green_node = GreenNode::new(RSyntaxKind(SyntaxKind::ARRAY_EXPR as u16), children);
    ast::ArrayExpr::cast(SyntaxNode::new_root(green_node)).unwrap()
}
pub fn array_expr1(
    attr: impl IntoIterator<Item = ast::Attr>,
    attr1: impl IntoIterator<Item = ast::Attr>,
    expr: ast::Expr,
    expr1: ast::Expr,
) -> ast::ArrayExpr {
    let mut children = vec![];
    children.extend(attr.into_iter().map(|item| NodeOrToken::Node(item.syntax().green().clone())));
    children
        .push(NodeOrToken::Token(GreenToken::new(RSyntaxKind(T!['['] as u16), SmolStr::from("["))));
    children.extend(attr1.into_iter().map(|item| NodeOrToken::Node(item.syntax().green().clone())));
    children.push(NodeOrToken::Node(expr.syntax().green().clone()));
    children
        .push(NodeOrToken::Token(GreenToken::new(RSyntaxKind(T ! [;] as u16), SmolStr::from(";"))));
    children.push(NodeOrToken::Node(expr1.syntax().green().clone()));
    children
        .push(NodeOrToken::Token(GreenToken::new(RSyntaxKind(T![']'] as u16), SmolStr::from("]"))));
    let green_node = GreenNode::new(RSyntaxKind(SyntaxKind::ARRAY_EXPR as u16), children);
    ast::ArrayExpr::cast(SyntaxNode::new_root(green_node)).unwrap()
}
pub fn await_expr(attr: impl IntoIterator<Item = ast::Attr>, expr: ast::Expr) -> ast::AwaitExpr {
    let mut children = vec![];
    children.extend(attr.into_iter().map(|item| NodeOrToken::Node(item.syntax().green().clone())));
    children.push(NodeOrToken::Node(expr.syntax().green().clone()));
    children
        .push(NodeOrToken::Token(GreenToken::new(RSyntaxKind(T ! [.] as u16), SmolStr::from("."))));
    children.push(NodeOrToken::Token(GreenToken::new(
        RSyntaxKind(T![await] as u16),
        SmolStr::from("await"),
    )));
    let green_node = GreenNode::new(RSyntaxKind(SyntaxKind::AWAIT_EXPR as u16), children);
    ast::AwaitExpr::cast(SyntaxNode::new_root(green_node)).unwrap()
}
pub fn bin_expr0(
    attr: impl IntoIterator<Item = ast::Attr>,
    expr: ast::Expr,
    expr1: ast::Expr,
) -> ast::BinExpr {
    let mut children = vec![];
    children.extend(attr.into_iter().map(|item| NodeOrToken::Node(item.syntax().green().clone())));
    children.push(NodeOrToken::Node(expr.syntax().green().clone()));
    children.push(NodeOrToken::Token(GreenToken::new(
        RSyntaxKind(T ! [||] as u16),
        SmolStr::from("||"),
    )));
    children.push(NodeOrToken::Node(expr1.syntax().green().clone()));
    let green_node = GreenNode::new(RSyntaxKind(SyntaxKind::BIN_EXPR as u16), children);
    ast::BinExpr::cast(SyntaxNode::new_root(green_node)).unwrap()
}
pub fn bin_expr1(
    attr: impl IntoIterator<Item = ast::Attr>,
    expr: ast::Expr,
    expr1: ast::Expr,
) -> ast::BinExpr {
    let mut children = vec![];
    children.extend(attr.into_iter().map(|item| NodeOrToken::Node(item.syntax().green().clone())));
    children.push(NodeOrToken::Node(expr.syntax().green().clone()));
    children.push(NodeOrToken::Token(GreenToken::new(
        RSyntaxKind(T ! [&&] as u16),
        SmolStr::from("&&"),
    )));
    children.push(NodeOrToken::Node(expr1.syntax().green().clone()));
    let green_node = GreenNode::new(RSyntaxKind(SyntaxKind::BIN_EXPR as u16), children);
    ast::BinExpr::cast(SyntaxNode::new_root(green_node)).unwrap()
}
pub fn bin_expr2(
    attr: impl IntoIterator<Item = ast::Attr>,
    expr: ast::Expr,
    expr1: ast::Expr,
) -> ast::BinExpr {
    let mut children = vec![];
    children.extend(attr.into_iter().map(|item| NodeOrToken::Node(item.syntax().green().clone())));
    children.push(NodeOrToken::Node(expr.syntax().green().clone()));
    children.push(NodeOrToken::Token(GreenToken::new(
        RSyntaxKind(T ! [==] as u16),
        SmolStr::from("=="),
    )));
    children.push(NodeOrToken::Node(expr1.syntax().green().clone()));
    let green_node = GreenNode::new(RSyntaxKind(SyntaxKind::BIN_EXPR as u16), children);
    ast::BinExpr::cast(SyntaxNode::new_root(green_node)).unwrap()
}
pub fn bin_expr3(
    attr: impl IntoIterator<Item = ast::Attr>,
    expr: ast::Expr,
    expr1: ast::Expr,
) -> ast::BinExpr {
    let mut children = vec![];
    children.extend(attr.into_iter().map(|item| NodeOrToken::Node(item.syntax().green().clone())));
    children.push(NodeOrToken::Node(expr.syntax().green().clone()));
    children.push(NodeOrToken::Token(GreenToken::new(
        RSyntaxKind(T ! [!=] as u16),
        SmolStr::from("!="),
    )));
    children.push(NodeOrToken::Node(expr1.syntax().green().clone()));
    let green_node = GreenNode::new(RSyntaxKind(SyntaxKind::BIN_EXPR as u16), children);
    ast::BinExpr::cast(SyntaxNode::new_root(green_node)).unwrap()
}
pub fn bin_expr4(
    attr: impl IntoIterator<Item = ast::Attr>,
    expr: ast::Expr,
    expr1: ast::Expr,
) -> ast::BinExpr {
    let mut children = vec![];
    children.extend(attr.into_iter().map(|item| NodeOrToken::Node(item.syntax().green().clone())));
    children.push(NodeOrToken::Node(expr.syntax().green().clone()));
    children.push(NodeOrToken::Token(GreenToken::new(
        RSyntaxKind(T ! [<=] as u16),
        SmolStr::from("<="),
    )));
    children.push(NodeOrToken::Node(expr1.syntax().green().clone()));
    let green_node = GreenNode::new(RSyntaxKind(SyntaxKind::BIN_EXPR as u16), children);
    ast::BinExpr::cast(SyntaxNode::new_root(green_node)).unwrap()
}
pub fn bin_expr5(
    attr: impl IntoIterator<Item = ast::Attr>,
    expr: ast::Expr,
    expr1: ast::Expr,
) -> ast::BinExpr {
    let mut children = vec![];
    children.extend(attr.into_iter().map(|item| NodeOrToken::Node(item.syntax().green().clone())));
    children.push(NodeOrToken::Node(expr.syntax().green().clone()));
    children.push(NodeOrToken::Token(GreenToken::new(
        RSyntaxKind(T ! [>=] as u16),
        SmolStr::from(">="),
    )));
    children.push(NodeOrToken::Node(expr1.syntax().green().clone()));
    let green_node = GreenNode::new(RSyntaxKind(SyntaxKind::BIN_EXPR as u16), children);
    ast::BinExpr::cast(SyntaxNode::new_root(green_node)).unwrap()
}
pub fn bin_expr6(
    attr: impl IntoIterator<Item = ast::Attr>,
    expr: ast::Expr,
    expr1: ast::Expr,
) -> ast::BinExpr {
    let mut children = vec![];
    children.extend(attr.into_iter().map(|item| NodeOrToken::Node(item.syntax().green().clone())));
    children.push(NodeOrToken::Node(expr.syntax().green().clone()));
    children
        .push(NodeOrToken::Token(GreenToken::new(RSyntaxKind(T ! [<] as u16), SmolStr::from("<"))));
    children.push(NodeOrToken::Node(expr1.syntax().green().clone()));
    let green_node = GreenNode::new(RSyntaxKind(SyntaxKind::BIN_EXPR as u16), children);
    ast::BinExpr::cast(SyntaxNode::new_root(green_node)).unwrap()
}
pub fn bin_expr7(
    attr: impl IntoIterator<Item = ast::Attr>,
    expr: ast::Expr,
    expr1: ast::Expr,
) -> ast::BinExpr {
    let mut children = vec![];
    children.extend(attr.into_iter().map(|item| NodeOrToken::Node(item.syntax().green().clone())));
    children.push(NodeOrToken::Node(expr.syntax().green().clone()));
    children
        .push(NodeOrToken::Token(GreenToken::new(RSyntaxKind(T ! [>] as u16), SmolStr::from(">"))));
    children.push(NodeOrToken::Node(expr1.syntax().green().clone()));
    let green_node = GreenNode::new(RSyntaxKind(SyntaxKind::BIN_EXPR as u16), children);
    ast::BinExpr::cast(SyntaxNode::new_root(green_node)).unwrap()
}
pub fn bin_expr8(
    attr: impl IntoIterator<Item = ast::Attr>,
    expr: ast::Expr,
    expr1: ast::Expr,
) -> ast::BinExpr {
    let mut children = vec![];
    children.extend(attr.into_iter().map(|item| NodeOrToken::Node(item.syntax().green().clone())));
    children.push(NodeOrToken::Node(expr.syntax().green().clone()));
    children
        .push(NodeOrToken::Token(GreenToken::new(RSyntaxKind(T ! [+] as u16), SmolStr::from("+"))));
    children.push(NodeOrToken::Node(expr1.syntax().green().clone()));
    let green_node = GreenNode::new(RSyntaxKind(SyntaxKind::BIN_EXPR as u16), children);
    ast::BinExpr::cast(SyntaxNode::new_root(green_node)).unwrap()
}
pub fn bin_expr9(
    attr: impl IntoIterator<Item = ast::Attr>,
    expr: ast::Expr,
    expr1: ast::Expr,
) -> ast::BinExpr {
    let mut children = vec![];
    children.extend(attr.into_iter().map(|item| NodeOrToken::Node(item.syntax().green().clone())));
    children.push(NodeOrToken::Node(expr.syntax().green().clone()));
    children
        .push(NodeOrToken::Token(GreenToken::new(RSyntaxKind(T ! [*] as u16), SmolStr::from("*"))));
    children.push(NodeOrToken::Node(expr1.syntax().green().clone()));
    let green_node = GreenNode::new(RSyntaxKind(SyntaxKind::BIN_EXPR as u16), children);
    ast::BinExpr::cast(SyntaxNode::new_root(green_node)).unwrap()
}
pub fn bin_expr10(
    attr: impl IntoIterator<Item = ast::Attr>,
    expr: ast::Expr,
    expr1: ast::Expr,
) -> ast::BinExpr {
    let mut children = vec![];
    children.extend(attr.into_iter().map(|item| NodeOrToken::Node(item.syntax().green().clone())));
    children.push(NodeOrToken::Node(expr.syntax().green().clone()));
    children
        .push(NodeOrToken::Token(GreenToken::new(RSyntaxKind(T ! [-] as u16), SmolStr::from("-"))));
    children.push(NodeOrToken::Node(expr1.syntax().green().clone()));
    let green_node = GreenNode::new(RSyntaxKind(SyntaxKind::BIN_EXPR as u16), children);
    ast::BinExpr::cast(SyntaxNode::new_root(green_node)).unwrap()
}
pub fn bin_expr11(
    attr: impl IntoIterator<Item = ast::Attr>,
    expr: ast::Expr,
    expr1: ast::Expr,
) -> ast::BinExpr {
    let mut children = vec![];
    children.extend(attr.into_iter().map(|item| NodeOrToken::Node(item.syntax().green().clone())));
    children.push(NodeOrToken::Node(expr.syntax().green().clone()));
    children
        .push(NodeOrToken::Token(GreenToken::new(RSyntaxKind(T ! [/] as u16), SmolStr::from("/"))));
    children.push(NodeOrToken::Node(expr1.syntax().green().clone()));
    let green_node = GreenNode::new(RSyntaxKind(SyntaxKind::BIN_EXPR as u16), children);
    ast::BinExpr::cast(SyntaxNode::new_root(green_node)).unwrap()
}
pub fn bin_expr12(
    attr: impl IntoIterator<Item = ast::Attr>,
    expr: ast::Expr,
    expr1: ast::Expr,
) -> ast::BinExpr {
    let mut children = vec![];
    children.extend(attr.into_iter().map(|item| NodeOrToken::Node(item.syntax().green().clone())));
    children.push(NodeOrToken::Node(expr.syntax().green().clone()));
    children
        .push(NodeOrToken::Token(GreenToken::new(RSyntaxKind(T ! [%] as u16), SmolStr::from("%"))));
    children.push(NodeOrToken::Node(expr1.syntax().green().clone()));
    let green_node = GreenNode::new(RSyntaxKind(SyntaxKind::BIN_EXPR as u16), children);
    ast::BinExpr::cast(SyntaxNode::new_root(green_node)).unwrap()
}
pub fn bin_expr13(
    attr: impl IntoIterator<Item = ast::Attr>,
    expr: ast::Expr,
    expr1: ast::Expr,
) -> ast::BinExpr {
    let mut children = vec![];
    children.extend(attr.into_iter().map(|item| NodeOrToken::Node(item.syntax().green().clone())));
    children.push(NodeOrToken::Node(expr.syntax().green().clone()));
    children.push(NodeOrToken::Token(GreenToken::new(
        RSyntaxKind(T ! [<<] as u16),
        SmolStr::from("<<"),
    )));
    children.push(NodeOrToken::Node(expr1.syntax().green().clone()));
    let green_node = GreenNode::new(RSyntaxKind(SyntaxKind::BIN_EXPR as u16), children);
    ast::BinExpr::cast(SyntaxNode::new_root(green_node)).unwrap()
}
pub fn bin_expr14(
    attr: impl IntoIterator<Item = ast::Attr>,
    expr: ast::Expr,
    expr1: ast::Expr,
) -> ast::BinExpr {
    let mut children = vec![];
    children.extend(attr.into_iter().map(|item| NodeOrToken::Node(item.syntax().green().clone())));
    children.push(NodeOrToken::Node(expr.syntax().green().clone()));
    children.push(NodeOrToken::Token(GreenToken::new(
        RSyntaxKind(T ! [>>] as u16),
        SmolStr::from(">>"),
    )));
    children.push(NodeOrToken::Node(expr1.syntax().green().clone()));
    let green_node = GreenNode::new(RSyntaxKind(SyntaxKind::BIN_EXPR as u16), children);
    ast::BinExpr::cast(SyntaxNode::new_root(green_node)).unwrap()
}
pub fn bin_expr15(
    attr: impl IntoIterator<Item = ast::Attr>,
    expr: ast::Expr,
    expr1: ast::Expr,
) -> ast::BinExpr {
    let mut children = vec![];
    children.extend(attr.into_iter().map(|item| NodeOrToken::Node(item.syntax().green().clone())));
    children.push(NodeOrToken::Node(expr.syntax().green().clone()));
    children
        .push(NodeOrToken::Token(GreenToken::new(RSyntaxKind(T ! [^] as u16), SmolStr::from("^"))));
    children.push(NodeOrToken::Node(expr1.syntax().green().clone()));
    let green_node = GreenNode::new(RSyntaxKind(SyntaxKind::BIN_EXPR as u16), children);
    ast::BinExpr::cast(SyntaxNode::new_root(green_node)).unwrap()
}
pub fn bin_expr16(
    attr: impl IntoIterator<Item = ast::Attr>,
    expr: ast::Expr,
    expr1: ast::Expr,
) -> ast::BinExpr {
    let mut children = vec![];
    children.extend(attr.into_iter().map(|item| NodeOrToken::Node(item.syntax().green().clone())));
    children.push(NodeOrToken::Node(expr.syntax().green().clone()));
    children
        .push(NodeOrToken::Token(GreenToken::new(RSyntaxKind(T ! [|] as u16), SmolStr::from("|"))));
    children.push(NodeOrToken::Node(expr1.syntax().green().clone()));
    let green_node = GreenNode::new(RSyntaxKind(SyntaxKind::BIN_EXPR as u16), children);
    ast::BinExpr::cast(SyntaxNode::new_root(green_node)).unwrap()
}
pub fn bin_expr17(
    attr: impl IntoIterator<Item = ast::Attr>,
    expr: ast::Expr,
    expr1: ast::Expr,
) -> ast::BinExpr {
    let mut children = vec![];
    children.extend(attr.into_iter().map(|item| NodeOrToken::Node(item.syntax().green().clone())));
    children.push(NodeOrToken::Node(expr.syntax().green().clone()));
    children
        .push(NodeOrToken::Token(GreenToken::new(RSyntaxKind(T ! [&] as u16), SmolStr::from("&"))));
    children.push(NodeOrToken::Node(expr1.syntax().green().clone()));
    let green_node = GreenNode::new(RSyntaxKind(SyntaxKind::BIN_EXPR as u16), children);
    ast::BinExpr::cast(SyntaxNode::new_root(green_node)).unwrap()
}
pub fn bin_expr18(
    attr: impl IntoIterator<Item = ast::Attr>,
    expr: ast::Expr,
    expr1: ast::Expr,
) -> ast::BinExpr {
    let mut children = vec![];
    children.extend(attr.into_iter().map(|item| NodeOrToken::Node(item.syntax().green().clone())));
    children.push(NodeOrToken::Node(expr.syntax().green().clone()));
    children
        .push(NodeOrToken::Token(GreenToken::new(RSyntaxKind(T ! [=] as u16), SmolStr::from("="))));
    children.push(NodeOrToken::Node(expr1.syntax().green().clone()));
    let green_node = GreenNode::new(RSyntaxKind(SyntaxKind::BIN_EXPR as u16), children);
    ast::BinExpr::cast(SyntaxNode::new_root(green_node)).unwrap()
}
pub fn bin_expr19(
    attr: impl IntoIterator<Item = ast::Attr>,
    expr: ast::Expr,
    expr1: ast::Expr,
) -> ast::BinExpr {
    let mut children = vec![];
    children.extend(attr.into_iter().map(|item| NodeOrToken::Node(item.syntax().green().clone())));
    children.push(NodeOrToken::Node(expr.syntax().green().clone()));
    children.push(NodeOrToken::Token(GreenToken::new(
        RSyntaxKind(T ! [+=] as u16),
        SmolStr::from("+="),
    )));
    children.push(NodeOrToken::Node(expr1.syntax().green().clone()));
    let green_node = GreenNode::new(RSyntaxKind(SyntaxKind::BIN_EXPR as u16), children);
    ast::BinExpr::cast(SyntaxNode::new_root(green_node)).unwrap()
}
pub fn bin_expr20(
    attr: impl IntoIterator<Item = ast::Attr>,
    expr: ast::Expr,
    expr1: ast::Expr,
) -> ast::BinExpr {
    let mut children = vec![];
    children.extend(attr.into_iter().map(|item| NodeOrToken::Node(item.syntax().green().clone())));
    children.push(NodeOrToken::Node(expr.syntax().green().clone()));
    children.push(NodeOrToken::Token(GreenToken::new(
        RSyntaxKind(T ! [/=] as u16),
        SmolStr::from("/="),
    )));
    children.push(NodeOrToken::Node(expr1.syntax().green().clone()));
    let green_node = GreenNode::new(RSyntaxKind(SyntaxKind::BIN_EXPR as u16), children);
    ast::BinExpr::cast(SyntaxNode::new_root(green_node)).unwrap()
}
pub fn bin_expr21(
    attr: impl IntoIterator<Item = ast::Attr>,
    expr: ast::Expr,
    expr1: ast::Expr,
) -> ast::BinExpr {
    let mut children = vec![];
    children.extend(attr.into_iter().map(|item| NodeOrToken::Node(item.syntax().green().clone())));
    children.push(NodeOrToken::Node(expr.syntax().green().clone()));
    children.push(NodeOrToken::Token(GreenToken::new(
        RSyntaxKind(T ! [*=] as u16),
        SmolStr::from("*="),
    )));
    children.push(NodeOrToken::Node(expr1.syntax().green().clone()));
    let green_node = GreenNode::new(RSyntaxKind(SyntaxKind::BIN_EXPR as u16), children);
    ast::BinExpr::cast(SyntaxNode::new_root(green_node)).unwrap()
}
pub fn bin_expr22(
    attr: impl IntoIterator<Item = ast::Attr>,
    expr: ast::Expr,
    expr1: ast::Expr,
) -> ast::BinExpr {
    let mut children = vec![];
    children.extend(attr.into_iter().map(|item| NodeOrToken::Node(item.syntax().green().clone())));
    children.push(NodeOrToken::Node(expr.syntax().green().clone()));
    children.push(NodeOrToken::Token(GreenToken::new(
        RSyntaxKind(T ! [%=] as u16),
        SmolStr::from("%="),
    )));
    children.push(NodeOrToken::Node(expr1.syntax().green().clone()));
    let green_node = GreenNode::new(RSyntaxKind(SyntaxKind::BIN_EXPR as u16), children);
    ast::BinExpr::cast(SyntaxNode::new_root(green_node)).unwrap()
}
pub fn bin_expr23(
    attr: impl IntoIterator<Item = ast::Attr>,
    expr: ast::Expr,
    expr1: ast::Expr,
) -> ast::BinExpr {
    let mut children = vec![];
    children.extend(attr.into_iter().map(|item| NodeOrToken::Node(item.syntax().green().clone())));
    children.push(NodeOrToken::Node(expr.syntax().green().clone()));
    children.push(NodeOrToken::Token(GreenToken::new(
        RSyntaxKind(T ! [>>=] as u16),
        SmolStr::from(">>="),
    )));
    children.push(NodeOrToken::Node(expr1.syntax().green().clone()));
    let green_node = GreenNode::new(RSyntaxKind(SyntaxKind::BIN_EXPR as u16), children);
    ast::BinExpr::cast(SyntaxNode::new_root(green_node)).unwrap()
}
pub fn bin_expr24(
    attr: impl IntoIterator<Item = ast::Attr>,
    expr: ast::Expr,
    expr1: ast::Expr,
) -> ast::BinExpr {
    let mut children = vec![];
    children.extend(attr.into_iter().map(|item| NodeOrToken::Node(item.syntax().green().clone())));
    children.push(NodeOrToken::Node(expr.syntax().green().clone()));
    children.push(NodeOrToken::Token(GreenToken::new(
        RSyntaxKind(T ! [<<=] as u16),
        SmolStr::from("<<="),
    )));
    children.push(NodeOrToken::Node(expr1.syntax().green().clone()));
    let green_node = GreenNode::new(RSyntaxKind(SyntaxKind::BIN_EXPR as u16), children);
    ast::BinExpr::cast(SyntaxNode::new_root(green_node)).unwrap()
}
pub fn bin_expr25(
    attr: impl IntoIterator<Item = ast::Attr>,
    expr: ast::Expr,
    expr1: ast::Expr,
) -> ast::BinExpr {
    let mut children = vec![];
    children.extend(attr.into_iter().map(|item| NodeOrToken::Node(item.syntax().green().clone())));
    children.push(NodeOrToken::Node(expr.syntax().green().clone()));
    children.push(NodeOrToken::Token(GreenToken::new(
        RSyntaxKind(T ! [-=] as u16),
        SmolStr::from("-="),
    )));
    children.push(NodeOrToken::Node(expr1.syntax().green().clone()));
    let green_node = GreenNode::new(RSyntaxKind(SyntaxKind::BIN_EXPR as u16), children);
    ast::BinExpr::cast(SyntaxNode::new_root(green_node)).unwrap()
}
pub fn bin_expr26(
    attr: impl IntoIterator<Item = ast::Attr>,
    expr: ast::Expr,
    expr1: ast::Expr,
) -> ast::BinExpr {
    let mut children = vec![];
    children.extend(attr.into_iter().map(|item| NodeOrToken::Node(item.syntax().green().clone())));
    children.push(NodeOrToken::Node(expr.syntax().green().clone()));
    children.push(NodeOrToken::Token(GreenToken::new(
        RSyntaxKind(T ! [|=] as u16),
        SmolStr::from("|="),
    )));
    children.push(NodeOrToken::Node(expr1.syntax().green().clone()));
    let green_node = GreenNode::new(RSyntaxKind(SyntaxKind::BIN_EXPR as u16), children);
    ast::BinExpr::cast(SyntaxNode::new_root(green_node)).unwrap()
}
pub fn bin_expr27(
    attr: impl IntoIterator<Item = ast::Attr>,
    expr: ast::Expr,
    expr1: ast::Expr,
) -> ast::BinExpr {
    let mut children = vec![];
    children.extend(attr.into_iter().map(|item| NodeOrToken::Node(item.syntax().green().clone())));
    children.push(NodeOrToken::Node(expr.syntax().green().clone()));
    children.push(NodeOrToken::Token(GreenToken::new(
        RSyntaxKind(T ! [&=] as u16),
        SmolStr::from("&="),
    )));
    children.push(NodeOrToken::Node(expr1.syntax().green().clone()));
    let green_node = GreenNode::new(RSyntaxKind(SyntaxKind::BIN_EXPR as u16), children);
    ast::BinExpr::cast(SyntaxNode::new_root(green_node)).unwrap()
}
pub fn bin_expr28(
    attr: impl IntoIterator<Item = ast::Attr>,
    expr: ast::Expr,
    expr1: ast::Expr,
) -> ast::BinExpr {
    let mut children = vec![];
    children.extend(attr.into_iter().map(|item| NodeOrToken::Node(item.syntax().green().clone())));
    children.push(NodeOrToken::Node(expr.syntax().green().clone()));
    children.push(NodeOrToken::Token(GreenToken::new(
        RSyntaxKind(T ! [^=] as u16),
        SmolStr::from("^="),
    )));
    children.push(NodeOrToken::Node(expr1.syntax().green().clone()));
    let green_node = GreenNode::new(RSyntaxKind(SyntaxKind::BIN_EXPR as u16), children);
    ast::BinExpr::cast(SyntaxNode::new_root(green_node)).unwrap()
}
pub fn box_expr(attr: impl IntoIterator<Item = ast::Attr>, expr: ast::Expr) -> ast::BoxExpr {
    let mut children = vec![];
    children.extend(attr.into_iter().map(|item| NodeOrToken::Node(item.syntax().green().clone())));
    children.push(NodeOrToken::Token(GreenToken::new(
        RSyntaxKind(T![box] as u16),
        SmolStr::from("box"),
    )));
    children.push(NodeOrToken::Node(expr.syntax().green().clone()));
    let green_node = GreenNode::new(RSyntaxKind(SyntaxKind::BOX_EXPR as u16), children);
    ast::BoxExpr::cast(SyntaxNode::new_root(green_node)).unwrap()
}
pub fn break_expr<'a>(
    attr: impl IntoIterator<Item = ast::Attr>,
    lifetime: impl Into<Option<&'a str>>,
    expr: impl Into<Option<ast::Expr>>,
) -> ast::BreakExpr {
    let mut children = vec![];
    children.extend(attr.into_iter().map(|item| NodeOrToken::Node(item.syntax().green().clone())));
    children.push(NodeOrToken::Token(GreenToken::new(
        RSyntaxKind(T![break] as u16),
        SmolStr::from("break"),
    )));
    if let Some(lifetime) = lifetime.into() {
        children.push(NodeOrToken::Token(GreenToken::new(
            RSyntaxKind(T![lifetime] as u16),
            SmolStr::from(lifetime),
        )));
    }
    if let Some(expr) = expr.into() {
        children.push(NodeOrToken::Node(expr.syntax().green().clone()));
    }
    let green_node = GreenNode::new(RSyntaxKind(SyntaxKind::BREAK_EXPR as u16), children);
    ast::BreakExpr::cast(SyntaxNode::new_root(green_node)).unwrap()
}
pub fn call_expr(
    attr: impl IntoIterator<Item = ast::Attr>,
    expr: ast::Expr,
    arg_list: ast::ArgList,
) -> ast::CallExpr {
    let mut children = vec![];
    children.extend(attr.into_iter().map(|item| NodeOrToken::Node(item.syntax().green().clone())));
    children.push(NodeOrToken::Node(expr.syntax().green().clone()));
    children.push(NodeOrToken::Node(arg_list.syntax().green().clone()));
    let green_node = GreenNode::new(RSyntaxKind(SyntaxKind::CALL_EXPR as u16), children);
    ast::CallExpr::cast(SyntaxNode::new_root(green_node)).unwrap()
}
pub fn cast_expr(
    attr: impl IntoIterator<Item = ast::Attr>,
    expr: ast::Expr,
    ty: ast::Type,
) -> ast::CastExpr {
    let mut children = vec![];
    children.extend(attr.into_iter().map(|item| NodeOrToken::Node(item.syntax().green().clone())));
    children.push(NodeOrToken::Node(expr.syntax().green().clone()));
    children
        .push(NodeOrToken::Token(GreenToken::new(RSyntaxKind(T![as] as u16), SmolStr::from("as"))));
    children.push(NodeOrToken::Node(ty.syntax().green().clone()));
    let green_node = GreenNode::new(RSyntaxKind(SyntaxKind::CAST_EXPR as u16), children);
    ast::CastExpr::cast(SyntaxNode::new_root(green_node)).unwrap()
}
pub fn continue_expr<'a>(
    attr: impl IntoIterator<Item = ast::Attr>,
    lifetime: impl Into<Option<&'a str>>,
) -> ast::ContinueExpr {
    let mut children = vec![];
    children.extend(attr.into_iter().map(|item| NodeOrToken::Node(item.syntax().green().clone())));
    children.push(NodeOrToken::Token(GreenToken::new(
        RSyntaxKind(T![continue] as u16),
        SmolStr::from("continue"),
    )));
    if let Some(lifetime) = lifetime.into() {
        children.push(NodeOrToken::Token(GreenToken::new(
            RSyntaxKind(T![lifetime] as u16),
            SmolStr::from(lifetime),
        )));
    }
    let green_node = GreenNode::new(RSyntaxKind(SyntaxKind::CONTINUE_EXPR as u16), children);
    ast::ContinueExpr::cast(SyntaxNode::new_root(green_node)).unwrap()
}
pub fn effect_expr0(
    attr: impl IntoIterator<Item = ast::Attr>,
    label: impl Into<Option<ast::Label>>,
    block_expr: ast::BlockExpr,
) -> ast::EffectExpr {
    let mut children = vec![];
    children.extend(attr.into_iter().map(|item| NodeOrToken::Node(item.syntax().green().clone())));
    if let Some(label) = label.into() {
        children.push(NodeOrToken::Node(label.syntax().green().clone()));
    }
    children.push(NodeOrToken::Token(GreenToken::new(
        RSyntaxKind(T![try] as u16),
        SmolStr::from("try"),
    )));
    children.push(NodeOrToken::Node(block_expr.syntax().green().clone()));
    let green_node = GreenNode::new(RSyntaxKind(SyntaxKind::EFFECT_EXPR as u16), children);
    ast::EffectExpr::cast(SyntaxNode::new_root(green_node)).unwrap()
}
pub fn effect_expr1(
    attr: impl IntoIterator<Item = ast::Attr>,
    label: impl Into<Option<ast::Label>>,
    block_expr: ast::BlockExpr,
) -> ast::EffectExpr {
    let mut children = vec![];
    children.extend(attr.into_iter().map(|item| NodeOrToken::Node(item.syntax().green().clone())));
    if let Some(label) = label.into() {
        children.push(NodeOrToken::Node(label.syntax().green().clone()));
    }
    children.push(NodeOrToken::Token(GreenToken::new(
        RSyntaxKind(T![unsafe] as u16),
        SmolStr::from("unsafe"),
    )));
    children.push(NodeOrToken::Node(block_expr.syntax().green().clone()));
    let green_node = GreenNode::new(RSyntaxKind(SyntaxKind::EFFECT_EXPR as u16), children);
    ast::EffectExpr::cast(SyntaxNode::new_root(green_node)).unwrap()
}
pub fn effect_expr2(
    attr: impl IntoIterator<Item = ast::Attr>,
    label: impl Into<Option<ast::Label>>,
    block_expr: ast::BlockExpr,
) -> ast::EffectExpr {
    let mut children = vec![];
    children.extend(attr.into_iter().map(|item| NodeOrToken::Node(item.syntax().green().clone())));
    if let Some(label) = label.into() {
        children.push(NodeOrToken::Node(label.syntax().green().clone()));
    }
    children.push(NodeOrToken::Token(GreenToken::new(
        RSyntaxKind(T![async] as u16),
        SmolStr::from("async"),
    )));
    children.push(NodeOrToken::Node(block_expr.syntax().green().clone()));
    let green_node = GreenNode::new(RSyntaxKind(SyntaxKind::EFFECT_EXPR as u16), children);
    ast::EffectExpr::cast(SyntaxNode::new_root(green_node)).unwrap()
}
pub fn field_expr(
    attr: impl IntoIterator<Item = ast::Attr>,
    expr: ast::Expr,
    name_ref: ast::NameRef,
) -> ast::FieldExpr {
    let mut children = vec![];
    children.extend(attr.into_iter().map(|item| NodeOrToken::Node(item.syntax().green().clone())));
    children.push(NodeOrToken::Node(expr.syntax().green().clone()));
    children
        .push(NodeOrToken::Token(GreenToken::new(RSyntaxKind(T ! [.] as u16), SmolStr::from("."))));
    children.push(NodeOrToken::Node(name_ref.syntax().green().clone()));
    let green_node = GreenNode::new(RSyntaxKind(SyntaxKind::FIELD_EXPR as u16), children);
    ast::FieldExpr::cast(SyntaxNode::new_root(green_node)).unwrap()
}
pub fn for_expr(
    attr: impl IntoIterator<Item = ast::Attr>,
    label: impl Into<Option<ast::Label>>,
    pat: ast::Pat,
    expr: ast::Expr,
    block_expr: ast::BlockExpr,
) -> ast::ForExpr {
    let mut children = vec![];
    children.extend(attr.into_iter().map(|item| NodeOrToken::Node(item.syntax().green().clone())));
    if let Some(label) = label.into() {
        children.push(NodeOrToken::Node(label.syntax().green().clone()));
    }
    children.push(NodeOrToken::Token(GreenToken::new(
        RSyntaxKind(T![for] as u16),
        SmolStr::from("for"),
    )));
    children.push(NodeOrToken::Node(pat.syntax().green().clone()));
    children
        .push(NodeOrToken::Token(GreenToken::new(RSyntaxKind(T![in] as u16), SmolStr::from("in"))));
    children.push(NodeOrToken::Node(expr.syntax().green().clone()));
    children.push(NodeOrToken::Node(block_expr.syntax().green().clone()));
    let green_node = GreenNode::new(RSyntaxKind(SyntaxKind::FOR_EXPR as u16), children);
    ast::ForExpr::cast(SyntaxNode::new_root(green_node)).unwrap()
}
pub fn if_expr0(
    attr: impl IntoIterator<Item = ast::Attr>,
    condition: ast::Condition,
    block_expr: ast::BlockExpr,
    if_expr: impl Into<Option<ast::IfExpr>>,
) -> ast::IfExpr {
    let mut children = vec![];
    children.extend(attr.into_iter().map(|item| NodeOrToken::Node(item.syntax().green().clone())));
    children
        .push(NodeOrToken::Token(GreenToken::new(RSyntaxKind(T![if] as u16), SmolStr::from("if"))));
    children.push(NodeOrToken::Node(condition.syntax().green().clone()));
    children.push(NodeOrToken::Node(block_expr.syntax().green().clone()));
    if let Some(if_expr) = if_expr.into() {
        children.push(NodeOrToken::Token(GreenToken::new(
            RSyntaxKind(T![else] as u16),
            SmolStr::from("else"),
        )));
        children.push(NodeOrToken::Node(if_expr.syntax().green().clone()));
    }
    let green_node = GreenNode::new(RSyntaxKind(SyntaxKind::IF_EXPR as u16), children);
    ast::IfExpr::cast(SyntaxNode::new_root(green_node)).unwrap()
}
pub fn if_expr1(
    attr: impl IntoIterator<Item = ast::Attr>,
    condition: ast::Condition,
    block_expr: ast::BlockExpr,
    block_expr1: impl Into<Option<ast::BlockExpr>>,
) -> ast::IfExpr {
    let mut children = vec![];
    children.extend(attr.into_iter().map(|item| NodeOrToken::Node(item.syntax().green().clone())));
    children
        .push(NodeOrToken::Token(GreenToken::new(RSyntaxKind(T![if] as u16), SmolStr::from("if"))));
    children.push(NodeOrToken::Node(condition.syntax().green().clone()));
    children.push(NodeOrToken::Node(block_expr.syntax().green().clone()));
    if let Some(block_expr1) = block_expr1.into() {
        children.push(NodeOrToken::Token(GreenToken::new(
            RSyntaxKind(T![else] as u16),
            SmolStr::from("else"),
        )));
        children.push(NodeOrToken::Node(block_expr1.syntax().green().clone()));
    }
    let green_node = GreenNode::new(RSyntaxKind(SyntaxKind::IF_EXPR as u16), children);
    ast::IfExpr::cast(SyntaxNode::new_root(green_node)).unwrap()
}
pub fn index_expr(
    attr: impl IntoIterator<Item = ast::Attr>,
    expr: ast::Expr,
    expr1: ast::Expr,
) -> ast::IndexExpr {
    let mut children = vec![];
    children.extend(attr.into_iter().map(|item| NodeOrToken::Node(item.syntax().green().clone())));
    children.push(NodeOrToken::Node(expr.syntax().green().clone()));
    children
        .push(NodeOrToken::Token(GreenToken::new(RSyntaxKind(T!['['] as u16), SmolStr::from("["))));
    children.push(NodeOrToken::Node(expr1.syntax().green().clone()));
    children
        .push(NodeOrToken::Token(GreenToken::new(RSyntaxKind(T![']'] as u16), SmolStr::from("]"))));
    let green_node = GreenNode::new(RSyntaxKind(SyntaxKind::INDEX_EXPR as u16), children);
    ast::IndexExpr::cast(SyntaxNode::new_root(green_node)).unwrap()
}
pub fn loop_expr(
    attr: impl IntoIterator<Item = ast::Attr>,
    label: impl Into<Option<ast::Label>>,
    block_expr: ast::BlockExpr,
) -> ast::LoopExpr {
    let mut children = vec![];
    children.extend(attr.into_iter().map(|item| NodeOrToken::Node(item.syntax().green().clone())));
    if let Some(label) = label.into() {
        children.push(NodeOrToken::Node(label.syntax().green().clone()));
    }
    children.push(NodeOrToken::Token(GreenToken::new(
        RSyntaxKind(T![loop] as u16),
        SmolStr::from("loop"),
    )));
    children.push(NodeOrToken::Node(block_expr.syntax().green().clone()));
    let green_node = GreenNode::new(RSyntaxKind(SyntaxKind::LOOP_EXPR as u16), children);
    ast::LoopExpr::cast(SyntaxNode::new_root(green_node)).unwrap()
}
pub fn match_expr(
    attr: impl IntoIterator<Item = ast::Attr>,
    expr: ast::Expr,
    match_arm_list: ast::MatchArmList,
) -> ast::MatchExpr {
    let mut children = vec![];
    children.extend(attr.into_iter().map(|item| NodeOrToken::Node(item.syntax().green().clone())));
    children.push(NodeOrToken::Token(GreenToken::new(
        RSyntaxKind(T![match] as u16),
        SmolStr::from("match"),
    )));
    children.push(NodeOrToken::Node(expr.syntax().green().clone()));
    children.push(NodeOrToken::Node(match_arm_list.syntax().green().clone()));
    let green_node = GreenNode::new(RSyntaxKind(SyntaxKind::MATCH_EXPR as u16), children);
    ast::MatchExpr::cast(SyntaxNode::new_root(green_node)).unwrap()
}
pub fn method_call_expr(
    attr: impl IntoIterator<Item = ast::Attr>,
    expr: ast::Expr,
    name_ref: ast::NameRef,
    generic_arg_list: impl Into<Option<ast::GenericArgList>>,
    arg_list: ast::ArgList,
) -> ast::MethodCallExpr {
    let mut children = vec![];
    children.extend(attr.into_iter().map(|item| NodeOrToken::Node(item.syntax().green().clone())));
    children.push(NodeOrToken::Node(expr.syntax().green().clone()));
    children
        .push(NodeOrToken::Token(GreenToken::new(RSyntaxKind(T ! [.] as u16), SmolStr::from("."))));
    children.push(NodeOrToken::Node(name_ref.syntax().green().clone()));
    if let Some(generic_arg_list) = generic_arg_list.into() {
        children.push(NodeOrToken::Node(generic_arg_list.syntax().green().clone()));
    }
    children.push(NodeOrToken::Node(arg_list.syntax().green().clone()));
    let green_node = GreenNode::new(RSyntaxKind(SyntaxKind::METHOD_CALL_EXPR as u16), children);
    ast::MethodCallExpr::cast(SyntaxNode::new_root(green_node)).unwrap()
}
pub fn paren_expr(
    attr: impl IntoIterator<Item = ast::Attr>,
    attr1: impl IntoIterator<Item = ast::Attr>,
    expr: ast::Expr,
) -> ast::ParenExpr {
    let mut children = vec![];
    children.extend(attr.into_iter().map(|item| NodeOrToken::Node(item.syntax().green().clone())));
    children
        .push(NodeOrToken::Token(GreenToken::new(RSyntaxKind(T!['('] as u16), SmolStr::from("("))));
    children.extend(attr1.into_iter().map(|item| NodeOrToken::Node(item.syntax().green().clone())));
    children.push(NodeOrToken::Node(expr.syntax().green().clone()));
    children
        .push(NodeOrToken::Token(GreenToken::new(RSyntaxKind(T![')'] as u16), SmolStr::from(")"))));
    let green_node = GreenNode::new(RSyntaxKind(SyntaxKind::PAREN_EXPR as u16), children);
    ast::ParenExpr::cast(SyntaxNode::new_root(green_node)).unwrap()
}
pub fn path_expr(attr: impl IntoIterator<Item = ast::Attr>, path: ast::Path) -> ast::PathExpr {
    let mut children = vec![];
    children.extend(attr.into_iter().map(|item| NodeOrToken::Node(item.syntax().green().clone())));
    children.push(NodeOrToken::Node(path.syntax().green().clone()));
    let green_node = GreenNode::new(RSyntaxKind(SyntaxKind::PATH_EXPR as u16), children);
    ast::PathExpr::cast(SyntaxNode::new_root(green_node)).unwrap()
}
pub fn prefix_expr0(attr: impl IntoIterator<Item = ast::Attr>, expr: ast::Expr) -> ast::PrefixExpr {
    let mut children = vec![];
    children.extend(attr.into_iter().map(|item| NodeOrToken::Node(item.syntax().green().clone())));
    children
        .push(NodeOrToken::Token(GreenToken::new(RSyntaxKind(T ! [-] as u16), SmolStr::from("-"))));
    children.push(NodeOrToken::Node(expr.syntax().green().clone()));
    let green_node = GreenNode::new(RSyntaxKind(SyntaxKind::PREFIX_EXPR as u16), children);
    ast::PrefixExpr::cast(SyntaxNode::new_root(green_node)).unwrap()
}
pub fn prefix_expr1(attr: impl IntoIterator<Item = ast::Attr>, expr: ast::Expr) -> ast::PrefixExpr {
    let mut children = vec![];
    children.extend(attr.into_iter().map(|item| NodeOrToken::Node(item.syntax().green().clone())));
    children
        .push(NodeOrToken::Token(GreenToken::new(RSyntaxKind(T![!] as u16), SmolStr::from("!"))));
    children.push(NodeOrToken::Node(expr.syntax().green().clone()));
    let green_node = GreenNode::new(RSyntaxKind(SyntaxKind::PREFIX_EXPR as u16), children);
    ast::PrefixExpr::cast(SyntaxNode::new_root(green_node)).unwrap()
}
pub fn prefix_expr2(attr: impl IntoIterator<Item = ast::Attr>, expr: ast::Expr) -> ast::PrefixExpr {
    let mut children = vec![];
    children.extend(attr.into_iter().map(|item| NodeOrToken::Node(item.syntax().green().clone())));
    children
        .push(NodeOrToken::Token(GreenToken::new(RSyntaxKind(T ! [*] as u16), SmolStr::from("*"))));
    children.push(NodeOrToken::Node(expr.syntax().green().clone()));
    let green_node = GreenNode::new(RSyntaxKind(SyntaxKind::PREFIX_EXPR as u16), children);
    ast::PrefixExpr::cast(SyntaxNode::new_root(green_node)).unwrap()
}
pub fn range_expr0(
    attr: impl IntoIterator<Item = ast::Attr>,
    expr: impl Into<Option<ast::Expr>>,
    expr1: impl Into<Option<ast::Expr>>,
) -> ast::RangeExpr {
    let mut children = vec![];
    children.extend(attr.into_iter().map(|item| NodeOrToken::Node(item.syntax().green().clone())));
    if let Some(expr) = expr.into() {
        children.push(NodeOrToken::Node(expr.syntax().green().clone()));
    }
    children
        .push(NodeOrToken::Token(GreenToken::new(RSyntaxKind(T![..] as u16), SmolStr::from(".."))));
    if let Some(expr1) = expr1.into() {
        children.push(NodeOrToken::Node(expr1.syntax().green().clone()));
    }
    let green_node = GreenNode::new(RSyntaxKind(SyntaxKind::RANGE_EXPR as u16), children);
    ast::RangeExpr::cast(SyntaxNode::new_root(green_node)).unwrap()
}
pub fn range_expr1(
    attr: impl IntoIterator<Item = ast::Attr>,
    expr: impl Into<Option<ast::Expr>>,
    expr1: impl Into<Option<ast::Expr>>,
) -> ast::RangeExpr {
    let mut children = vec![];
    children.extend(attr.into_iter().map(|item| NodeOrToken::Node(item.syntax().green().clone())));
    if let Some(expr) = expr.into() {
        children.push(NodeOrToken::Node(expr.syntax().green().clone()));
    }
    children.push(NodeOrToken::Token(GreenToken::new(
        RSyntaxKind(T ! [..=] as u16),
        SmolStr::from("..="),
    )));
    if let Some(expr1) = expr1.into() {
        children.push(NodeOrToken::Node(expr1.syntax().green().clone()));
    }
    let green_node = GreenNode::new(RSyntaxKind(SyntaxKind::RANGE_EXPR as u16), children);
    ast::RangeExpr::cast(SyntaxNode::new_root(green_node)).unwrap()
}
pub fn record_expr(
    path: ast::Path,
    record_expr_field_list: ast::RecordExprFieldList,
) -> ast::RecordExpr {
    let mut children = vec![];
    children.push(NodeOrToken::Node(path.syntax().green().clone()));
    children.push(NodeOrToken::Node(record_expr_field_list.syntax().green().clone()));
    let green_node = GreenNode::new(RSyntaxKind(SyntaxKind::RECORD_EXPR as u16), children);
    ast::RecordExpr::cast(SyntaxNode::new_root(green_node)).unwrap()
}
pub fn ref_expr0(attr: impl IntoIterator<Item = ast::Attr>, expr: ast::Expr) -> ast::RefExpr {
    let mut children = vec![];
    children.extend(attr.into_iter().map(|item| NodeOrToken::Node(item.syntax().green().clone())));
    children
        .push(NodeOrToken::Token(GreenToken::new(RSyntaxKind(T ! [&] as u16), SmolStr::from("&"))));
    children.push(NodeOrToken::Token(GreenToken::new(
        RSyntaxKind(T![raw] as u16),
        SmolStr::from("raw"),
    )));
    children.push(NodeOrToken::Node(expr.syntax().green().clone()));
    let green_node = GreenNode::new(RSyntaxKind(SyntaxKind::REF_EXPR as u16), children);
    ast::RefExpr::cast(SyntaxNode::new_root(green_node)).unwrap()
}
pub fn ref_expr1(attr: impl IntoIterator<Item = ast::Attr>, expr: ast::Expr) -> ast::RefExpr {
    let mut children = vec![];
    children.extend(attr.into_iter().map(|item| NodeOrToken::Node(item.syntax().green().clone())));
    children
        .push(NodeOrToken::Token(GreenToken::new(RSyntaxKind(T ! [&] as u16), SmolStr::from("&"))));
    children.push(NodeOrToken::Token(GreenToken::new(
        RSyntaxKind(T![mut] as u16),
        SmolStr::from("mut"),
    )));
    children.push(NodeOrToken::Node(expr.syntax().green().clone()));
    let green_node = GreenNode::new(RSyntaxKind(SyntaxKind::REF_EXPR as u16), children);
    ast::RefExpr::cast(SyntaxNode::new_root(green_node)).unwrap()
}
pub fn ref_expr2(attr: impl IntoIterator<Item = ast::Attr>, expr: ast::Expr) -> ast::RefExpr {
    let mut children = vec![];
    children.extend(attr.into_iter().map(|item| NodeOrToken::Node(item.syntax().green().clone())));
    children
        .push(NodeOrToken::Token(GreenToken::new(RSyntaxKind(T ! [&] as u16), SmolStr::from("&"))));
    children.push(NodeOrToken::Token(GreenToken::new(
        RSyntaxKind(T![const] as u16),
        SmolStr::from("const"),
    )));
    children.push(NodeOrToken::Node(expr.syntax().green().clone()));
    let green_node = GreenNode::new(RSyntaxKind(SyntaxKind::REF_EXPR as u16), children);
    ast::RefExpr::cast(SyntaxNode::new_root(green_node)).unwrap()
}
pub fn return_expr(
    attr: impl IntoIterator<Item = ast::Attr>,
    expr: impl Into<Option<ast::Expr>>,
) -> ast::ReturnExpr {
    let mut children = vec![];
    children.extend(attr.into_iter().map(|item| NodeOrToken::Node(item.syntax().green().clone())));
    children.push(NodeOrToken::Token(GreenToken::new(
        RSyntaxKind(T![return] as u16),
        SmolStr::from("return"),
    )));
    if let Some(expr) = expr.into() {
        children.push(NodeOrToken::Node(expr.syntax().green().clone()));
    }
    let green_node = GreenNode::new(RSyntaxKind(SyntaxKind::RETURN_EXPR as u16), children);
    ast::ReturnExpr::cast(SyntaxNode::new_root(green_node)).unwrap()
}
pub fn try_expr(attr: impl IntoIterator<Item = ast::Attr>, expr: ast::Expr) -> ast::TryExpr {
    let mut children = vec![];
    children.extend(attr.into_iter().map(|item| NodeOrToken::Node(item.syntax().green().clone())));
    children.push(NodeOrToken::Node(expr.syntax().green().clone()));
    children
        .push(NodeOrToken::Token(GreenToken::new(RSyntaxKind(T ! [?] as u16), SmolStr::from("?"))));
    let green_node = GreenNode::new(RSyntaxKind(SyntaxKind::TRY_EXPR as u16), children);
    ast::TryExpr::cast(SyntaxNode::new_root(green_node)).unwrap()
}
pub fn tuple_expr(
    attr: impl IntoIterator<Item = ast::Attr>,
    attr1: impl IntoIterator<Item = ast::Attr>,
    expr: impl IntoIterator<Item = ast::Expr>,
) -> ast::TupleExpr {
    let mut children = vec![];
    children.extend(attr.into_iter().map(|item| NodeOrToken::Node(item.syntax().green().clone())));
    children
        .push(NodeOrToken::Token(GreenToken::new(RSyntaxKind(T!['('] as u16), SmolStr::from("("))));
    children.extend(attr1.into_iter().map(|item| NodeOrToken::Node(item.syntax().green().clone())));
    children.extend(
        expr.into_iter().map(|item| NodeOrToken::Node(item.syntax().green().clone())).intersperse(
            NodeOrToken::Token(GreenToken::new(RSyntaxKind(T ! [,] as u16), SmolStr::from(","))),
        ),
    );
    children
        .push(NodeOrToken::Token(GreenToken::new(RSyntaxKind(T![')'] as u16), SmolStr::from(")"))));
    let green_node = GreenNode::new(RSyntaxKind(SyntaxKind::TUPLE_EXPR as u16), children);
    ast::TupleExpr::cast(SyntaxNode::new_root(green_node)).unwrap()
}
pub fn while_expr(
    attr: impl IntoIterator<Item = ast::Attr>,
    label: impl Into<Option<ast::Label>>,
    condition: ast::Condition,
    block_expr: ast::BlockExpr,
) -> ast::WhileExpr {
    let mut children = vec![];
    children.extend(attr.into_iter().map(|item| NodeOrToken::Node(item.syntax().green().clone())));
    if let Some(label) = label.into() {
        children.push(NodeOrToken::Node(label.syntax().green().clone()));
    }
    children.push(NodeOrToken::Token(GreenToken::new(
        RSyntaxKind(T![while] as u16),
        SmolStr::from("while"),
    )));
    children.push(NodeOrToken::Node(condition.syntax().green().clone()));
    children.push(NodeOrToken::Node(block_expr.syntax().green().clone()));
    let green_node = GreenNode::new(RSyntaxKind(SyntaxKind::WHILE_EXPR as u16), children);
    ast::WhileExpr::cast(SyntaxNode::new_root(green_node)).unwrap()
}
pub fn label<'a>(lifetime: &'a str) -> ast::Label {
    let mut children = vec![];
    children.push(NodeOrToken::Token(GreenToken::new(
        RSyntaxKind(T![lifetime] as u16),
        SmolStr::from(lifetime),
    )));
    let green_node = GreenNode::new(RSyntaxKind(SyntaxKind::LABEL as u16), children);
    ast::Label::cast(SyntaxNode::new_root(green_node)).unwrap()
}
pub fn record_expr_field_list(
    attr: impl IntoIterator<Item = ast::Attr>,
    record_expr_field: impl IntoIterator<Item = ast::RecordExprField>,
    expr: impl Into<Option<ast::Expr>>,
) -> ast::RecordExprFieldList {
    let mut children = vec![];
    children
        .push(NodeOrToken::Token(GreenToken::new(RSyntaxKind(T!['{'] as u16), SmolStr::from("{"))));
    children.extend(attr.into_iter().map(|item| NodeOrToken::Node(item.syntax().green().clone())));
    children.extend(
        record_expr_field
            .into_iter()
            .map(|item| NodeOrToken::Node(item.syntax().green().clone()))
            .intersperse(NodeOrToken::Token(GreenToken::new(
                RSyntaxKind(T ! [,] as u16),
                SmolStr::from(","),
            ))),
    );
    if let Some(expr) = expr.into() {
        children.push(NodeOrToken::Token(GreenToken::new(
            RSyntaxKind(T![..] as u16),
            SmolStr::from(".."),
        )));
        children.push(NodeOrToken::Node(expr.syntax().green().clone()));
    }
    children
        .push(NodeOrToken::Token(GreenToken::new(RSyntaxKind(T!['}'] as u16), SmolStr::from("}"))));
    let green_node =
        GreenNode::new(RSyntaxKind(SyntaxKind::RECORD_EXPR_FIELD_LIST as u16), children);
    ast::RecordExprFieldList::cast(SyntaxNode::new_root(green_node)).unwrap()
}
pub fn record_expr_field(
    attr: impl IntoIterator<Item = ast::Attr>,
    name_ref: ast::NameRef,
    expr: impl Into<Option<ast::Expr>>,
) -> ast::RecordExprField {
    let mut children = vec![];
    children.extend(attr.into_iter().map(|item| NodeOrToken::Node(item.syntax().green().clone())));
    children.push(NodeOrToken::Node(name_ref.syntax().green().clone()));
    if let Some(expr) = expr.into() {
        children.push(NodeOrToken::Token(GreenToken::new(
            RSyntaxKind(T ! [:] as u16),
            SmolStr::from(":"),
        )));
        children.push(NodeOrToken::Node(expr.syntax().green().clone()));
    }
    let green_node = GreenNode::new(RSyntaxKind(SyntaxKind::RECORD_EXPR_FIELD as u16), children);
    ast::RecordExprField::cast(SyntaxNode::new_root(green_node)).unwrap()
}
pub fn arg_list(expr: impl IntoIterator<Item = ast::Expr>) -> ast::ArgList {
    let mut children = vec![];
    children
        .push(NodeOrToken::Token(GreenToken::new(RSyntaxKind(T!['('] as u16), SmolStr::from("("))));
    children.extend(
        expr.into_iter().map(|item| NodeOrToken::Node(item.syntax().green().clone())).intersperse(
            NodeOrToken::Token(GreenToken::new(RSyntaxKind(T ! [,] as u16), SmolStr::from(","))),
        ),
    );
    children
        .push(NodeOrToken::Token(GreenToken::new(RSyntaxKind(T![')'] as u16), SmolStr::from(")"))));
    let green_node = GreenNode::new(RSyntaxKind(SyntaxKind::ARG_LIST as u16), children);
    ast::ArgList::cast(SyntaxNode::new_root(green_node)).unwrap()
}
pub fn condition0(pat: ast::Pat, expr: ast::Expr) -> ast::Condition {
    let mut children = vec![];
    children.push(NodeOrToken::Token(GreenToken::new(
        RSyntaxKind(T![let] as u16),
        SmolStr::from("let"),
    )));
    children.push(NodeOrToken::Node(pat.syntax().green().clone()));
    children
        .push(NodeOrToken::Token(GreenToken::new(RSyntaxKind(T ! [=] as u16), SmolStr::from("="))));
    children.push(NodeOrToken::Node(expr.syntax().green().clone()));
    let green_node = GreenNode::new(RSyntaxKind(SyntaxKind::CONDITION as u16), children);
    ast::Condition::cast(SyntaxNode::new_root(green_node)).unwrap()
}
pub fn condition1(expr: ast::Expr) -> ast::Condition {
    let mut children = vec![];
    children.push(NodeOrToken::Node(expr.syntax().green().clone()));
    let green_node = GreenNode::new(RSyntaxKind(SyntaxKind::CONDITION as u16), children);
    ast::Condition::cast(SyntaxNode::new_root(green_node)).unwrap()
}
pub fn match_arm_list(
    attr: impl IntoIterator<Item = ast::Attr>,
    match_arm: impl IntoIterator<Item = ast::MatchArm>,
) -> ast::MatchArmList {
    let mut children = vec![];
    children
        .push(NodeOrToken::Token(GreenToken::new(RSyntaxKind(T!['{'] as u16), SmolStr::from("{"))));
    children.extend(attr.into_iter().map(|item| NodeOrToken::Node(item.syntax().green().clone())));
    children
        .extend(match_arm.into_iter().map(|item| NodeOrToken::Node(item.syntax().green().clone())));
    children
        .push(NodeOrToken::Token(GreenToken::new(RSyntaxKind(T!['}'] as u16), SmolStr::from("}"))));
    let green_node = GreenNode::new(RSyntaxKind(SyntaxKind::MATCH_ARM_LIST as u16), children);
    ast::MatchArmList::cast(SyntaxNode::new_root(green_node)).unwrap()
}
pub fn match_guard(expr: ast::Expr) -> ast::MatchGuard {
    let mut children = vec![];
    children
        .push(NodeOrToken::Token(GreenToken::new(RSyntaxKind(T![if] as u16), SmolStr::from("if"))));
    children.push(NodeOrToken::Node(expr.syntax().green().clone()));
    let green_node = GreenNode::new(RSyntaxKind(SyntaxKind::MATCH_GUARD as u16), children);
    ast::MatchGuard::cast(SyntaxNode::new_root(green_node)).unwrap()
}
pub fn array_type(ty: ast::Type, expr: ast::Expr) -> ast::ArrayType {
    let mut children = vec![];
    children
        .push(NodeOrToken::Token(GreenToken::new(RSyntaxKind(T!['['] as u16), SmolStr::from("["))));
    children.push(NodeOrToken::Node(ty.syntax().green().clone()));
    children
        .push(NodeOrToken::Token(GreenToken::new(RSyntaxKind(T ! [;] as u16), SmolStr::from(";"))));
    children.push(NodeOrToken::Node(expr.syntax().green().clone()));
    children
        .push(NodeOrToken::Token(GreenToken::new(RSyntaxKind(T![']'] as u16), SmolStr::from("]"))));
    let green_node = GreenNode::new(RSyntaxKind(SyntaxKind::ARRAY_TYPE as u16), children);
    ast::ArrayType::cast(SyntaxNode::new_root(green_node)).unwrap()
}
pub fn dyn_trait_type(type_bound_list: ast::TypeBoundList) -> ast::DynTraitType {
    let mut children = vec![];
    children.push(NodeOrToken::Token(GreenToken::new(
        RSyntaxKind(T![dyn] as u16),
        SmolStr::from("dyn"),
    )));
    children.push(NodeOrToken::Node(type_bound_list.syntax().green().clone()));
    let green_node = GreenNode::new(RSyntaxKind(SyntaxKind::DYN_TRAIT_TYPE as u16), children);
    ast::DynTraitType::cast(SyntaxNode::new_root(green_node)).unwrap()
}
pub fn for_type(generic_param_list: ast::GenericParamList, ty: ast::Type) -> ast::ForType {
    let mut children = vec![];
    children.push(NodeOrToken::Token(GreenToken::new(
        RSyntaxKind(T![for] as u16),
        SmolStr::from("for"),
    )));
    children.push(NodeOrToken::Node(generic_param_list.syntax().green().clone()));
    children.push(NodeOrToken::Node(ty.syntax().green().clone()));
    let green_node = GreenNode::new(RSyntaxKind(SyntaxKind::FOR_TYPE as u16), children);
    ast::ForType::cast(SyntaxNode::new_root(green_node)).unwrap()
}
pub fn impl_trait_type(type_bound_list: ast::TypeBoundList) -> ast::ImplTraitType {
    let mut children = vec![];
    children.push(NodeOrToken::Token(GreenToken::new(
        RSyntaxKind(T![impl] as u16),
        SmolStr::from("impl"),
    )));
    children.push(NodeOrToken::Node(type_bound_list.syntax().green().clone()));
    let green_node = GreenNode::new(RSyntaxKind(SyntaxKind::IMPL_TRAIT_TYPE as u16), children);
    ast::ImplTraitType::cast(SyntaxNode::new_root(green_node)).unwrap()
}
pub fn infer_type() -> ast::InferType {
    let mut children = vec![];
    children
        .push(NodeOrToken::Token(GreenToken::new(RSyntaxKind(T![_] as u16), SmolStr::from("_"))));
    let green_node = GreenNode::new(RSyntaxKind(SyntaxKind::INFER_TYPE as u16), children);
    ast::InferType::cast(SyntaxNode::new_root(green_node)).unwrap()
}
pub fn never_type() -> ast::NeverType {
    let mut children = vec![];
    children
        .push(NodeOrToken::Token(GreenToken::new(RSyntaxKind(T![!] as u16), SmolStr::from("!"))));
    let green_node = GreenNode::new(RSyntaxKind(SyntaxKind::NEVER_TYPE as u16), children);
    ast::NeverType::cast(SyntaxNode::new_root(green_node)).unwrap()
}
pub fn paren_type(ty: ast::Type) -> ast::ParenType {
    let mut children = vec![];
    children
        .push(NodeOrToken::Token(GreenToken::new(RSyntaxKind(T!['('] as u16), SmolStr::from("("))));
    children.push(NodeOrToken::Node(ty.syntax().green().clone()));
    children
        .push(NodeOrToken::Token(GreenToken::new(RSyntaxKind(T![')'] as u16), SmolStr::from(")"))));
    let green_node = GreenNode::new(RSyntaxKind(SyntaxKind::PAREN_TYPE as u16), children);
    ast::ParenType::cast(SyntaxNode::new_root(green_node)).unwrap()
}
pub fn ptr_type0(ty: ast::Type) -> ast::PtrType {
    let mut children = vec![];
    children
        .push(NodeOrToken::Token(GreenToken::new(RSyntaxKind(T ! [*] as u16), SmolStr::from("*"))));
    children.push(NodeOrToken::Token(GreenToken::new(
        RSyntaxKind(T![const] as u16),
        SmolStr::from("const"),
    )));
    children.push(NodeOrToken::Node(ty.syntax().green().clone()));
    let green_node = GreenNode::new(RSyntaxKind(SyntaxKind::PTR_TYPE as u16), children);
    ast::PtrType::cast(SyntaxNode::new_root(green_node)).unwrap()
}
pub fn ptr_type1(ty: ast::Type) -> ast::PtrType {
    let mut children = vec![];
    children
        .push(NodeOrToken::Token(GreenToken::new(RSyntaxKind(T ! [*] as u16), SmolStr::from("*"))));
    children.push(NodeOrToken::Token(GreenToken::new(
        RSyntaxKind(T![mut] as u16),
        SmolStr::from("mut"),
    )));
    children.push(NodeOrToken::Node(ty.syntax().green().clone()));
    let green_node = GreenNode::new(RSyntaxKind(SyntaxKind::PTR_TYPE as u16), children);
    ast::PtrType::cast(SyntaxNode::new_root(green_node)).unwrap()
}
pub fn slice_type(ty: ast::Type) -> ast::SliceType {
    let mut children = vec![];
    children
        .push(NodeOrToken::Token(GreenToken::new(RSyntaxKind(T!['['] as u16), SmolStr::from("["))));
    children.push(NodeOrToken::Node(ty.syntax().green().clone()));
    children
        .push(NodeOrToken::Token(GreenToken::new(RSyntaxKind(T![']'] as u16), SmolStr::from("]"))));
    let green_node = GreenNode::new(RSyntaxKind(SyntaxKind::SLICE_TYPE as u16), children);
    ast::SliceType::cast(SyntaxNode::new_root(green_node)).unwrap()
}
pub fn tuple_type(ty: impl IntoIterator<Item = ast::Type>) -> ast::TupleType {
    let mut children = vec![];
    children
        .push(NodeOrToken::Token(GreenToken::new(RSyntaxKind(T!['('] as u16), SmolStr::from("("))));
    children.extend(
        ty.into_iter().map(|item| NodeOrToken::Node(item.syntax().green().clone())).intersperse(
            NodeOrToken::Token(GreenToken::new(RSyntaxKind(T ! [,] as u16), SmolStr::from(","))),
        ),
    );
    children
        .push(NodeOrToken::Token(GreenToken::new(RSyntaxKind(T![')'] as u16), SmolStr::from(")"))));
    let green_node = GreenNode::new(RSyntaxKind(SyntaxKind::TUPLE_TYPE as u16), children);
    ast::TupleType::cast(SyntaxNode::new_root(green_node)).unwrap()
}
pub fn box_pat(pat: ast::Pat) -> ast::BoxPat {
    let mut children = vec![];
    children.push(NodeOrToken::Token(GreenToken::new(
        RSyntaxKind(T![box] as u16),
        SmolStr::from("box"),
    )));
    children.push(NodeOrToken::Node(pat.syntax().green().clone()));
    let green_node = GreenNode::new(RSyntaxKind(SyntaxKind::BOX_PAT as u16), children);
    ast::BoxPat::cast(SyntaxNode::new_root(green_node)).unwrap()
}
pub fn rest_pat() -> ast::RestPat {
    let mut children = vec![];
    children
        .push(NodeOrToken::Token(GreenToken::new(RSyntaxKind(T![..] as u16), SmolStr::from(".."))));
    let green_node = GreenNode::new(RSyntaxKind(SyntaxKind::REST_PAT as u16), children);
    ast::RestPat::cast(SyntaxNode::new_root(green_node)).unwrap()
}
pub fn literal_pat(literal: ast::Literal) -> ast::LiteralPat {
    let mut children = vec![];
    children.push(NodeOrToken::Node(literal.syntax().green().clone()));
    let green_node = GreenNode::new(RSyntaxKind(SyntaxKind::LITERAL_PAT as u16), children);
    ast::LiteralPat::cast(SyntaxNode::new_root(green_node)).unwrap()
}
pub fn macro_pat(macro_call: ast::MacroCall) -> ast::MacroPat {
    let mut children = vec![];
    children.push(NodeOrToken::Node(macro_call.syntax().green().clone()));
    let green_node = GreenNode::new(RSyntaxKind(SyntaxKind::MACRO_PAT as u16), children);
    ast::MacroPat::cast(SyntaxNode::new_root(green_node)).unwrap()
}
pub fn or_pat(pat: impl IntoIterator<Item = ast::Pat>) -> ast::OrPat {
    let mut children = vec![];
    children.extend(
        pat.into_iter().map(|item| NodeOrToken::Node(item.syntax().green().clone())).intersperse(
            NodeOrToken::Token(GreenToken::new(RSyntaxKind(T ! [|] as u16), SmolStr::from("|"))),
        ),
    );
    let green_node = GreenNode::new(RSyntaxKind(SyntaxKind::OR_PAT as u16), children);
    ast::OrPat::cast(SyntaxNode::new_root(green_node)).unwrap()
}
pub fn paren_pat(pat: ast::Pat) -> ast::ParenPat {
    let mut children = vec![];
    children
        .push(NodeOrToken::Token(GreenToken::new(RSyntaxKind(T!['('] as u16), SmolStr::from("("))));
    children.push(NodeOrToken::Node(pat.syntax().green().clone()));
    children
        .push(NodeOrToken::Token(GreenToken::new(RSyntaxKind(T![')'] as u16), SmolStr::from(")"))));
    let green_node = GreenNode::new(RSyntaxKind(SyntaxKind::PAREN_PAT as u16), children);
    ast::ParenPat::cast(SyntaxNode::new_root(green_node)).unwrap()
}
pub fn path_pat(path: ast::Path) -> ast::PathPat {
    let mut children = vec![];
    children.push(NodeOrToken::Node(path.syntax().green().clone()));
    let green_node = GreenNode::new(RSyntaxKind(SyntaxKind::PATH_PAT as u16), children);
    ast::PathPat::cast(SyntaxNode::new_root(green_node)).unwrap()
}
pub fn wildcard_pat() -> ast::WildcardPat {
    let mut children = vec![];
    children
        .push(NodeOrToken::Token(GreenToken::new(RSyntaxKind(T![_] as u16), SmolStr::from("_"))));
    let green_node = GreenNode::new(RSyntaxKind(SyntaxKind::WILDCARD_PAT as u16), children);
    ast::WildcardPat::cast(SyntaxNode::new_root(green_node)).unwrap()
}
pub fn range_pat0(pat: ast::Pat, pat1: ast::Pat) -> ast::RangePat {
    let mut children = vec![];
    children.push(NodeOrToken::Node(pat.syntax().green().clone()));
    children
        .push(NodeOrToken::Token(GreenToken::new(RSyntaxKind(T![..] as u16), SmolStr::from(".."))));
    children.push(NodeOrToken::Node(pat1.syntax().green().clone()));
    let green_node = GreenNode::new(RSyntaxKind(SyntaxKind::RANGE_PAT as u16), children);
    ast::RangePat::cast(SyntaxNode::new_root(green_node)).unwrap()
}
pub fn range_pat1(pat: ast::Pat, pat1: ast::Pat) -> ast::RangePat {
    let mut children = vec![];
    children.push(NodeOrToken::Node(pat.syntax().green().clone()));
    children.push(NodeOrToken::Token(GreenToken::new(
        RSyntaxKind(T ! [..=] as u16),
        SmolStr::from("..="),
    )));
    children.push(NodeOrToken::Node(pat1.syntax().green().clone()));
    let green_node = GreenNode::new(RSyntaxKind(SyntaxKind::RANGE_PAT as u16), children);
    ast::RangePat::cast(SyntaxNode::new_root(green_node)).unwrap()
}
pub fn record_pat(
    path: ast::Path,
    record_pat_field_list: ast::RecordPatFieldList,
) -> ast::RecordPat {
    let mut children = vec![];
    children.push(NodeOrToken::Node(path.syntax().green().clone()));
    children.push(NodeOrToken::Node(record_pat_field_list.syntax().green().clone()));
    let green_node = GreenNode::new(RSyntaxKind(SyntaxKind::RECORD_PAT as u16), children);
    ast::RecordPat::cast(SyntaxNode::new_root(green_node)).unwrap()
}
pub fn slice_pat(pat: impl IntoIterator<Item = ast::Pat>) -> ast::SlicePat {
    let mut children = vec![];
    children
        .push(NodeOrToken::Token(GreenToken::new(RSyntaxKind(T!['['] as u16), SmolStr::from("["))));
    children.extend(
        pat.into_iter().map(|item| NodeOrToken::Node(item.syntax().green().clone())).intersperse(
            NodeOrToken::Token(GreenToken::new(RSyntaxKind(T ! [,] as u16), SmolStr::from(","))),
        ),
    );
    children
        .push(NodeOrToken::Token(GreenToken::new(RSyntaxKind(T![']'] as u16), SmolStr::from("]"))));
    let green_node = GreenNode::new(RSyntaxKind(SyntaxKind::SLICE_PAT as u16), children);
    ast::SlicePat::cast(SyntaxNode::new_root(green_node)).unwrap()
}
pub fn tuple_pat(pat: impl IntoIterator<Item = ast::Pat>) -> ast::TuplePat {
    let mut children = vec![];
    children
        .push(NodeOrToken::Token(GreenToken::new(RSyntaxKind(T!['('] as u16), SmolStr::from("("))));
    children.extend(
        pat.into_iter().map(|item| NodeOrToken::Node(item.syntax().green().clone())).intersperse(
            NodeOrToken::Token(GreenToken::new(RSyntaxKind(T ! [,] as u16), SmolStr::from(","))),
        ),
    );
    children
        .push(NodeOrToken::Token(GreenToken::new(RSyntaxKind(T![')'] as u16), SmolStr::from(")"))));
    let green_node = GreenNode::new(RSyntaxKind(SyntaxKind::TUPLE_PAT as u16), children);
    ast::TuplePat::cast(SyntaxNode::new_root(green_node)).unwrap()
}
pub fn tuple_struct_pat(
    path: ast::Path,
    pat: impl IntoIterator<Item = ast::Pat>,
) -> ast::TupleStructPat {
    let mut children = vec![];
    children.push(NodeOrToken::Node(path.syntax().green().clone()));
    children
        .push(NodeOrToken::Token(GreenToken::new(RSyntaxKind(T!['('] as u16), SmolStr::from("("))));
    children.extend(
        pat.into_iter().map(|item| NodeOrToken::Node(item.syntax().green().clone())).intersperse(
            NodeOrToken::Token(GreenToken::new(RSyntaxKind(T ! [,] as u16), SmolStr::from(","))),
        ),
    );
    children
        .push(NodeOrToken::Token(GreenToken::new(RSyntaxKind(T![')'] as u16), SmolStr::from(")"))));
    let green_node = GreenNode::new(RSyntaxKind(SyntaxKind::TUPLE_STRUCT_PAT as u16), children);
    ast::TupleStructPat::cast(SyntaxNode::new_root(green_node)).unwrap()
}
pub fn record_pat_field(
    attr: impl IntoIterator<Item = ast::Attr>,
    name_ref: impl Into<Option<ast::NameRef>>,
    pat: ast::Pat,
) -> ast::RecordPatField {
    let mut children = vec![];
    children.extend(attr.into_iter().map(|item| NodeOrToken::Node(item.syntax().green().clone())));
    if let Some(name_ref) = name_ref.into() {
        children.push(NodeOrToken::Node(name_ref.syntax().green().clone()));
        children.push(NodeOrToken::Token(GreenToken::new(
            RSyntaxKind(T ! [:] as u16),
            SmolStr::from(":"),
        )));
    }
    children.push(NodeOrToken::Node(pat.syntax().green().clone()));
    let green_node = GreenNode::new(RSyntaxKind(SyntaxKind::RECORD_PAT_FIELD as u16), children);
    ast::RecordPatField::cast(SyntaxNode::new_root(green_node)).unwrap()
}
