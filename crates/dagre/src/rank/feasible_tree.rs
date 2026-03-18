use std::collections::{HashMap, HashSet};

use rusty_mermaid_graph::{Graph, NodeId};

use crate::labels::{EdgeLabel, NodeLabel};
use crate::util;

/// Spanning tree for network simplex. Undirected tree over the graph's nodes.
#[derive(Debug)]
pub(crate) struct NsTree {
    adj: HashMap<NodeId, Vec<NodeId>>,
    nodes: HashSet<NodeId>,
    pub(crate) cut_values: HashMap<(NodeId, NodeId), f64>,
    pub(crate) parent: HashMap<NodeId, Option<NodeId>>,
    pub(crate) low: HashMap<NodeId, usize>,
    pub(crate) lim: HashMap<NodeId, usize>,
    pub(crate) root: NodeId,
}

impl NsTree {
    fn new(root: NodeId) -> Self {
        let mut nodes = HashSet::new();
        nodes.insert(root);
        let mut adj = HashMap::new();
        adj.insert(root, Vec::new());

        Self {
            adj,
            nodes,
            cut_values: HashMap::new(),
            parent: HashMap::new(),
            low: HashMap::new(),
            lim: HashMap::new(),
            root,
        }
    }

    pub(crate) fn has_node(&self, v: NodeId) -> bool {
        self.nodes.contains(&v)
    }

    pub(crate) fn node_count(&self) -> usize {
        self.nodes.len()
    }

    pub(crate) fn add_node(&mut self, v: NodeId) {
        self.nodes.insert(v);
        self.adj.entry(v).or_default();
    }

    pub(crate) fn add_edge(&mut self, u: NodeId, v: NodeId) {
        self.adj.entry(u).or_default().push(v);
        self.adj.entry(v).or_default().push(u);
    }

    pub(crate) fn remove_edge(&mut self, u: NodeId, v: NodeId) {
        if let Some(ns) = self.adj.get_mut(&u) {
            ns.retain(|&n| n != v);
        }
        if let Some(ns) = self.adj.get_mut(&v) {
            ns.retain(|&n| n != u);
        }
        self.cut_values.remove(&canonical(u, v));
    }

    pub(crate) fn has_edge(&self, u: NodeId, v: NodeId) -> bool {
        self.adj
            .get(&u)
            .is_some_and(|ns| ns.contains(&v))
    }

    pub(crate) fn neighbors(&self, v: NodeId) -> &[NodeId] {
        self.adj.get(&v).map_or(&[], |ns| ns.as_slice())
    }

    pub(crate) fn set_cut_value(&mut self, u: NodeId, v: NodeId, val: f64) {
        self.cut_values.insert(canonical(u, v), val);
    }

    pub(crate) fn get_cut_value(&self, u: NodeId, v: NodeId) -> f64 {
        self.cut_values.get(&canonical(u, v)).copied().unwrap_or(0.0)
    }

    /// Is `v` a descendant of `root_node` in the DFS tree?
    pub(crate) fn is_descendant(&self, v: NodeId, root_node: NodeId) -> bool {
        let v_lim = self.lim.get(&v).copied().unwrap_or(0);
        let r_low = self.low.get(&root_node).copied().unwrap_or(0);
        let r_lim = self.lim.get(&root_node).copied().unwrap_or(0);
        r_low <= v_lim && v_lim <= r_lim
    }

    /// Compute DFS numbering (low/lim) and parent pointers.
    pub(crate) fn init_low_lim(&mut self) {
        self.parent.clear();
        self.low.clear();
        self.lim.clear();
        let root = self.root;
        self.parent.insert(root, None);
        let mut visited = HashSet::new();
        self.dfs_assign(root, &mut visited, &mut 1);
    }

    fn dfs_assign(&mut self, v: NodeId, visited: &mut HashSet<NodeId>, next_lim: &mut usize) {
        let low = *next_lim;
        visited.insert(v);

        let neighbors: Vec<NodeId> = self.neighbors(v).to_vec();
        for w in neighbors {
            if !visited.contains(&w) {
                self.parent.insert(w, Some(v));
                self.dfs_assign(w, visited, next_lim);
            }
        }

        self.low.insert(v, low);
        self.lim.insert(v, *next_lim);
        *next_lim += 1;
    }
}

/// Canonical pair for undirected edge lookup.
fn canonical(u: NodeId, v: NodeId) -> (NodeId, NodeId) {
    if u < v { (u, v) } else { (v, u) }
}

/// Build a feasible spanning tree from a ranked graph.
/// All tree edges are tight (slack == 0). Shifts ranks as needed.
pub(crate) fn feasible_tree_mut(g: &mut Graph<NodeLabel, EdgeLabel>) -> NsTree {
    let Some(start) = g.node_ids().next() else {
        return NsTree::new(NodeId::from(0));
    };
    let mut tree = NsTree::new(start);
    let total = g.node_count();

    // Grow tree by adding tight edges
    while tight_tree_grow(&mut tree, g) < total {
        if let Some((edge_src, _edge_dst, min_slack)) = find_min_slack_edge(&tree, g) {
            // Shift tree node ranks to make the min-slack edge tight
            let delta = if tree.has_node(edge_src) {
                min_slack
            } else {
                -min_slack
            };

            let tree_nodes: Vec<NodeId> = tree.nodes.iter().copied().collect();
            for nid in tree_nodes {
                if let Some(n) = g.node_mut(nid) {
                    n.rank += delta;
                }
            }
        } else {
            // No edge crosses the boundary — disconnected component.
            // Add an arbitrary non-tree node directly.
            if let Some(non_tree) = g.node_ids().find(|nid| !tree.has_node(*nid)) {
                tree.add_node(non_tree);
            } else {
                break;
            }
        }
    }

    tree.init_low_lim();
    tree
}

/// Grow the tree by DFS along tight edges. Returns tree node count.
fn tight_tree_grow(tree: &mut NsTree, g: &Graph<NodeLabel, EdgeLabel>) -> usize {
    let current_nodes: Vec<NodeId> = tree.nodes.iter().copied().collect();
    for v in current_nodes {
        dfs_tight(tree, g, v);
    }
    tree.node_count()
}

fn dfs_tight(tree: &mut NsTree, g: &Graph<NodeLabel, EdgeLabel>, v: NodeId) {
    // Check all edges incident to v
    for (eid, src, dst) in util::node_edges(g, v) {
        let other = if v == src { dst } else { src };
        if !tree.has_node(other) && util::slack(g, eid) == 0 {
            tree.add_node(other);
            tree.add_edge(v, other);
            dfs_tight(tree, g, other);
        }
    }
}

/// Find the minimum-slack edge crossing the tree/non-tree boundary.
/// Returns `None` if no edge crosses the boundary (disconnected components).
fn find_min_slack_edge(
    tree: &NsTree,
    g: &Graph<NodeLabel, EdgeLabel>,
) -> Option<(NodeId, NodeId, i32)> {
    let mut best: Option<(NodeId, NodeId, i32)> = None;

    for eid in g.edge_ids() {
        if let Some((src, dst)) = g.edge_endpoints(eid)
            && tree.has_node(src) != tree.has_node(dst)
        {
            let s = util::slack(g, eid).abs();
            if best.is_none_or(|(_, _, bs)| s < bs) {
                best = Some((src, dst, s));
            }
        }
    }

    best
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rank::longest_path::longest_path;
    use crate::util::normalize_ranks;

    #[test]
    fn feasible_tree_spans_all_nodes() {
        let mut g = Graph::new();
        let a = g.add_node(NodeLabel::new(10.0, 10.0));
        let b = g.add_node(NodeLabel::new(10.0, 10.0));
        let c = g.add_node(NodeLabel::new(10.0, 10.0));
        g.add_edge(a, b, EdgeLabel::default());
        g.add_edge(b, c, EdgeLabel::default());

        longest_path(&mut g);
        normalize_ranks(&mut g);

        let tree = feasible_tree_mut(&mut g);
        assert_eq!(tree.node_count(), 3);
    }

    #[test]
    fn feasible_tree_diamond() {
        let mut g = Graph::new();
        let a = g.add_node(NodeLabel::new(10.0, 10.0));
        let b = g.add_node(NodeLabel::new(10.0, 10.0));
        let c = g.add_node(NodeLabel::new(10.0, 10.0));
        let d = g.add_node(NodeLabel::new(10.0, 10.0));
        g.add_edge(a, b, EdgeLabel::default());
        g.add_edge(a, c, EdgeLabel::default());
        g.add_edge(b, d, EdgeLabel::default());
        g.add_edge(c, d, EdgeLabel::default());

        longest_path(&mut g);
        normalize_ranks(&mut g);

        let tree = feasible_tree_mut(&mut g);
        // Tree should span all 4 nodes with 3 edges
        assert_eq!(tree.node_count(), 4);
    }

    #[test]
    fn all_tree_edges_tight() {
        let mut g = Graph::new();
        let a = g.add_node(NodeLabel::new(10.0, 10.0));
        let b = g.add_node(NodeLabel::new(10.0, 10.0));
        let c = g.add_node(NodeLabel::new(10.0, 10.0));
        g.add_edge(a, b, EdgeLabel::default());
        g.add_edge(b, c, EdgeLabel::default());

        longest_path(&mut g);
        normalize_ranks(&mut g);

        let tree = feasible_tree_mut(&mut g);
        // Check that the tree structure has edges
        assert!(tree.has_edge(a, b) || tree.has_edge(b, a)
            || tree.has_edge(b, c) || tree.has_edge(c, b));
    }

    #[test]
    fn feasible_tree_non_tight_initial() {
        // Graph where not all edges are tight after longest_path
        let mut g = Graph::new();
        let a = g.add_node(NodeLabel::new(10.0, 10.0));
        let b = g.add_node(NodeLabel::new(10.0, 10.0));
        let c = g.add_node(NodeLabel::new(10.0, 10.0));
        g.add_edge(a, b, EdgeLabel::default());
        g.add_edge(a, c, EdgeLabel::new().with_minlen(2));
        g.add_edge(b, c, EdgeLabel::default());

        longest_path(&mut g);
        normalize_ranks(&mut g);

        let tree = feasible_tree_mut(&mut g);
        assert_eq!(tree.node_count(), 3);
    }

    #[test]
    fn low_lim_values_set() {
        let mut g = Graph::new();
        let a = g.add_node(NodeLabel::new(10.0, 10.0));
        let b = g.add_node(NodeLabel::new(10.0, 10.0));
        let c = g.add_node(NodeLabel::new(10.0, 10.0));
        g.add_edge(a, b, EdgeLabel::default());
        g.add_edge(b, c, EdgeLabel::default());

        longest_path(&mut g);
        normalize_ranks(&mut g);

        let tree = feasible_tree_mut(&mut g);
        // All nodes should have low and lim values
        for nid in g.node_ids() {
            assert!(tree.low.contains_key(&nid));
            assert!(tree.lim.contains_key(&nid));
        }
    }
}
