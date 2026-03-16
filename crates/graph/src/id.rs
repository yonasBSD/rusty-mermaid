use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};

/// Opaque node identifier. Cheap to copy and compare.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct NodeId(u64);

/// Opaque edge identifier. Cheap to copy and compare.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct EdgeId(u64);

impl NodeId {
    pub fn raw(self) -> u64 {
        self.0
    }
}

impl EdgeId {
    pub fn raw(self) -> u64 {
        self.0
    }
}

impl fmt::Display for NodeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "n{}", self.0)
    }
}

impl fmt::Display for EdgeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "e{}", self.0)
    }
}

/// Thread-safe monotonic ID generator.
/// Produces unique NodeId and EdgeId values.
#[derive(Debug)]
pub struct IdGen {
    next_node: AtomicU64,
    next_edge: AtomicU64,
}

impl IdGen {
    pub fn new() -> Self {
        Self {
            next_node: AtomicU64::new(0),
            next_edge: AtomicU64::new(0),
        }
    }

    pub fn next_node(&self) -> NodeId {
        NodeId(self.next_node.fetch_add(1, Ordering::Relaxed))
    }

    pub fn next_edge(&self) -> EdgeId {
        EdgeId(self.next_edge.fetch_add(1, Ordering::Relaxed))
    }
}

impl Clone for IdGen {
    fn clone(&self) -> Self {
        Self {
            next_node: AtomicU64::new(self.next_node.load(Ordering::Relaxed)),
            next_edge: AtomicU64::new(self.next_edge.load(Ordering::Relaxed)),
        }
    }
}

impl Default for IdGen {
    fn default() -> Self {
        Self::new()
    }
}

/// Create a NodeId from a raw value. Useful for tests and deserialization.
impl From<u64> for NodeId {
    fn from(v: u64) -> Self {
        Self(v)
    }
}

/// Create an EdgeId from a raw value. Useful for tests and deserialization.
impl From<u64> for EdgeId {
    fn from(v: u64) -> Self {
        Self(v)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn node_id_equality_and_hash() {
        let a = NodeId::from(1);
        let b = NodeId::from(1);
        let c = NodeId::from(2);
        assert_eq!(a, b);
        assert_ne!(a, c);

        let mut set = std::collections::HashSet::new();
        set.insert(a);
        set.insert(b);
        assert_eq!(set.len(), 1);
    }

    #[test]
    fn edge_id_equality() {
        let a = EdgeId::from(0);
        let b = EdgeId::from(0);
        assert_eq!(a, b);
    }

    #[test]
    fn id_gen_monotonic() {
        let ids = IdGen::new();
        let n0 = ids.next_node();
        let n1 = ids.next_node();
        let n2 = ids.next_node();
        assert_eq!(n0.raw(), 0);
        assert_eq!(n1.raw(), 1);
        assert_eq!(n2.raw(), 2);
    }

    #[test]
    fn id_gen_node_and_edge_independent() {
        let ids = IdGen::new();
        let n0 = ids.next_node();
        let e0 = ids.next_edge();
        let n1 = ids.next_node();
        let e1 = ids.next_edge();
        assert_eq!(n0.raw(), 0);
        assert_eq!(e0.raw(), 0);
        assert_eq!(n1.raw(), 1);
        assert_eq!(e1.raw(), 1);
    }

    #[test]
    fn display_formatting() {
        assert_eq!(format!("{}", NodeId::from(42)), "n42");
        assert_eq!(format!("{}", EdgeId::from(7)), "e7");
    }

    #[test]
    fn node_id_ordering() {
        let a = NodeId::from(1);
        let b = NodeId::from(5);
        assert!(a < b);
    }

    #[test]
    fn id_gen_default() {
        let ids = IdGen::default();
        assert_eq!(ids.next_node().raw(), 0);
    }
}
