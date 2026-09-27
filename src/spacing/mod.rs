use std::fmt::Debug;

use ahash::AHashMap as HashMap;
use smithay::{
    backend::renderer::element::Kind,
    desktop::{Space, Window, space::SpaceElement},
    utils::{Logical, Rectangle},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NodeId(u32);

#[derive(Debug, Default)]
pub struct Workspace<T: std::fmt::Debug> {
    nodes: HashMap<NodeId, Node<T>>,
    root: Option<NodeId>,
    next_id: u32,
}

impl<T: Debug + PartialEq + Eq> Workspace<T> {
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            root: None,
            next_id: 0,
        }
    }

    fn alloc_id(&mut self) -> NodeId {
        let id = NodeId(self.next_id);
        self.next_id += 1;
        id
    }

    // TODO workspace should own creating new nodes because of node_id coupling
    // should take parent node as argument, handle splitting
    // do not mess with the nodeid -> data invariant
    // we'd traverse to the parent, replace with a new split node, and set the left to the caller
    // and the right to the new node
    // if you somehow call on a split, then the reallocation is a similar process

    fn find_node_id(&self, target: &T) -> Option<NodeId> {
        self.nodes
            .iter()
            .find(|(_, item)| {
                if let InnerNode::Leaf(leaf) = &item.inner {
                    leaf.data == *target
                } else {
                    false
                }
            })
            .map(|(node_id, _)| node_id.clone())
    }

    // TODO don't let this fail silently - return a result
    pub fn add_node(&mut self, parent: Option<&T>, data: T) {
        if self.root.is_none() && parent.is_none() {
            self.add_node_inner(None, data);
        } else if let Some(inner_parent) = parent
            && let Some(node_id) = self.find_node_id(inner_parent)
        {
            self.add_node_inner(Some(node_id), data);
        }
    }

    fn add_node_inner(&mut self, parent: Option<NodeId>, data: T) {
        let new_id = self.alloc_id();
        let mut new_node = Node {
            inner: InnerNode::Leaf(Leaf { data }),
            id: new_id,
            parent: None,
        };
        if self.root.is_none() {
            self.root = Some(new_id);
            self.nodes.insert(new_id, new_node);

            if parent.is_some() {
                panic!("How are you passing in a parent when workspace root is None!?");
            }
            return;
        }

        let parent_node_id =
            parent.expect("Expected a valid parent root NodeId for new element, got None");
        let new_parent_node_id = self.alloc_id();
        let parent_node = &mut self
            .nodes
            .get(&parent_node_id)
            .expect("Data for NodeId unexpectedly missing!");

        match &parent_node.inner {
            InnerNode::Leaf(_leaf) => {
                // 1. mint a new node thats a split, it will hold the former parent and new id
                let new_parent_node = Node {
                    id: new_parent_node_id,
                    inner: InnerNode::<T>::Split(Split {
                        axis: Axis::Vertical,
                        split_ratio: 0.5,
                        left: parent_node_id,
                        right: new_id,
                    }),
                    parent: parent_node.parent,
                };

                // 2. now find the former parent's parent
                if let Some(grandparent_node_id) = parent_node.parent {
                    let grandparent_node = self
                        .nodes
                        .get_mut(&grandparent_node_id)
                        .expect("Expected a valid grandparent root NodeId!");

                    if let InnerNode::Split(grandparent_split) = &mut grandparent_node.inner {
                        if grandparent_split.left == parent_node_id {
                            grandparent_split.left = new_parent_node_id;
                        } else if grandparent_split.right == parent_node_id {
                            grandparent_split.right = new_parent_node_id;
                        }
                    }
                } else {
                    self.root = Some(new_parent_node_id)
                }

                // 3. link both children to the new split, then insert into our hashmap
                self.nodes.get_mut(&parent_node_id).unwrap().parent = Some(new_parent_node_id);
                new_node.parent = Some(new_parent_node_id);
                self.nodes.insert(new_parent_node_id, new_parent_node);
                self.nodes.insert(new_id, new_node);
            }
            InnerNode::Split(_split) => {
                panic!("Not implemented yet!")
            }
        }
    }

    pub fn remove_node(&mut self, target: NodeId) {}

    pub fn get_data(&self, id: NodeId) -> Option<&T> {
        if let Some(node) = self.nodes.get(&id) {
            match &node.inner {
                InnerNode::Leaf(leaf) => Some(&leaf.data),
                InnerNode::Split(_) => None,
            }
        } else {
            None
        }
    }
}

impl<T: Debug + SpaceElement + PartialEq + Eq + Clone> Workspace<T> {
    /*
     * Convert entire workspace into series of
     *
     */
    fn layout(&self, area: Rectangle<i32, Logical>) -> Vec<(Rectangle<i32, Logical>, NodeId)> {
        let mut out: Vec<(Rectangle<i32, Logical>, NodeId)> = Vec::new();
        if let Some(root) = self.root {
            self.layout_node(root, area, &mut out);
        }
        return out;
    }

    fn layout_node(
        &self,
        parent: NodeId,
        area: Rectangle<i32, Logical>,
        out_vec: &mut Vec<(Rectangle<i32, Logical>, NodeId)>,
    ) {
        let parent_node = self
            .nodes
            .get(&parent)
            .expect("Could not retrieve layout node!");
        match &parent_node.inner {
            InnerNode::Leaf(_) => {
                out_vec.push((area, parent));
            }
            InnerNode::Split(split) => {
                let (left, right): (Rectangle<i32, Logical>, Rectangle<i32, Logical>) =
                    match split.axis {
                        Axis::Horizontal => {
                            let top_h = (area.size.h as f64 * split.split_ratio) as i32;
                            let bottom_h = area.size.h - top_h;
                            (
                                Rectangle::new(area.loc, (area.size.w, top_h).into()),
                                Rectangle::new(
                                    (area.loc.x, area.loc.y + top_h).into(),
                                    (area.size.w, bottom_h).into(),
                                ),
                            )
                        }
                        Axis::Vertical => {
                            let left_w = (area.size.w as f64 * split.split_ratio) as i32;
                            let right_w = area.size.w - left_w;
                            (
                                Rectangle::new(area.loc, (right_w, area.size.h).into()),
                                Rectangle::new(
                                    (area.loc.x + left_w, area.loc.y).into(),
                                    (right_w, area.size.h).into(),
                                ),
                            )
                        }
                    };
                self.layout_node(split.left, left, out_vec);
                self.layout_node(split.right, right, out_vec);
            }
        }
    }
}

impl Workspace<Window> {
    pub fn apply_layout(&self, space: &mut Space<Window>) {
        let output = space.outputs().next().unwrap();
        let area = space.output_geometry(&output).unwrap();
        let elements = self.layout(area);
        for (rec, node_id) in elements {
            let element = (self.get_data(node_id).unwrap()).clone();
            let top_lvl = element.toplevel().unwrap();
            top_lvl.with_pending_state(|s| s.size = Some(rec.size));
            top_lvl.send_pending_configure();
            space.map_element(element, rec.loc, false);
        }
    }
}

#[derive(Debug)]
pub struct Leaf<T: Debug> {
    data: T,
}

#[derive(Debug)]
pub enum Axis {
    Horizontal,
    Vertical,
}

#[derive(Debug)]
pub struct Split {
    axis: Axis,
    split_ratio: f64,
    left: NodeId,
    right: NodeId,
}

#[derive(Debug)]
pub struct Node<T: std::fmt::Debug> {
    id: NodeId,
    inner: InnerNode<T>,
    parent: Option<NodeId>,
}

#[derive(Debug)]
pub enum InnerNode<T: std::fmt::Debug> {
    Leaf(Leaf<T>),
    Split(Split),
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    fn split<'a>(workspace: &'a Workspace<&str>, id: NodeId) -> &'a Split {
        match &workspace.nodes[&id].inner {
            InnerNode::Split(split) => split,
            InnerNode::Leaf(_) => panic!("Expected a split at {id:?}"),
        }
    }

    fn assert_valid_tree(workspace: &Workspace<&str>, leaf_count: usize) {
        let mut visited = HashSet::new();
        let mut pending = vec![(workspace.root.unwrap(), None)];
        let mut leaves = 0;

        while let Some((id, parent)) = pending.pop() {
            assert!(
                visited.insert(id),
                "Node {id:?} is reachable more than once"
            );
            let node = &workspace.nodes[&id];
            assert_eq!(node.id, id);
            assert_eq!(node.parent, parent, "Incorrect parent for {id:?}");
            match &node.inner {
                InnerNode::Leaf(_) => leaves += 1,
                InnerNode::Split(split) => {
                    assert!(matches!(split.axis, Axis::Horizontal));
                    assert_eq!(split.split_ratio, 0.5);
                    assert_eq!(workspace.get_data(id), None);
                    pending.push((split.left, Some(id)));
                    pending.push((split.right, Some(id)));
                }
            }
        }

        assert_eq!(leaves, leaf_count);
        assert_eq!(visited.len(), workspace.nodes.len(), "Unreachable nodes");
        assert_eq!(workspace.nodes.len(), 2 * leaf_count - 1);
    }

    #[test]
    fn first_insertion_creates_a_root_leaf() {
        let mut workspace = Workspace::default();

        workspace.add_node(None, "first");

        assert_eq!(workspace.get_data(workspace.root.unwrap()), Some(&"first"));
        assert_valid_tree(&workspace, 1);
    }

    #[test]
    fn inserting_at_root_preserves_leaf_id_and_data() {
        let mut workspace = Workspace::default();
        workspace.add_node(None, "first");
        let first = workspace.root.unwrap();

        workspace.add_node(Some(first), "second");

        let root = workspace.root.unwrap();
        assert_ne!(root, first);
        let children = split(&workspace, root);
        assert_eq!(children.left, first);
        assert_eq!(workspace.get_data(first), Some(&"first"));
        assert_eq!(workspace.get_data(children.right), Some(&"second"));
        assert_valid_tree(&workspace, 2);
    }

    fn assert_insertion_at_child(insert_left: bool) {
        let mut workspace = Workspace::default();
        workspace.add_node(None, "first");
        let first = workspace.root.unwrap();
        workspace.add_node(Some(first), "second");
        let root = workspace.root.unwrap();
        let children = split(&workspace, root);
        let (target, sibling) = if insert_left {
            (children.left, children.right)
        } else {
            (children.right, children.left)
        };
        let target_data = *workspace.get_data(target).unwrap();
        let sibling_data = *workspace.get_data(sibling).unwrap();

        workspace.add_node(Some(target), "third");

        assert_eq!(workspace.root, Some(root));
        let children = split(&workspace, root);
        let replacement = if insert_left {
            assert_eq!(children.right, sibling);
            children.left
        } else {
            assert_eq!(children.left, sibling);
            children.right
        };
        let new_children = split(&workspace, replacement);
        assert_eq!(new_children.left, target);
        assert_eq!(workspace.get_data(target), Some(&target_data));
        assert_eq!(workspace.get_data(sibling), Some(&sibling_data));
        assert_eq!(workspace.get_data(new_children.right), Some(&"third"));
        assert_valid_tree(&workspace, 3);
    }

    #[test]
    fn inserting_at_left_child_replaces_only_that_child() {
        assert_insertion_at_child(true);
    }

    #[test]
    fn inserting_at_right_child_replaces_only_that_child() {
        assert_insertion_at_child(false);
    }

    #[test]
    fn repeated_insertions_preserve_existing_leaf_ids_and_data() {
        let mut workspace = Workspace::default();
        workspace.add_node(None, "first");
        let mut target = workspace.root.unwrap();
        let mut expected = vec![(target, "first")];

        for data in ["second", "third", "fourth", "fifth"] {
            workspace.add_node(Some(target), data);
            let parent = workspace.nodes[&target].parent.unwrap();
            target = split(&workspace, parent).right;
            expected.push((target, data));

            for &(id, data) in &expected {
                assert_eq!(workspace.get_data(id), Some(&data));
            }
            assert_valid_tree(&workspace, expected.len());
        }
    }

    #[test]
    #[should_panic(expected = "How are you passing in a parent when workspace root is None!?")]
    fn first_insertion_rejects_a_parent() {
        let mut workspace = Workspace::default();
        workspace.add_node(Some(NodeId(42)), "first");
    }

    #[test]
    #[should_panic(expected = "Expected a valid parent root NodeId for new element, got None")]
    fn subsequent_insertion_requires_a_parent() {
        let mut workspace = Workspace::default();
        workspace.add_node(None, "first");
        workspace.add_node(None, "second");
    }

    #[test]
    #[should_panic(expected = "Data for NodeId unexpectedly missing!")]
    fn insertion_rejects_an_unknown_parent() {
        let mut workspace = Workspace::default();
        workspace.add_node(None, "first");
        workspace.add_node(Some(NodeId(42)), "second");
    }
}
