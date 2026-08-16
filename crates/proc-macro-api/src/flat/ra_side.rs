//! Conversion from/to the flat tree for rust-analyzer.

use tt::Span;

use crate::flat::{FlatTree, ReaderTrait, SpanDataIndexMap, SubtreeOrLeafRef, WriterTrait};

struct Writer;

impl<'a> WriterTrait<'a, Span> for Writer {
    type Subtree = (tt::Subtree, tt::TtIter<'a>);
    type Leaf = tt::Leaf;

    type SubtreeIter = tt::TtIter<'a>;

    fn subtree_data((subtree, iter): Self::Subtree) -> (usize, tt::Delimiter, Self::SubtreeIter) {
        // FIXME: `count()` walks over the iterator.
        (iter.clone().count(), subtree.delimiter, iter)
    }

    fn subtree_iter_next(iter: &mut Self::SubtreeIter) -> Option<SubtreeOrLeafRef<'a, Span, Self>> {
        iter.next().map(|item| match item {
            tt::TtElement::Leaf(leaf) => SubtreeOrLeafRef::Leaf(leaf),
            tt::TtElement::Subtree(subtree, iter) => SubtreeOrLeafRef::Subtree((subtree, iter)),
        })
    }
}

struct Reader;

impl ReaderTrait<Span> for Reader {
    type TokenTree = tt::TokenTree;

    fn leaf(leaf: tt::Leaf<Span>) -> Self::TokenTree {
        tt::TokenTree::Leaf(leaf)
    }

    fn append_subtree(
        delimiter: tt::Delimiter,
        children: Vec<tt::TokenTree>,
        insert_into: &mut Vec<tt::TokenTree>,
    ) {
        insert_into
            .push(tt::TokenTree::Subtree(tt::Subtree { delimiter, len: children.len() as u32 }));
        insert_into.extend(children);
    }
}

impl FlatTree {
    pub fn from_subtree(
        subtree: tt::SubtreeView<'_>,
        version: u32,
        span_data_table: &mut SpanDataIndexMap,
    ) -> FlatTree {
        FlatTree::deserialize::<Span, Writer>(
            (subtree.top_subtree(), subtree.iter()),
            version,
            span_data_table,
        )
    }

    pub fn to_subtree(self, version: u32, span_data_table: &SpanDataIndexMap) -> tt::TopSubtree {
        let (top_delimiter, mut top_children) =
            self.serialize::<Span, Reader>(version, span_data_table);
        top_children.insert(
            0,
            tt::TokenTree::Subtree(tt::Subtree {
                delimiter: top_delimiter,
                len: top_children.len() as u32,
            }),
        );
        tt::TopSubtree::from_serialized(top_children)
    }
}
