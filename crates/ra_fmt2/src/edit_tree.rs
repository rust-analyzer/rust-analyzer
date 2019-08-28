use crate::dsl::{Space, SpaceLoc, SpaceValue, SpacingDsl, SpacingRule};
// use crate::indent::Indentation;
use crate::pattern::{Pattern, PatternSet};
use crate::rules::spacing;
use crate::trav_util::{walk, walk_nodes, walk_tokens};
use crate::whitespace::Whitespace;

use ra_syntax::{
    NodeOrToken, SmolStr, SyntaxElement,
    SyntaxKind::{self, *}, Direction,
    SyntaxNode, SyntaxToken, TextRange, TextUnit, WalkEvent, T,
};

use std::collections::BTreeSet;
use std::cell::{Cell, RefCell};
use std::fmt::Write;

// TODO make more like intellij's fmt model
// Model holds immutable tree and mutable intermediate model to produce diff
// the model will probably have to create its own tree to add the extra
// info to each token/node:
//
// [1,2,3];
// can be Brace token, ident, comma all of which knows their own rules and apply
// them accordingly to produce [1, 2, 3]; ???

#[derive(Clone, Debug)]
/// Holds nodes and tokens as a tree with whitespace information
///
pub(crate) struct Block {
    //indent: some enum?
    element: SyntaxElement,
    // parent: Cell<Option<&Block>>,
    children: Vec<Block>,
    text: SmolStr,
    range: TextRange,
    whitespace: RefCell<Whitespace>,
}

impl Eq for Block {}
impl PartialEq for Block {
    fn eq(&self, rhs: &Block) -> bool {
        self.range == rhs.range && self.text == rhs.text
        && self.element == rhs.element
    }
}

impl Ord for Block {
    fn cmp(&self, rhs: &Block) -> std::cmp::Ordering {
        self.range.start().cmp(&rhs.range.start())
    }
}

impl PartialOrd for Block {
    fn partial_cmp(&self, rhs: &Block) -> Option<std::cmp::Ordering> {
        self.range.start().partial_cmp(&rhs.range.start())
    }
}

/// Block abstracts every node and token in a `SourceFile` of SyntaxElement`s, keeping indent
/// and `Whitespace` information for later formatting.
impl Block {
    /// Returns `Block` from either `SyntaxNode` or `SyntaxToken`.
    pub(crate) fn build_block(element: SyntaxElement) -> Block {
        // recursivly add to children
        let children = match &element {
            NodeOrToken::Node(node) => {
                node.children_with_tokens()
                .filter(|ele| match ele{
                    NodeOrToken::Node(_) => true,
                    NodeOrToken::Token(t) => t.kind() != WHITESPACE,
                })
                .map(Block::build_block)
                .collect::<Vec<_>>()
            }
            NodeOrToken::Token(_) => vec![],
        };
        let range = match &element {
            NodeOrToken::Node(node) => node.text_range(),
            NodeOrToken::Token(token) => token.text_range(),
        };
        let text = match &element {
            NodeOrToken::Node(node) => SmolStr::from(node.text().to_string()),
            NodeOrToken::Token(token) => token.text().clone(),
        };

        let whitespace = RefCell::new(Whitespace::new(&element));

        let whitespace = Rc::new(RefCell::new(Whitespace::new(&element)));
        let indentation = Rc::new(RefCell::new(Indentation::new(&element)));

        Self { element, text, children, range, whitespace, indentation, }
    }

    /// Compare pointers to check if two Blocks are equal.
    /// Remove??
    fn compare(&self, other: &Block) -> bool {
        self as *const _ == other as *const _
    }

    /// Returns an iterator of ancestor from current element.
    /// TODO cant return impl Iterator any ideas
    /// FIX probably not the best way to do this, building all new Blocks.
    pub(crate) fn ancestors_tokens(&self) -> Vec<Block> {
        match &self.element {
            NodeOrToken::Node(node) => {
                node.siblings_with_tokens(Direction::Prev)
                    .map(Block::build_single)
                    .collect::<Vec<_>>()
            },
            NodeOrToken::Token(token) => {
                token.siblings_with_tokens(Direction::Prev)
                    .map(Block::build_single)
                    .collect::<Vec<_>>()
            },
        }
    }
    pub(crate) fn ancestors_nodes(&self) -> Vec<Block> {
        match &self.element {
            NodeOrToken::Node(node) => {
                std::iter::successors(node.parent(), |this| {
                    this.parent()
                })
                .map(|n| Block::build_single(NodeOrToken::Node(n)))
                .collect::<Vec<_>>()
            },
            NodeOrToken::Token(token) => {
                std::iter::successors(Some(token.parent()), |this| {
                    this.parent()
                })
                .map(|n| Block::build_single(NodeOrToken::Node(n)))
                .collect::<Vec<_>>()
            },
        }
    }

    /// Traverse all blocks in order including current.
    pub(crate) fn traverse_inc(&self) -> impl Iterator<Item = &Block> {
        Traversal { blocks: self.order_flatten_blocks_inc(), idx: 0 }
    }

    /// Traverse all blocks in order excluding current.
    pub(crate) fn traverse_exc(&self) -> impl Iterator<Item = &Block> {
        Traversal { blocks: self.order_flatten_blocks_exc_curr(), idx: 0 }
    }

    /// Vec of all Blocks in order including current, parent then children.
    fn order_flatten_blocks_inc(&self) -> Vec<&Block> {
        let mut blocks = vec![self];
        for blk in self.children() {
            blocks.push(blk);
            if !blk.children.is_empty() {
                let mut kids = Block::order_flatten_blocks_inc(blk);
                blocks.append(&mut kids);
            }
        }
        blocks
    }

    /// Vec of all Blocks in order excluding current, parent then children.
    fn order_flatten_blocks_exc_curr(&self) -> Vec<&Block> {
        let mut blocks = vec![];
        for blk in self.children() {
            blocks.push(blk);
            if !blk.children.is_empty() {
                // we only want to exlcude the root
                let mut kids = Block::order_flatten_blocks_inc(blk);
                blocks.append(&mut kids);
            }
        }
        blocks
    }

    /// Vec of `Blocks` containing tokens, in order.
    fn order_flatten_blocks_tokens(&self) -> Vec<&Block> {
        let mut blocks = vec![];
        for blk in self.children() {
            if blk.as_element().as_token().is_some() {
                blocks.push(blk);
            }
            if !blk.children.is_empty() {
                let mut kids = Block::order_flatten_blocks_tokens(blk);
                blocks.append(&mut kids);
            }
        }
        blocks
    }

    /// Vec of `Blocks` containing nodes, in order.
    fn order_flatten_blocks_nodes(&self) -> Vec<&Block> {
        let mut blocks = vec![self];
        for blk in self.children() {
            if blk.as_element().as_node().is_some() {
                blocks.push(blk);
            }
            if !blk.children.is_empty() {
                let mut kids = Block::order_flatten_blocks_nodes(blk);
                blocks.append(&mut kids);
            }
        }
        blocks
    }

    /// Returns `Whitespace` which has knowledge of whitespace around current token.
    pub(crate) fn get_whitespace(&self) -> RefCell<Whitespace> {
        self.whitespace.clone()
    }

    /// Returns amount indenting whitespace.
    pub(crate) fn get_indent(&self) -> u32 {
        if self.whitespace.borrow().starts_with_lf {
            self.whitespace.borrow().text_len.0 
        } else {
            0
        }
    }

    /// Text range of current token.
    pub(crate) fn text_range(&self) -> TextRange {
        self.range
    }

    /// Returns an iterator of children from current element.
    pub(crate) fn children(&self) -> impl Iterator<Item = &Block> {
        self.children.iter()
    }

    /// Returns SyntaxKind.
    pub(crate) fn kind(&self) -> SyntaxKind {
        self.element.kind()
    }

    /// Returns an owned `SyntaxElement`.
    pub(crate) fn to_element(&self) -> SyntaxElement {
        self.element.clone()
    }

    /// Returns a reference to a `SyntaxElement`.
    pub(crate) fn as_element(&self) -> &SyntaxElement {
        &self.element
    }

    pub(crate) fn is_leaf(&self) -> bool {
        self.element.as_token().is_some()
    }

    /// Sets amount indenting whitespace.
    pub(crate) fn set_indent(&self, indent: u32) {
        self.whitespace.borrow_mut().text_len.0 = indent
    }

    /// Returns previous and next space amounts as tuple.
    pub(crate) fn space_value(&self) -> (u32, u32) {
        self.whitespace.borrow().text_len
    }

    /// Sets spacing based on rule.
    pub(crate) fn set_spacing(&self, rule: &SpacingRule) {
        self.whitespace.borrow_mut().apply_space_fix(rule)
    }

    /// Returns previous and next new line flags as tuple.
    pub(crate) fn eol_value(&self) -> (bool, bool) {
        self.whitespace.borrow().new_line
    }
    /// Returns true if `Block` starts with new line char.
    pub(crate) fn starts_with_lf(&self) -> bool {
        self.whitespace.borrow().starts_with_lf
    }

    /// Remove after dev ??
    fn to_string(&self) -> String {
        self.text.to_string()
    }

    /// Returns `Block`s text as str.
    pub(crate) fn as_str(&self) -> &str {
        self.text.as_str()
    }
}

#[derive(Debug, Clone)]
/// Traversal struct is the Iterator for flattened
/// ordered Block's, needed to fixes lifetime issue when
/// returning impl Iterator<_> for Block and EditTree.
pub(super) struct Traversal<'t> {
    blocks: Vec<&'t Block>,
    idx: usize,
}
impl<'t> Iterator for Traversal<'t> {
    type Item = &'t Block;

    fn next(&mut self) -> Option<Self::Item> {
        self.idx += 1;
        // copied otherwise we have a &&Block
        self.blocks.get(self.idx - 1).copied()
    }
}

#[derive(Debug, Clone)]
pub(crate) struct EditTree {
    root: Block,
}

impl EditTree {
    /// Walks all `SyntaxNode`s building an `EditTree`. 
    pub(crate) fn new(root: SyntaxNode) -> Self {
        EditTree::build_tree(root)
    }
    fn build_tree(root: SyntaxNode) -> EditTree {
        let ele = NodeOrToken::Node(root);
        let root = Block::build_block(ele);
        EditTree { root }
    }
    /// Returns the root node `SOURCE_FILE`.
    pub(crate) fn root(&self) -> &Block {
        &self.root
    }
    /// Returns the last token when ordered and flattened.
    pub(crate) fn last_token(&self) -> Option<&Block> {
        self.walk_tokens().last()
    }
    /// Walk all blocks including root.
    pub(crate) fn walk(&self) -> Traversal {
        Traversal { blocks: self.root.order_flatten_blocks_inc(), idx: 0 }
    }
    /// Walk blocks that represent tokens.
    pub(crate) fn walk_tokens(&self) -> Traversal {
        Traversal { blocks: self.root.order_flatten_blocks_tokens(), idx: 0 }
    }
    /// Walk blocks that represent nodes.
    pub(crate) fn walk_nodes(&self) -> Traversal {
        Traversal { blocks: self.root.order_flatten_blocks_nodes(), idx: 0 }
    }
    /// Walk all blocks excluding root.
    pub(crate) fn walk_exc_root(&self) -> Traversal {
        Traversal { blocks: self.root.order_flatten_blocks_exc_curr(), idx: 0 }
    }

    /// Returns the SmolStr of the root node, the whole text
    pub(crate) fn text(&self) -> SmolStr {
        self.root.text.clone()
    }

    /// TODO This needs work, less copying of the large vec of blocks
    /// Walks tokens and compares `Whitespace` to build the final String from `Blocks`.
    pub(crate) fn apply_edits(&self) -> Result<String, std::fmt::Error> {
        let traverse = self.walk_tokens();
        // scan's state var only needs to iter unique tokens.
        let de_dup = self.walk_tokens()
            .cloned()
            .collect::<BTreeSet<_>>();

        let mut iter_clone = de_dup.iter();
        // skip root
        iter_clone.next();
        // second token is scan's first state
        let first = iter_clone.next();
        let de_dup = self.walk_tokens().cloned().collect::<std::collections::BTreeSet<_>>();

        let mut iter_clone = de_dup.iter();
        // skip root
        iter_clone.next();
        // second token is scan's first state
        let first = iter_clone.next();
        traverse.scan(first, |next, blk| {
            let res = match blk.as_element() {
                NodeOrToken::Token(tkn) => {
                    if tkn.kind() != WHITESPACE {
                        let text = str_from_blk(&blk, *next).expect("failed to write to string");
                        *next = iter_clone.next();
                        text
                    } else {
                        "".into()
                    }
                },
                NodeOrToken::Node(_) => {
                    "".into()
                },
            };
            Some(res)
        })
        .collect::<String>();
        Ok(ret)
    }
}

fn str_from_root(block: &Block) -> String {
    let mut buff = String::new();
    eat_tkns(block, &mut buff);
    return buff;

    fn eat_tkns(block: &Block, mut buff: &mut String) {
        write!(buff, "{}", block.whitespace.borrow()).expect("write to string failed");
        if block.is_leaf() {
            write!(buff, "{}", block.element).expect("write to string failed");
        } else {
            block.children().for_each(|kid| eat_tkns(kid, &mut buff));
        }
    }
} 
// this currently works still uses next token to not duplicate whitespace
fn str_from_blk(block: &Block, next: Option<&Block>) -> Result<String, std::fmt::Error> {
    let mut buff = String::new();
    eat_tkns(block, next, &mut buff)?;
    return Ok(buff);

    fn eat_tkns(
        block: &Block,
        next: Option<&Block>,
        buff: &mut String
    ) -> std::fmt::Result {
        let ws = block.whitespace.borrow();
        let mut spaces = ws.to_space_text();

        //println!("CURR {:?}", spaces);

        if let Some(next_tkn) = next {
            let ws = next_tkn.whitespace.borrow();
            let next_spaces = ws.to_space_text();

            //println!("NEXT {:?}", next_spaces);

            // check if "after" whitespace and next token previous whitespace will double whitespace
            // or if next_spaces is indent
            if spaces[1] == next_spaces[0] || next_spaces[0].contains('\n') {
                spaces[1] = "".into();
            }
        };
        
        // TODO handle Err
        write!(buff, "{}", spaces[0])?;
        write!(buff, "{}", block.element)?;
        write!(buff, "{}", spaces[1])
    }
}

fn string_from_block(blk: &Block, next: &mut Option<&Block>) -> String {
    //println!{"BLK {:#?}\nNEXT {:#?}", blk, next}
    let mut ret = String::default();
    let (prev_s, next_s) = blk.space_value();
    let (prev_n, next_n) = blk.eol_value();

    // if new line
    if prev_n {
        ret.push('\n');
    // else push space
    } else {
        ret.push_str(&" ".repeat(prev_s as usize));
    }
    //println!{"BLK {:#?}\nNEXT {:#?}", current, next}
    let mut ret = String::default();

    let (curr_prev_space, curr_next_space) = current.space_value();
    let (curr_prev_lf, curr_next_lf) = current.eol_value();
    
    if let Some(block) = next {
        let (next_prev_space, _) = block.space_value();
        let (next_prev_lf, _) = block.eol_value();

        // TODO make sure "\n" will always come before " " do we need
        // to protect our indent info from spacing edits??

        // if new line
        if curr_prev_lf {
            ret.push('\n');
            if current.whitespace.borrow().starts_with_lf {
                ret.push_str(&" ".repeat(curr_prev_space as usize));
            }
        // else push space
        } else {
            ret.push_str(&" ".repeat(curr_prev_space as usize));
        }
        // add text token
        ret.push_str(current.as_str());

        // if the next token has no previous space but the current token has next space marked
        if next_prev_space == 0 && curr_next_space > 0 {
            ret.push_str(&" ".repeat(curr_next_space as usize));
        // same for new line add only if current says to and next does not
        } else if next_prev_lf && !curr_next_lf {
            ret.push('\n');
        }
    } else {
        //println!{"BLK {:#?}\nNEXT {:#?}", current, next}
        // if new line
        if curr_prev_lf {
            ret.push('\n');
            if current.whitespace.borrow().starts_with_lf {
                ret.push_str(&" ".repeat(curr_prev_space as usize));
            }
        // else push space
        } else {
            ret.push_str(&" ".repeat(curr_prev_space as usize));
        }
        // add text token
        ret.push_str(current.as_str());

        if curr_next_lf {
            ret.push('\n');
        // else push space
        } else {
            ret.push_str(&" ".repeat(curr_next_space as usize));
        }
    }
    // println!("{:?}", ret);
    ret
}
