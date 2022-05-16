use sws_tree::{iter::Edge, tree};

#[test]
fn try_into_iter() {
    let tree = tree!('a' => { 'b', 'c', 'd' });
    assert_eq!(
        vec!['a', 'b', 'c', 'd'],
        tree.try_into_iter().unwrap().collect::<Vec<_>>()
    );
}

#[test]
fn iter_values() {
    let tree = tree!('a' => { 'b', 'c', 'd' });
    let nodes = tree.nodes();
    assert_eq!(
        vec!['a', 'b', 'c', 'd'],
        nodes
            .into_iter()
            .map(|(_, node)| *node.value())
            .collect::<Vec<_>>()
    );
}

#[test]
fn iter_values_mut() {
    let tree = tree!('a' => { 'b', 'c', 'd' });

    for (_, node) in &tree.nodes() {
        let c = node.value().to_ascii_uppercase();
        *node.value_mut() = c;
    }

    let nodes = tree.nodes();
    assert_eq!(
        vec!['A', 'B', 'C', 'D'],
        nodes
            .into_iter()
            .map(|(_, node)| *node.value())
            .collect::<Vec<_>>()
    );
}

#[test]
fn iter_nodes() {
    let tree = tree!('a' => { 'b' => { 'c' }, 'd' });

    let e = tree.orphan('e');
    tree.get(e).map(|mut e| e.append('f'));
    tree.root().append('g');

    let nodes = tree.nodes();
    assert_eq!(
        vec!['a', 'b', 'c', 'd', 'e', 'f', 'g'],
        nodes
            .into_iter()
            .map(|(_, node)| *node.value())
            .collect::<Vec<_>>()
    );
}

#[test]
fn iter_ancestors() {
    let tree = tree!('a' => { 'b' => { 'c' => { 'd' } } });
    let d = tree
        .root()
        .last_child()
        .unwrap()
        .last_child()
        .unwrap()
        .last_child()
        .unwrap();

    assert_eq!(
        vec!['c', 'b', 'a'],
        d.ancestors()
            .filter_map(|nref| nref.map_value(|&c| c))
            .collect::<Vec<_>>()
    );
}

#[test]
fn iter_prev_siblings() {
    let tree = tree!('a' => { 'b', 'c', 'd' });

    assert_eq!(
        vec!['c', 'b'],
        tree.root()
            .last_child()
            .unwrap()
            .prev_siblings()
            .filter_map(|nref| nref.map_value(|&c| c))
            .collect::<Vec<_>>()
    );
}

#[test]
fn iter_next_siblings() {
    let tree = tree!('a' => { 'b', 'c', 'd' });

    assert_eq!(
        vec!['c', 'd'],
        tree.root()
            .first_child()
            .unwrap()
            .next_siblings()
            .filter_map(|nref| nref.map_value(|&c| c))
            .collect::<Vec<_>>()
    );
}

#[test]
fn iter_children() {
    let tree = tree!('a' => { 'b', 'c', 'd' });

    assert_eq!(
        vec!['b', 'c', 'd'],
        tree.root()
            .children()
            .filter_map(|nref| nref.map_value(|&c| c))
            .collect::<Vec<_>>()
    );
}

#[test]
fn iter_children_rev() {
    let tree = tree!('a' => { 'b', 'c', 'd' });

    assert_eq!(
        vec!['d', 'c', 'b'],
        tree.root()
            .children()
            .rev()
            .filter_map(|nref| nref.map_value(|&c| c))
            .collect::<Vec<_>>()
    );
}

#[test]
fn iter_first_children() {
    let tree = tree!('a' => { 'b' => { 'd', 'e' }, 'c' });

    assert_eq!(
        vec!['b', 'd'],
        tree.root()
            .first_children()
            .filter_map(|nref| nref.map_value(|&c| c))
            .collect::<Vec<_>>()
    );
}

#[test]
fn iter_last_children() {
    let tree = tree!('a' => { 'b', 'c' => { 'd', 'e' } });

    assert_eq!(
        vec!['c', 'e'],
        tree.root()
            .last_children()
            .filter_map(|nref| nref.map_value(|&c| c))
            .collect::<Vec<_>>()
    );
}

#[test]
fn iter_traverse() {
    #[derive(Debug, PartialEq, Eq)]
    enum Value {
        Open(char),
        Close(char),
    }

    let tree = tree!('a' => { 'b' => { 'd', 'e' }, 'c' });

    let traversal = tree
        .root()
        .traverse()
        .filter_map(|edge| match edge {
            Edge::Open(nref) => nref.map_value(|&c| c).map(|c| Value::Open(c)),
            Edge::Close(nref) => nref.map_value(|&c| c).map(|c| Value::Close(c)),
        })
        .collect::<Vec<_>>();

    assert_eq!(
        &[
            Value::Open('a'),
            Value::Open('b'),
            Value::Open('d'),
            Value::Close('d'),
            Value::Open('e'),
            Value::Close('e'),
            Value::Close('b'),
            Value::Open('c'),
            Value::Close('c'),
            Value::Close('a'),
        ],
        &traversal[..]
    );
}

#[test]
fn iter_descendants() {
    let tree = tree!('a' => { 'b' => { 'd', 'e' }, 'c' });

    let descendants = tree
        .root()
        .descendants()
        .filter_map(|nref| nref.map_value(|&c| c))
        .collect::<Vec<_>>();

    assert_eq!(&['a', 'b', 'd', 'e', 'c',], &descendants[..]);
}
