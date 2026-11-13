//! Conversion from/to the flat tree for the proc macro server.

use crate::{
    flat::{FlatTree, ReaderTrait, SpanTransformer, SubtreeOrLeafRef, WriterTrait},
    token_stream::{Group, SpanLike, TokenStream, TokenTree},
};

struct Writer;

impl<'a, Span: Copy + 'a> WriterTrait<'a, Span> for Writer {
    type Subtree = &'a Group<Span>;
    type Leaf = &'a tt::Leaf<Span>;

    type SubtreeIter = Option<std::slice::Iter<'a, TokenTree<Span>>>;

    fn subtree_data(subtree: Self::Subtree) -> (usize, tt::Delimiter<Span>, Self::SubtreeIter) {
        (subtree.stream_len(), subtree.delimiter, subtree.stream.as_ref().map(|it| it.iter()))
    }

    fn subtree_iter_next(iter: &mut Self::SubtreeIter) -> Option<SubtreeOrLeafRef<'a, Span, Self>> {
        iter.as_mut()?.next().map(|item| match item {
            TokenTree::Leaf(leaf) => SubtreeOrLeafRef::Leaf(leaf),
            TokenTree::Group(group) => SubtreeOrLeafRef::Subtree(group),
        })
    }
}

struct Reader;

impl<Span: SpanLike> ReaderTrait<Span> for Reader {
    type TokenTree = TokenTree<Span>;

    fn leaf(leaf: tt::Leaf<Span>) -> Self::TokenTree {
        TokenTree::Leaf(leaf)
    }

    fn append_subtree(
        delimiter: tt::Delimiter<Span>,
        children: Vec<Self::TokenTree>,
        insert_into: &mut Vec<Self::TokenTree>,
    ) {
        insert_into.push(TokenTree::Group(Group {
            delimiter,
            stream: TokenStream::new_or_empty(children),
        }));
    }
}

impl FlatTree {
    pub fn from_tokenstream<ST: SpanTransformer>(
        tokenstream: TokenStream<ST::Span>,
        call_site: ST::Span,
        version: u32,
        span_data_table: &mut ST::Table,
    ) -> FlatTree {
        let root = if let Some(group) = tokenstream.as_single_group() {
            group.clone()
        } else {
            Group {
                delimiter: tt::Delimiter {
                    open: call_site,
                    close: call_site,
                    kind: tt::DelimiterKind::Invisible,
                },
                stream: Some(tokenstream),
            }
        };
        FlatTree::deserialize::<ST, Writer>(&root, version, span_data_table)
    }

    pub fn to_tokenstream<ST: SpanTransformer<Span: SpanLike>>(
        self,
        version: u32,
        span_data_table: &ST::Table,
    ) -> TokenStream<ST::Span> {
        let (top_delimiter, top_children) = self.serialize::<ST, Reader>(version, span_data_table);
        let result = if top_delimiter.kind == tt::DelimiterKind::Invisible {
            top_children
        } else {
            vec![TokenTree::Group(Group {
                delimiter: top_delimiter,
                stream: TokenStream::new_or_empty(top_children),
            })]
        };
        TokenStream::new(result)
    }
}
