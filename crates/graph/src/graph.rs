use std::collections::{HashMap, HashSet};

use crate::id::{EdgeId, IdGen, NodeId};

/// Edge record stored internally.
#[derive(Debug, Clone)]
struct EdgeData<E> {
    src: NodeId,
    dst: NodeId,
    label: E,
}

/// Node record stored internally.
#[derive(Debug, Clone)]
struct NodeData<N> {
    label: N,
    parent: Option<NodeId>,
    children: HashSet<NodeId>,
    in_edges: Vec<EdgeId>,
    out_edges: Vec<EdgeId>,
}

/// Directed multigraph with compound (hierarchical) node support.
///
/// - Multiple edges between the same pair of nodes are allowed.
/// - Nodes can have a parent/children hierarchy for compound graphs (subgraphs).
/// - `N` is the node label type, `E` is the edge label type.
#[derive(Debug, Clone)]
pub struct Graph<N, E> {
    nodes: HashMap<NodeId, NodeData<N>>,
    edges: HashMap<EdgeId, EdgeData<E>>,
    id_gen: IdGen,
}

impl<N, E> Graph<N, E> {
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            edges: HashMap::new(),
            id_gen: IdGen::new(),
        }
    }

    // ── Node operations ──

    pub fn add_node(&mut self, label: N) -> NodeId {
        let id = self.id_gen.next_node();
        self.nodes.insert(
            id,
            NodeData {
                label,
                parent: None,
                children: HashSet::new(),
                in_edges: Vec::new(),
                out_edges: Vec::new(),
            },
        );
        id
    }

    pub fn remove_node(&mut self, id: NodeId) -> Option<N> {
        let data = self.nodes.remove(&id)?;

        // Remove all incident edges
        let edge_ids: Vec<EdgeId> = data
            .in_edges
            .iter()
            .chain(data.out_edges.iter())
            .copied()
            .collect();
        for eid in edge_ids {
            self.remove_edge(eid);
        }

        // Detach from parent
        if let Some(pid) = data.parent
            && let Some(parent) = self.nodes.get_mut(&pid)
        {
            parent.children.remove(&id);
        }

        // Orphan children (move them to root level)
        for child_id in &data.children {
            if let Some(child) = self.nodes.get_mut(child_id) {
                child.parent = None;
            }
        }

        Some(data.label)
    }

    pub fn node(&self, id: NodeId) -> Option<&N> {
        self.nodes.get(&id).map(|d| &d.label)
    }

    pub fn node_mut(&mut self, id: NodeId) -> Option<&mut N> {
        self.nodes.get_mut(&id).map(|d| &mut d.label)
    }

    pub fn has_node(&self, id: NodeId) -> bool {
        self.nodes.contains_key(&id)
    }

    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    pub fn node_ids(&self) -> impl Iterator<Item = NodeId> + '_ {
        self.nodes.keys().copied()
    }

    // ── Edge operations ──

    pub fn add_edge(&mut self, src: NodeId, dst: NodeId, label: E) -> EdgeId {
        debug_assert!(self.has_node(src), "source node {src} not in graph");
        debug_assert!(self.has_node(dst), "destination node {dst} not in graph");

        let id = self.id_gen.next_edge();
        self.edges.insert(id, EdgeData { src, dst, label });

        if let Some(src_data) = self.nodes.get_mut(&src) {
            src_data.out_edges.push(id);
        }
        if let Some(dst_data) = self.nodes.get_mut(&dst) {
            dst_data.in_edges.push(id);
        }
        id
    }

    pub fn remove_edge(&mut self, id: EdgeId) -> Option<E> {
        let data = self.edges.remove(&id)?;

        if let Some(src_data) = self.nodes.get_mut(&data.src) {
            src_data.out_edges.retain(|&eid| eid != id);
        }
        if let Some(dst_data) = self.nodes.get_mut(&data.dst) {
            dst_data.in_edges.retain(|&eid| eid != id);
        }

        Some(data.label)
    }

    pub fn edge(&self, id: EdgeId) -> Option<&E> {
        self.edges.get(&id).map(|d| &d.label)
    }

    pub fn edge_mut(&mut self, id: EdgeId) -> Option<&mut E> {
        self.edges.get_mut(&id).map(|d| &mut d.label)
    }

    pub fn edge_endpoints(&self, id: EdgeId) -> Option<(NodeId, NodeId)> {
        self.edges.get(&id).map(|d| (d.src, d.dst))
    }

    pub fn has_edge(&self, id: EdgeId) -> bool {
        self.edges.contains_key(&id)
    }

    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }

    pub fn edge_ids(&self) -> impl Iterator<Item = EdgeId> + '_ {
        self.edges.keys().copied()
    }

    // ── Adjacency queries ──

    pub fn out_edges(&self, id: NodeId) -> impl Iterator<Item = EdgeId> + '_ {
        self.nodes
            .get(&id)
            .into_iter()
            .flat_map(|d| d.out_edges.iter().copied())
    }

    pub fn in_edges(&self, id: NodeId) -> impl Iterator<Item = EdgeId> + '_ {
        self.nodes
            .get(&id)
            .into_iter()
            .flat_map(|d| d.in_edges.iter().copied())
    }

    pub fn successors(&self, id: NodeId) -> impl Iterator<Item = NodeId> + '_ {
        self.out_edges(id)
            .filter_map(|eid| self.edges.get(&eid).map(|d| d.dst))
    }

    pub fn predecessors(&self, id: NodeId) -> impl Iterator<Item = NodeId> + '_ {
        self.in_edges(id)
            .filter_map(|eid| self.edges.get(&eid).map(|d| d.src))
    }

    pub fn in_degree(&self, id: NodeId) -> usize {
        self.nodes.get(&id).map_or(0, |d| d.in_edges.len())
    }

    pub fn out_degree(&self, id: NodeId) -> usize {
        self.nodes.get(&id).map_or(0, |d| d.out_edges.len())
    }

    /// All edges between two nodes (in either direction).
    pub fn edges_between(&self, a: NodeId, b: NodeId) -> Vec<EdgeId> {
        let mut result = Vec::new();
        for &eid in self
            .nodes
            .get(&a)
            .into_iter()
            .flat_map(|d| d.out_edges.iter())
        {
            if let Some(ed) = self.edges.get(&eid)
                && ed.dst == b
            {
                result.push(eid);
            }
        }
        for &eid in self
            .nodes
            .get(&b)
            .into_iter()
            .flat_map(|d| d.out_edges.iter())
        {
            if let Some(ed) = self.edges.get(&eid)
                && ed.dst == a
            {
                result.push(eid);
            }
        }
        result
    }

    // ── Compound (hierarchy) ──

    pub fn set_parent(&mut self, child: NodeId, parent: NodeId) {
        debug_assert!(self.has_node(child), "child node {child} not in graph");
        debug_assert!(self.has_node(parent), "parent node {parent} not in graph");
        debug_assert!(child != parent, "node cannot be its own parent");

        // Remove from previous parent
        if let Some(old_parent) = self.parent(child)
            && let Some(old_data) = self.nodes.get_mut(&old_parent)
        {
            old_data.children.remove(&child);
        }

        if let Some(child_data) = self.nodes.get_mut(&child) {
            child_data.parent = Some(parent);
        }
        if let Some(parent_data) = self.nodes.get_mut(&parent) {
            parent_data.children.insert(child);
        }
    }

    pub fn remove_parent(&mut self, child: NodeId) {
        if let Some(pid) = self.parent(child)
            && let Some(parent_data) = self.nodes.get_mut(&pid)
        {
            parent_data.children.remove(&child);
        }
        if let Some(child_data) = self.nodes.get_mut(&child) {
            child_data.parent = None;
        }
    }

    pub fn parent(&self, id: NodeId) -> Option<NodeId> {
        self.nodes.get(&id).and_then(|d| d.parent)
    }

    pub fn children(&self, id: NodeId) -> impl Iterator<Item = NodeId> + '_ {
        self.nodes
            .get(&id)
            .into_iter()
            .flat_map(|d| d.children.iter().copied())
    }

    /// Root nodes: nodes with no parent.
    pub fn roots(&self) -> impl Iterator<Item = NodeId> + '_ {
        self.nodes
            .iter()
            .filter(|(_, d)| d.parent.is_none())
            .map(|(&id, _)| id)
    }

    /// Source nodes: nodes with no incoming edges.
    pub fn sources(&self) -> impl Iterator<Item = NodeId> + '_ {
        self.nodes
            .iter()
            .filter(|(_, d)| d.in_edges.is_empty())
            .map(|(&id, _)| id)
    }

    /// Sink nodes: nodes with no outgoing edges.
    pub fn sinks(&self) -> impl Iterator<Item = NodeId> + '_ {
        self.nodes
            .iter()
            .filter(|(_, d)| d.out_edges.is_empty())
            .map(|(&id, _)| id)
    }

    /// Check if graph is empty (no nodes).
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// Reverse all edge directions in-place.
    pub fn reverse(&mut self) {
        for edge_data in self.edges.values_mut() {
            std::mem::swap(&mut edge_data.src, &mut edge_data.dst);
        }
        // Swap in_edges and out_edges for all nodes
        for node_data in self.nodes.values_mut() {
            std::mem::swap(&mut node_data.in_edges, &mut node_data.out_edges);
        }
    }
}

impl<N, E> Default for Graph<N, E> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_graph() {
        let g: Graph<&str, &str> = Graph::new();
        assert_eq!(g.node_count(), 0);
        assert_eq!(g.edge_count(), 0);
        assert!(g.is_empty());
    }

    #[test]
    fn add_and_query_nodes() {
        let mut g: Graph<&str, ()> = Graph::new();
        let a = g.add_node("A");
        let b = g.add_node("B");
        assert_eq!(g.node_count(), 2);
        assert_eq!(g.node(a), Some(&"A"));
        assert_eq!(g.node(b), Some(&"B"));
        assert!(g.has_node(a));
    }

    #[test]
    fn add_and_query_edges() {
        let mut g = Graph::new();
        let a = g.add_node("A");
        let b = g.add_node("B");
        let e = g.add_edge(a, b, "A→B");
        assert_eq!(g.edge_count(), 1);
        assert_eq!(g.edge(e), Some(&"A→B"));
        assert_eq!(g.edge_endpoints(e), Some((a, b)));
    }

    #[test]
    fn remove_node_cleans_edges() {
        let mut g = Graph::new();
        let a = g.add_node("A");
        let b = g.add_node("B");
        let c = g.add_node("C");
        g.add_edge(a, b, "AB");
        g.add_edge(b, c, "BC");
        assert_eq!(g.edge_count(), 2);

        g.remove_node(b);
        assert_eq!(g.node_count(), 2);
        assert_eq!(g.edge_count(), 0);
    }

    #[test]
    fn remove_edge() {
        let mut g = Graph::new();
        let a = g.add_node("A");
        let b = g.add_node("B");
        let e = g.add_edge(a, b, "AB");
        assert_eq!(g.remove_edge(e), Some("AB"));
        assert_eq!(g.edge_count(), 0);
        assert_eq!(g.out_degree(a), 0);
        assert_eq!(g.in_degree(b), 0);
    }

    #[test]
    fn multigraph_parallel_edges() {
        let mut g = Graph::new();
        let a = g.add_node("A");
        let b = g.add_node("B");
        let e1 = g.add_edge(a, b, "first");
        let e2 = g.add_edge(a, b, "second");
        assert_eq!(g.edge_count(), 2);
        assert_ne!(e1, e2);
        assert_eq!(g.out_degree(a), 2);
    }

    #[test]
    fn successors_and_predecessors() {
        let mut g = Graph::new();
        let a = g.add_node("A");
        let b = g.add_node("B");
        let c = g.add_node("C");
        g.add_edge(a, b, ());
        g.add_edge(a, c, ());

        let succs: HashSet<_> = g.successors(a).collect();
        assert!(succs.contains(&b));
        assert!(succs.contains(&c));

        let preds: Vec<_> = g.predecessors(b).collect();
        assert_eq!(preds, vec![a]);
    }

    #[test]
    fn in_and_out_degree() {
        let mut g = Graph::new();
        let a = g.add_node(());
        let b = g.add_node(());
        let c = g.add_node(());
        g.add_edge(a, b, ());
        g.add_edge(a, c, ());
        g.add_edge(b, c, ());

        assert_eq!(g.out_degree(a), 2);
        assert_eq!(g.in_degree(a), 0);
        assert_eq!(g.in_degree(c), 2);
    }

    #[test]
    fn compound_parent_child() {
        let mut g: Graph<&str, ()> = Graph::new();
        let parent = g.add_node("parent");
        let child1 = g.add_node("child1");
        let child2 = g.add_node("child2");

        g.set_parent(child1, parent);
        g.set_parent(child2, parent);

        assert_eq!(g.parent(child1), Some(parent));
        assert_eq!(g.parent(child2), Some(parent));
        let children: HashSet<_> = g.children(parent).collect();
        assert_eq!(children.len(), 2);
        assert!(children.contains(&child1));
        assert!(children.contains(&child2));
    }

    #[test]
    fn remove_parent() {
        let mut g: Graph<&str, ()> = Graph::new();
        let parent = g.add_node("parent");
        let child = g.add_node("child");

        g.set_parent(child, parent);
        assert_eq!(g.parent(child), Some(parent));

        g.remove_parent(child);
        assert_eq!(g.parent(child), None);
        assert_eq!(g.children(parent).count(), 0);
    }

    #[test]
    fn reparent_moves_child() {
        let mut g: Graph<&str, ()> = Graph::new();
        let p1 = g.add_node("p1");
        let p2 = g.add_node("p2");
        let child = g.add_node("child");

        g.set_parent(child, p1);
        assert_eq!(g.children(p1).count(), 1);

        g.set_parent(child, p2);
        assert_eq!(g.parent(child), Some(p2));
        assert_eq!(g.children(p1).count(), 0);
        assert_eq!(g.children(p2).count(), 1);
    }

    #[test]
    fn remove_compound_node_orphans_children() {
        let mut g: Graph<&str, ()> = Graph::new();
        let parent = g.add_node("parent");
        let child = g.add_node("child");
        g.set_parent(child, parent);

        g.remove_node(parent);
        assert_eq!(g.parent(child), None);
    }

    #[test]
    fn roots_and_sources_and_sinks() {
        let mut g = Graph::new();
        let a = g.add_node("A");
        let b = g.add_node("B");
        let c = g.add_node("C");
        g.add_edge(a, b, ());

        let roots: HashSet<_> = g.roots().collect();
        assert_eq!(roots.len(), 3); // all are roots (no hierarchy)

        let sources: HashSet<_> = g.sources().collect();
        assert!(sources.contains(&a));
        assert!(sources.contains(&c));
        assert!(!sources.contains(&b));

        let sinks: HashSet<_> = g.sinks().collect();
        assert!(sinks.contains(&b));
        assert!(sinks.contains(&c));
        assert!(!sinks.contains(&a));
    }

    #[test]
    fn edges_between() {
        let mut g = Graph::new();
        let a = g.add_node("A");
        let b = g.add_node("B");
        g.add_edge(a, b, "fwd");
        g.add_edge(b, a, "bwd");

        let between = g.edges_between(a, b);
        assert_eq!(between.len(), 2);
    }

    #[test]
    fn node_mut_update() {
        let mut g: Graph<i32, ()> = Graph::new();
        let a = g.add_node(0);
        *g.node_mut(a).unwrap() = 42;
        assert_eq!(g.node(a), Some(&42));
    }

    #[test]
    fn node_ids_iteration() {
        let mut g: Graph<&str, ()> = Graph::new();
        let a = g.add_node("A");
        let b = g.add_node("B");
        let ids: HashSet<_> = g.node_ids().collect();
        assert!(ids.contains(&a));
        assert!(ids.contains(&b));
    }

    #[test]
    fn reverse_graph() {
        let mut g = Graph::new();
        let a = g.add_node("A");
        let b = g.add_node("B");
        g.add_edge(a, b, "fwd");

        assert_eq!(g.out_degree(a), 1);
        assert_eq!(g.in_degree(a), 0);

        g.reverse();

        assert_eq!(g.out_degree(a), 0);
        assert_eq!(g.in_degree(a), 1);
        assert_eq!(g.out_degree(b), 1);
        assert_eq!(g.in_degree(b), 0);
    }

    #[test]
    fn self_loop() {
        let mut g = Graph::new();
        let a = g.add_node("A");
        let e = g.add_edge(a, a, "loop");
        assert_eq!(g.edge_endpoints(e), Some((a, a)));
        assert_eq!(g.out_degree(a), 1);
        assert_eq!(g.in_degree(a), 1);
    }
}
