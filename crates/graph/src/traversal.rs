use std::collections::{HashMap, HashSet, VecDeque};

use crate::graph::Graph;
use crate::id::NodeId;

/// DFS visit order.
pub fn dfs<N, E>(graph: &Graph<N, E>, start: NodeId) -> Vec<NodeId> {
    let mut visited = HashSet::new();
    let mut order = Vec::new();
    dfs_visit(graph, start, &mut visited, &mut order);
    order
}

fn dfs_visit<N, E>(
    graph: &Graph<N, E>,
    node: NodeId,
    visited: &mut HashSet<NodeId>,
    order: &mut Vec<NodeId>,
) {
    if !visited.insert(node) {
        return;
    }
    order.push(node);
    for succ in graph.successors(node) {
        dfs_visit(graph, succ, visited, order);
    }
}

/// DFS over all nodes (handles disconnected components).
pub fn dfs_all<N, E>(graph: &Graph<N, E>) -> Vec<NodeId> {
    let mut visited = HashSet::new();
    let mut order = Vec::new();
    for node in graph.node_ids() {
        if !visited.contains(&node) {
            dfs_visit(graph, node, &mut visited, &mut order);
        }
    }
    order
}

/// BFS visit order.
pub fn bfs<N, E>(graph: &Graph<N, E>, start: NodeId) -> Vec<NodeId> {
    let mut visited = HashSet::new();
    let mut queue = VecDeque::new();
    let mut order = Vec::new();

    visited.insert(start);
    queue.push_back(start);

    while let Some(node) = queue.pop_front() {
        order.push(node);
        for succ in graph.successors(node) {
            if visited.insert(succ) {
                queue.push_back(succ);
            }
        }
    }
    order
}

/// Topological sort using Kahn's algorithm.
/// Returns None if the graph has a cycle.
pub fn topo_sort<N, E>(graph: &Graph<N, E>) -> Option<Vec<NodeId>> {
    let mut in_deg: HashMap<NodeId, usize> = HashMap::new();
    for node in graph.node_ids() {
        in_deg.insert(node, graph.in_degree(node));
    }

    let mut queue: VecDeque<NodeId> = in_deg
        .iter()
        .filter(|(_, deg)| **deg == 0)
        .map(|(id, _)| *id)
        .collect();

    let mut order = Vec::with_capacity(graph.node_count());

    while let Some(node) = queue.pop_front() {
        order.push(node);
        for succ in graph.successors(node) {
            if let Some(deg) = in_deg.get_mut(&succ) {
                *deg -= 1;
                if *deg == 0 {
                    queue.push_back(succ);
                }
            }
        }
    }

    if order.len() == graph.node_count() {
        Some(order)
    } else {
        None // cycle detected
    }
}

/// Post-order DFS (children visited before parent).
pub fn postorder<N, E>(graph: &Graph<N, E>, start: NodeId) -> Vec<NodeId> {
    let mut visited = HashSet::new();
    let mut order = Vec::new();
    postorder_visit(graph, start, &mut visited, &mut order);
    order
}

fn postorder_visit<N, E>(
    graph: &Graph<N, E>,
    node: NodeId,
    visited: &mut HashSet<NodeId>,
    order: &mut Vec<NodeId>,
) {
    if !visited.insert(node) {
        return;
    }
    for succ in graph.successors(node) {
        postorder_visit(graph, succ, visited, order);
    }
    order.push(node);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn diamond_graph() -> (Graph<&'static str, ()>, NodeId, NodeId, NodeId, NodeId) {
        let mut g = Graph::new();
        let a = g.add_node("A");
        let b = g.add_node("B");
        let c = g.add_node("C");
        let d = g.add_node("D");
        g.add_edge(a, b, ());
        g.add_edge(a, c, ());
        g.add_edge(b, d, ());
        g.add_edge(c, d, ());
        (g, a, b, c, d)
    }

    #[test]
    fn dfs_visits_all_reachable() {
        let (g, a, b, c, d) = diamond_graph();
        let order = dfs(&g, a);
        assert_eq!(order.len(), 4);
        assert_eq!(order[0], a);
        assert!(order.contains(&b));
        assert!(order.contains(&c));
        assert!(order.contains(&d));
    }

    #[test]
    fn dfs_from_leaf() {
        let (g, _, _, _, d) = diamond_graph();
        let order = dfs(&g, d);
        assert_eq!(order, vec![d]); // d has no successors
    }

    #[test]
    fn bfs_visits_all_reachable() {
        let (g, a, b, c, d) = diamond_graph();
        let order = bfs(&g, a);
        assert_eq!(order.len(), 4);
        assert_eq!(order[0], a);
        // b and c should come before d (BFS layer property)
        let pos = |n: NodeId| order.iter().position(|&x| x == n).unwrap();
        assert!(pos(b) < pos(d));
        assert!(pos(c) < pos(d));
    }

    #[test]
    fn topo_sort_dag() {
        let (g, a, b, c, d) = diamond_graph();
        let order = topo_sort(&g).unwrap();
        assert_eq!(order.len(), 4);
        let pos = |n: NodeId| order.iter().position(|&x| x == n).unwrap();
        assert!(pos(a) < pos(b));
        assert!(pos(a) < pos(c));
        assert!(pos(b) < pos(d));
        assert!(pos(c) < pos(d));
    }

    #[test]
    fn topo_sort_detects_cycle() {
        let mut g = Graph::new();
        let a = g.add_node("A");
        let b = g.add_node("B");
        let c = g.add_node("C");
        g.add_edge(a, b, ());
        g.add_edge(b, c, ());
        g.add_edge(c, a, ());

        assert!(topo_sort(&g).is_none());
    }

    #[test]
    fn postorder_children_before_parent() {
        let (g, a, b, c, d) = diamond_graph();
        let order = postorder(&g, a);
        assert_eq!(order.len(), 4);
        let pos = |n: NodeId| order.iter().position(|&x| x == n).unwrap();
        assert!(pos(d) < pos(b) || pos(d) < pos(c));
        assert!(pos(a) == order.len() - 1); // root is last
    }

    #[test]
    fn dfs_all_disconnected() {
        let mut g: Graph<&str, ()> = Graph::new();
        let a = g.add_node("A");
        let b = g.add_node("B");
        let c = g.add_node("C");
        // No edges — three disconnected components
        let order = dfs_all(&g);
        assert_eq!(order.len(), 3);
        let set: HashSet<_> = order.into_iter().collect();
        assert!(set.contains(&a));
        assert!(set.contains(&b));
        assert!(set.contains(&c));
    }

    #[test]
    fn topo_sort_linear() {
        let mut g = Graph::new();
        let a = g.add_node("A");
        let b = g.add_node("B");
        let c = g.add_node("C");
        g.add_edge(a, b, ());
        g.add_edge(b, c, ());

        let order = topo_sort(&g).unwrap();
        assert_eq!(order, vec![a, b, c]);
    }

    #[test]
    fn topo_sort_single_node() {
        let mut g: Graph<&str, ()> = Graph::new();
        let a = g.add_node("A");
        let order = topo_sort(&g).unwrap();
        assert_eq!(order, vec![a]);
    }

    #[test]
    fn bfs_single_node() {
        let mut g: Graph<&str, ()> = Graph::new();
        let a = g.add_node("A");
        assert_eq!(bfs(&g, a), vec![a]);
    }
}
