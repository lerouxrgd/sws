use sws_tree::tree;

#[test]
fn node_value() {
    let tree = tree!('a');
    assert_eq!('a', tree.root().map_value(|&c| c).unwrap());
}

#[test]
fn node_parent() {
    let tree = tree!('a' => { 'b' });
    let b = tree.root().first_child().unwrap();
    assert_eq!(tree.root(), b.parent().unwrap());
}

#[test]
fn node_prev_sibling() {
    let tree = tree!('a' => { 'b', 'c' });
    let c = tree.root().last_child().unwrap();
    assert_eq!(tree.root().first_child(), c.prev_sibling());
}

#[test]
fn node_next_sibling() {
    let tree = tree!('a' => { 'b', 'c' });
    let b = tree.root().first_child().unwrap();
    assert_eq!(tree.root().last_child(), b.next_sibling());
}

#[test]
fn node_first_child() {
    let tree = tree!('a' => { 'b', 'c' });
    assert_eq!(
        'b',
        tree.root()
            .first_child()
            .unwrap()
            .map_value(|&c| c)
            .unwrap()
    );
}

#[test]
fn node_last_child() {
    let tree = tree!('a' => { 'b', 'c' });
    assert_eq!(
        'c',
        tree.root().last_child().unwrap().map_value(|&c| c).unwrap()
    );
}

#[test]
fn node_has_siblings() {
    let tree = tree!('a' => { 'b', 'c' });
    assert_eq!(false, tree.root().has_siblings());
    assert_eq!(true, tree.root().first_child().unwrap().has_siblings());
}

#[test]
fn node_has_children() {
    let tree = tree!('a' => { 'b', 'c' });
    assert_eq!(true, tree.root().has_children());
    assert_eq!(false, tree.root().first_child().unwrap().has_children());
}

////////////////////////////////////////////////////////////////////////////////////////

#[test]
fn append_1() {
    let tree = tree!('a');
    tree.root().append('b');

    let root = tree.root();
    let child = root.first_child().unwrap();

    assert_eq!('b', child.map_value(|&c| c).unwrap());
    assert_eq!(Some(child.clone()), root.last_child());
    assert_eq!(Some(root.clone()), child.parent());
    assert_eq!(None, child.next_sibling());
    assert_eq!(None, child.next_sibling());
}

#[test]
fn append_2() {
    let tree = tree!('a');
    tree.root().append('b');
    tree.root().append('c');

    let root = tree.root();
    let b = root.first_child().unwrap();
    let c = root.last_child().unwrap();

    assert_eq!('b', b.map_value(|&c| c).unwrap());
    assert_eq!('c', c.map_value(|&c| c).unwrap());
    assert_eq!(Some(root.clone()), b.parent());
    assert_eq!(Some(root.clone()), c.parent());
    assert_eq!(None, b.prev_sibling());
    assert_eq!(Some(c.clone()), b.next_sibling());
    assert_eq!(Some(b.clone()), c.prev_sibling());
    assert_eq!(None, c.next_sibling());
}

#[test]
fn append_3() {
    let tree = tree!('a');
    tree.root().append('b');
    tree.root().append('c');
    tree.root().append('d');

    let root = tree.root();
    let b = root.first_child().unwrap();
    let c = b.next_sibling().unwrap();
    let d = root.last_child().unwrap();

    assert_eq!('b', b.map_value(|&c| c).unwrap());
    assert_eq!('c', c.map_value(|&c| c).unwrap());
    assert_eq!('d', d.map_value(|&c| c).unwrap());
    assert_eq!(Some(root.clone()), b.parent());
    assert_eq!(Some(root.clone()), c.parent());
    assert_eq!(Some(root.clone()), d.parent());
    assert_eq!(None, b.prev_sibling());
    assert_eq!(Some(c.clone()), b.next_sibling());
    assert_eq!(Some(b.clone()), c.prev_sibling());
    assert_eq!(Some(d.clone()), c.next_sibling());
    assert_eq!(Some(c.clone()), d.prev_sibling());
    assert_eq!(None, d.next_sibling());
}

#[test]
fn prepend_1() {
    let tree = tree!('a');
    tree.root().prepend('b');

    let root = tree.root();
    let child = root.first_child().unwrap();

    assert_eq!('b', child.map_value(|&c| c).unwrap());
    assert_eq!(Some(child.clone()), root.last_child());
    assert_eq!(Some(root.clone()), child.parent());
    assert_eq!(None, child.next_sibling());
    assert_eq!(None, child.next_sibling());
}

#[test]
fn prepend_2() {
    let tree = tree!('a');
    tree.root().prepend('c');
    tree.root().prepend('b');

    let root = tree.root();
    let b = root.first_child().unwrap();
    let c = root.last_child().unwrap();

    assert_eq!('b', b.map_value(|&c| c).unwrap());
    assert_eq!('c', c.map_value(|&c| c).unwrap());
    assert_eq!(Some(root.clone()), b.parent());
    assert_eq!(Some(root.clone()), c.parent());
    assert_eq!(None, b.prev_sibling());
    assert_eq!(Some(c.clone()), b.next_sibling());
    assert_eq!(Some(b.clone()), c.prev_sibling());
    assert_eq!(None, c.next_sibling());
}

#[test]
fn prepend_3() {
    let tree = tree!('a');
    tree.root().prepend('d');
    tree.root().prepend('c');
    tree.root().prepend('b');

    let root = tree.root();
    let b = root.first_child().unwrap();
    let c = b.next_sibling().unwrap();
    let d = root.last_child().unwrap();

    assert_eq!('b', b.map_value(|&c| c).unwrap());
    assert_eq!('c', c.map_value(|&c| c).unwrap());
    assert_eq!('d', d.map_value(|&c| c).unwrap());
    assert_eq!(Some(root.clone()), b.parent());
    assert_eq!(Some(root.clone()), c.parent());
    assert_eq!(Some(root.clone()), d.parent());
    assert_eq!(None, b.prev_sibling());
    assert_eq!(Some(c.clone()), b.next_sibling());
    assert_eq!(Some(b.clone()), c.prev_sibling());
    assert_eq!(Some(d.clone()), c.next_sibling());
    assert_eq!(Some(c.clone()), d.prev_sibling());
    assert_eq!(None, d.next_sibling());
}

#[test]
fn insert_before_first() {
    let tree = tree!('a' => { 'c' });
    tree.root().first_child().unwrap().insert_before('b');

    let root = tree.root();
    let b = root.first_child().unwrap();
    let c = root.last_child().unwrap();

    assert_eq!('b', b.map_value(|&c| c).unwrap());
    assert_eq!(Some(root), b.parent());
    assert_eq!(None, b.prev_sibling());
    assert_eq!(Some(c.clone()), b.next_sibling());
    assert_eq!(Some(b.clone()), c.prev_sibling());
    assert_eq!(None, c.next_sibling());
}

#[test]
fn insert_before() {
    let tree = tree!('a' => { 'b', 'd' });
    tree.root().last_child().unwrap().insert_before('c');

    let root = tree.root();
    let b = root.first_child().unwrap();
    let c = b.next_sibling().unwrap();
    let d = root.last_child().unwrap();

    assert_eq!('c', c.map_value(|&c| c).unwrap());
    assert_eq!(Some(root.clone()), b.parent());
    assert_eq!(Some(root.clone()), c.parent());
    assert_eq!(Some(root.clone()), d.parent());
    assert_eq!(None, b.prev_sibling());
    assert_eq!(Some(c.clone()), b.next_sibling());
    assert_eq!(Some(b.clone()), c.prev_sibling());
    assert_eq!(Some(d.clone()), c.next_sibling());
    assert_eq!(Some(c.clone()), d.prev_sibling());
    assert_eq!(None, d.next_sibling());
}

#[test]
fn insert_after_first() {
    let tree = tree!('a' => { 'b' });
    tree.root().first_child().unwrap().insert_after('c');

    let root = tree.root();
    let b = root.first_child().unwrap();
    let c = root.last_child().unwrap();

    assert_eq!('c', c.map_value(|&c| c).unwrap());
    assert_eq!(Some(root), c.parent());
    assert_eq!(None, b.prev_sibling());
    assert_eq!(Some(c.clone()), b.next_sibling());
    assert_eq!(Some(b.clone()), c.prev_sibling());
    assert_eq!(None, c.next_sibling());
}

#[test]
fn insert_after() {
    let tree = tree!('a' => { 'b', 'd' });
    tree.root().first_child().unwrap().insert_after('c');

    let root = tree.root();
    let b = root.first_child().unwrap();
    let c = b.next_sibling().unwrap();
    let d = root.last_child().unwrap();

    assert_eq!('c', c.map_value(|&c| c).unwrap());
    assert_eq!(Some(root.clone()), b.parent());
    assert_eq!(Some(root.clone()), c.parent());
    assert_eq!(Some(root.clone()), d.parent());
    assert_eq!(None, b.prev_sibling());
    assert_eq!(Some(c.clone()), b.next_sibling());
    assert_eq!(Some(b.clone()), c.prev_sibling());
    assert_eq!(Some(d.clone()), c.next_sibling());
    assert_eq!(Some(c.clone()), d.prev_sibling());
    assert_eq!(None, d.next_sibling());
}

#[test]
fn detach() {
    let tree = tree!('a' => { 'b', 'd' });
    let root = tree.root();
    let mut b = root.first_child().unwrap();
    let mut c = b.insert_after('c').unwrap();
    c.detach();

    assert!(c.parent().is_none());
    assert!(c.prev_sibling().is_none());
    assert!(c.next_sibling().is_none());
}

#[test]
fn reparent_from_id_append() {
    let tree = tree! {
        'a' => {
            'b' => { 'c', 'd' },
            'e' => { 'f', 'g' },
        }
    };
    let e_id = tree.root().last_child().unwrap().id();
    tree.root()
        .first_child()
        .unwrap()
        .reparent_from_id_append(e_id);

    let b = tree.root().first_child().unwrap();
    let e = tree.root().last_child().unwrap();
    let d = b.first_child().unwrap().next_sibling().unwrap();
    let g = b.last_child().unwrap();
    let f = g.prev_sibling().unwrap();

    assert_eq!(false, e.has_children());
    assert_eq!('f', f.map_value(|&c| c).unwrap());
    assert_eq!('g', g.map_value(|&c| c).unwrap());
    assert_eq!(Some(&f), d.next_sibling().as_ref());
    assert_eq!(Some(&d), f.prev_sibling().as_ref());
}
