use sws_tree::{tree, Tree};

#[test]
fn tree_new() {
    let tree = Tree::new('a');
    let root = tree.root();

    assert_eq!(Some('a'), root.map_value(|&v| v));
    assert_eq!(None, root.parent());
    assert_eq!(None, root.prev_sibling());
    assert_eq!(None, root.next_sibling());
    assert_eq!(None, root.first_child());
    assert_eq!(None, root.last_child());
}

#[test]
fn tree_root() {
    let tree = Tree::new('a');

    assert_eq!(Some('a'), tree.root().map_value(|&v| v));
}

#[test]
fn tree_get() {
    let tree = Tree::new('a');
    let id = tree.root().id();

    assert_eq!(Some(tree.root()), tree.get(id));
    assert_eq!(
        Some('a'),
        tree.get(id).and_then(|node| node.map_value(|&v| v))
    );
}

#[test]
fn tree_eq() {
    let one = Tree::new('a');
    let two = Tree::new('a');

    assert_eq!(one, two);
}

#[test]
fn tree_neq() {
    let one = Tree::new('a');
    let two = Tree::new('b');

    assert_ne!(one, two);
}

#[test]
fn macro_single_child() {
    let macro_tree = tree!('a' => { 'b' });

    let manual_tree = Tree::new('a');
    manual_tree.root().append('b');

    assert_eq!(manual_tree, macro_tree);
}

#[test]
fn macro_single_child_comma() {
    let macro_tree = tree! {
        'a' => {
            'b',
        }
    };

    let manual_tree = Tree::new('a');
    manual_tree.root().append('b');

    assert_eq!(manual_tree, macro_tree);
}

#[test]
fn macro_leaves() {
    let macro_tree = tree!('a' => { 'b', 'c', 'd' });

    let manual_tree = Tree::new('a');
    manual_tree.root().append('b');
    manual_tree.root().append('c');
    manual_tree.root().append('d');

    assert_eq!(manual_tree, macro_tree);
}

#[test]
fn macro_nested_single_child() {
    let macro_tree = tree!('a' => { 'b' => { 'c' } });

    let manual_tree = Tree::new('a');
    manual_tree.root().append('b').unwrap().append('c');

    assert_eq!(manual_tree, macro_tree);
}

#[test]
fn macro_nested_leaves() {
    let macro_tree = tree!('a' => { 'b' => { 'c', 'd', 'e' } });

    let manual_tree = Tree::new('a');
    {
        let mut root = manual_tree.root();
        let mut node = root.append('b').unwrap();
        node.append('c');
        node.append('d');
        node.append('e');
    }

    assert_eq!(manual_tree, macro_tree);
}

#[test]
fn macro_nested_nested() {
    let macro_tree = tree!('a' => { 'b' => { 'c' => { 'd' } } });

    let manual_tree = Tree::new('a');
    manual_tree
        .root()
        .append('b')
        .unwrap()
        .append('c')
        .unwrap()
        .append('d');

    assert_eq!(manual_tree, macro_tree);
}

#[test]
fn macro_mixed() {
    let macro_tree = tree! {
        'a' => {
            'b',
            'd' => { 'e', 'f' },
            'g' => { 'h' => { 'i' } },
            'j',
        }
    };

    let manual_tree = Tree::new('a');
    {
        let mut node = manual_tree.root();
        node.append('b');
        {
            let mut d = node.append('d').unwrap();
            d.append('e');
            d.append('f');
        }
        node.append('g').unwrap().append('h').unwrap().append('i');
        node.append('j');
    }

    assert_eq!(manual_tree, macro_tree);
}
