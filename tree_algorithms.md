# Tree Algorithms — Rust Power Reference (Arena-First)

Every solution uses the **arena pattern** — one technique for all tree problems.
No `Rc<RefCell<>>` gymnastics, no borrow checker fights.

---

## Table of Contents

1. [The Arena Pattern](#the-arena-pattern)
2. [Drop-in Converters](#drop-in-converters)
3. [Tree Building Helper](#tree-building-helper)
4. [Core Traversals](#core-traversals)
5. [Solved Problems](#solved-problems)
   - [LC 101 — Symmetric Tree](#lc-101)
   - [LC 104 — Maximum Depth](#lc-104)
   - [LC 226 — Invert Binary Tree](#lc-226)
   - [LC 236 — Lowest Common Ancestor](#lc-236)
   - [LC 543 — Diameter of Binary Tree](#lc-543)
   - [LC 98 — Validate BST](#lc-98)
   - [LC 297 — Serialize / Deserialize](#lc-297)
   - [LC 102 — Level-Order Traversal](#lc-102)
   - [LC 112 — Path Sum](#lc-112)
   - [LC 114 — Flatten to Linked List](#lc-114)
   - [LC 669 — Trim a BST (in → transform → out)](#lc-669)
   - [LC 1038 — BST to Greater Sum Tree (in → compute → out)](#lc-1038)
   - [LC 105 — Construct from Preorder + Inorder (build from scratch)](#lc-105)
   - [LC 450 — Delete Node in BST (arena → selective rebuild → out)](#lc-450)
6. [Full LeetCode Template (Diameter)](#full-leetcode-template)
7. [The Universal Template](#the-universal-template)
8. [Complexity Cheat Sheet](#complexity-cheat-sheet)

---

## The Arena Pattern

All nodes live in one flat `Vec`. Parent-child relationships are indices,
not references. Indices are `Copy` — pass them anywhere, store them anywhere.
No `Rc`, no `RefCell`, no lifetimes.

```rust
struct Arena {
    nodes: Vec<(i32, Option<usize>, Option<usize>)>, // (val, left_idx, right_idx)
}
```

**Why this works for everything:**
- `arena.val(i)` — O(1), no borrowing
- `arena.left(i)` / `arena.right(i)` — just `Option<usize>`, `Copy`
- Mutate node `i` while reading node `j` — no conflicts
- Works for trees, graphs, tries, segment trees — any linked structure
- The same pattern competitive programmers use in C++ (global array + indices)

**The workflow:**
1. **Convert in**: LeetCode tree → Arena (O(n))
2. **Solve**: work entirely with indices (clean, fast)
3. **Convert out**: Arena → LeetCode tree if needed (O(n))

---

## Drop-in Converters

Copy-paste this into any LeetCode solution:

```rust
use std::rc::Rc;
use std::cell::RefCell;

type Node = Option<Rc<RefCell<TreeNode>>>;

struct Arena {
    nodes: Vec<(i32, Option<usize>, Option<usize>)>,
}

impl Arena {
    fn new() -> Self { Arena { nodes: Vec::new() } }

    /// LeetCode tree → Arena. Returns (arena, root_index).
    fn from_leetcode(root: &Node) -> (Self, Option<usize>) {
        let mut a = Arena::new();
        let r = Self::build(&mut a, root);
        (a, r)
    }

    fn build(a: &mut Arena, node: &Node) -> Option<usize> {
        let rc = node.as_ref()?;
        let n = rc.borrow();
        let idx = a.nodes.len();
        a.nodes.push((n.val, None, None));
        let left = Self::build(a, &n.left);
        let right = Self::build(a, &n.right);
        a.nodes[idx].1 = left;
        a.nodes[idx].2 = right;
        Some(idx)
    }

    /// Arena → LeetCode tree.
    fn to_leetcode(&self, idx: Option<usize>) -> Node {
        let i = idx?;
        let (val, left, right) = self.nodes[i];
        Some(Rc::new(RefCell::new(TreeNode {
            val,
            left: self.to_leetcode(left),
            right: self.to_leetcode(right),
        })))
    }

    fn val(&self, i: usize) -> i32 { self.nodes[i].0 }
    fn left(&self, i: usize) -> Option<usize> { self.nodes[i].1 }
    fn right(&self, i: usize) -> Option<usize> { self.nodes[i].2 }
}
```

---

## Tree Building Helper

For local testing — build a tree from LeetCode's level-order format:

```rust
use std::collections::VecDeque;

fn from_level_order(vals: &[Option<i32>]) -> Node {
    if vals.is_empty() || vals[0].is_none() { return None; }
    let root = Rc::new(RefCell::new(TreeNode::new(vals[0].unwrap())));
    let mut queue = VecDeque::new();
    queue.push_back(Rc::clone(&root));
    let mut i = 1;
    while let Some(node) = queue.pop_front() {
        if i < vals.len() {
            if let Some(v) = vals[i] {
                let left = Rc::new(RefCell::new(TreeNode::new(v)));
                node.borrow_mut().left = Some(Rc::clone(&left));
                queue.push_back(left);
            }
            i += 1;
        }
        if i < vals.len() {
            if let Some(v) = vals[i] {
                let right = Rc::new(RefCell::new(TreeNode::new(v)));
                node.borrow_mut().right = Some(Rc::clone(&right));
                queue.push_back(right);
            }
            i += 1;
        }
    }
    Some(root)
}
```

---

## Core Traversals

All traversals are the same pattern: recurse on `a.left(i)` and `a.right(i)`.

```rust
fn inorder(a: &Arena, idx: Option<usize>, result: &mut Vec<i32>) {
    let Some(i) = idx else { return };
    inorder(a, a.left(i), result);
    result.push(a.val(i));
    inorder(a, a.right(i), result);
}

fn preorder(a: &Arena, idx: Option<usize>, result: &mut Vec<i32>) {
    let Some(i) = idx else { return };
    result.push(a.val(i));
    preorder(a, a.left(i), result);
    preorder(a, a.right(i), result);
}

fn postorder(a: &Arena, idx: Option<usize>, result: &mut Vec<i32>) {
    let Some(i) = idx else { return };
    postorder(a, a.left(i), result);
    postorder(a, a.right(i), result);
    result.push(a.val(i));
}
```

Zero `.borrow()` calls, zero `.clone()` calls. Just indices.

---

## Solved Problems

Every solution follows the same shape:
```rust
fn solve(root: Node) -> Answer {
    let (a, ri) = Arena::from_leetcode(&root);
    // ... recursive helper using a.val(i), a.left(i), a.right(i) ...
}
```

---

### LC 101 — Symmetric Tree {#lc-101}

**Pattern:** Dual-pointer mirror comparison

```rust
fn is_symmetric(root: Node) -> bool {
    let (a, ri) = Arena::from_leetcode(&root);
    fn mirror(a: &Arena, i: Option<usize>, j: Option<usize>) -> bool {
        match (i, j) {
            (None, None) => true,
            (Some(i), Some(j)) => {
                a.val(i) == a.val(j)
                    && mirror(a, a.left(i), a.right(j))
                    && mirror(a, a.right(i), a.left(j))
            }
            _ => false,
        }
    }
    let Some(r) = ri else { return true };
    mirror(&a, a.left(r), a.right(r))
}
```

**Key insight:** Compare left subtree's left with right subtree's right.

<details>
<summary>Iterative (BFS pair queue)</summary>

```rust
fn is_symmetric(root: Node) -> bool {
    let (a, ri) = Arena::from_leetcode(&root);
    let Some(r) = ri else { return true };
    let mut queue: VecDeque<(Option<usize>, Option<usize>)> = VecDeque::new();
    queue.push_back((a.left(r), a.right(r)));
    while let Some((left, right)) = queue.pop_front() {
        match (left, right) {
            (None, None) => {}
            (Some(l), Some(r)) => {
                if a.val(l) != a.val(r) { return false; }
                queue.push_back((a.left(l), a.right(r)));
                queue.push_back((a.right(l), a.left(r)));
            }
            _ => return false,
        }
    }
    true
}
```
</details>

---

### LC 104 — Maximum Depth {#lc-104}

**Pattern:** Simple recursion, base case returns 0

```rust
fn max_depth(root: Node) -> i32 {
    let (a, ri) = Arena::from_leetcode(&root);
    fn depth(a: &Arena, idx: Option<usize>) -> i32 {
        let Some(i) = idx else { return 0 };
        1 + depth(a, a.left(i)).max(depth(a, a.right(i)))
    }
    depth(&a, ri)
}
```

<details>
<summary>Iterative (BFS level count)</summary>

```rust
fn max_depth(root: Node) -> i32 {
    let (a, ri) = Arena::from_leetcode(&root);
    let Some(r) = ri else { return 0 };
    let mut queue = VecDeque::new();
    queue.push_back(r);
    let mut depth = 0;
    while !queue.is_empty() {
        depth += 1;
        for _ in 0..queue.len() {
            let i = queue.pop_front().unwrap();
            if let Some(l) = a.left(i) { queue.push_back(l); }
            if let Some(r) = a.right(i) { queue.push_back(r); }
        }
    }
    depth
}
```
</details>

---

### LC 226 — Invert Binary Tree {#lc-226}

**Pattern:** Rebuild with left/right swapped

```rust
fn invert_tree(root: Node) -> Node {
    let (a, ri) = Arena::from_leetcode(&root);
    fn build(a: &Arena, idx: Option<usize>) -> Node {
        let i = idx?;
        Some(Rc::new(RefCell::new(TreeNode {
            val: a.val(i),
            left: build(a, a.right(i)),   // swap
            right: build(a, a.left(i)),
        })))
    }
    build(&a, ri)
}
```

**Key insight:** Read from arena (original order), write to new tree (swapped).
No `.take()`, no borrow gymnastics.

---

### LC 236 — Lowest Common Ancestor {#lc-236}

**Pattern:** Postorder — bubble up found nodes

```rust
fn lowest_common_ancestor(root: Node, p: i32, q: i32) -> Node {
    let (a, ri) = Arena::from_leetcode(&root);
    fn find(a: &Arena, idx: Option<usize>, p: i32, q: i32) -> Option<usize> {
        let i = idx?;
        if a.val(i) == p || a.val(i) == q { return Some(i); }
        let left = find(a, a.left(i), p, q);
        let right = find(a, a.right(i), p, q);
        match (left, right) {
            (Some(_), Some(_)) => Some(i),  // both sides found → LCA
            (Some(_), None) => left,
            (None, r) => r,
        }
    }
    let lca = find(&a, ri, p, q);
    lca.map(|i| Rc::new(RefCell::new(TreeNode::new(a.val(i)))))
}
```

**Key insight:** Returns an arena index, not a reference. Convert back at the end.

<details>
<summary>Iterative (parent map + ancestor set)</summary>

```rust
fn lowest_common_ancestor(root: Node, p: i32, q: i32) -> i32 {
    let (a, ri) = Arena::from_leetcode(&root);
    let Some(r) = ri else { return -1 };
    let mut parent = vec![None::<usize>; a.nodes.len()];
    let mut stack = vec![r];
    while let Some(i) = stack.pop() {
        if let Some(l) = a.left(i) { parent[l] = Some(i); stack.push(l); }
        if let Some(r) = a.right(i) { parent[r] = Some(i); stack.push(r); }
    }
    let p_idx = (0..a.nodes.len()).find(|&i| a.val(i) == p).unwrap();
    let mut ancestors = std::collections::HashSet::new();
    let mut cur = Some(p_idx);
    while let Some(c) = cur { ancestors.insert(c); cur = parent[c]; }
    let q_idx = (0..a.nodes.len()).find(|&i| a.val(i) == q).unwrap();
    cur = Some(q_idx);
    while let Some(c) = cur {
        if ancestors.contains(&c) { return a.val(c); }
        cur = parent[c];
    }
    a.val(r)
}
```
</details>

---

### LC 543 — Diameter of Binary Tree {#lc-543}

**Pattern:** Postorder with side-channel `&mut` for max tracking

```rust
fn diameter_of_binary_tree(root: Node) -> i32 {
    let (a, ri) = Arena::from_leetcode(&root);
    fn dfs(a: &Arena, idx: Option<usize>, dia: &mut i32) -> i32 {
        let Some(i) = idx else { return 0 };
        let l = dfs(a, a.left(i), dia);
        let r = dfs(a, a.right(i), dia);
        *dia = (*dia).max(l + r);
        1 + l.max(r)
    }
    let mut d = 0;
    dfs(&a, ri, &mut d);
    d
}
```

**Key insight:** The function returns *height* for the parent, but tracks
*diameter* (left + right) via `&mut`. With arena, `&Arena` and `&mut i32`
don't conflict — the arena is read-only while the diameter is the only mutation.

<details>
<summary>Iterative (postorder with visited flag)</summary>

```rust
fn diameter_of_binary_tree(root: Node) -> i32 {
    let (a, ri) = Arena::from_leetcode(&root);
    let Some(r) = ri else { return 0 };
    let mut heights = vec![0i32; a.nodes.len()];
    let mut diameter = 0;
    let mut stack: Vec<(usize, bool)> = vec![(r, false)];
    while let Some((i, visited)) = stack.pop() {
        if visited {
            let lh = a.left(i).map(|l| heights[l]).unwrap_or(0);
            let rh = a.right(i).map(|r| heights[r]).unwrap_or(0);
            heights[i] = 1 + lh.max(rh);
            diameter = diameter.max(lh + rh);
        } else {
            stack.push((i, true));
            if let Some(r) = a.right(i) { stack.push((r, false)); }
            if let Some(l) = a.left(i) { stack.push((l, false)); }
        }
    }
    diameter
}
```

The `(node, visited)` pattern simulates postorder: first push children (unvisited),
then when popped again (visited=true), children's heights are already computed.
</details>

---

### LC 98 — Validate BST {#lc-98}

**Pattern:** Range validation with `(min, max)` bounds

```rust
fn is_valid_bst(root: Node) -> bool {
    let (a, ri) = Arena::from_leetcode(&root);
    fn valid(a: &Arena, idx: Option<usize>, min: i64, max: i64) -> bool {
        let Some(i) = idx else { return true };
        let v = a.val(i) as i64;
        if v <= min || v >= max { return false; }
        valid(a, a.left(i), min, v) && valid(a, a.right(i), v, max)
    }
    valid(&a, ri, i64::MIN, i64::MAX)
}
```

**Key insight:** Use `i64` for bounds to handle `i32::MIN`/`i32::MAX` edge cases.

<details>
<summary>Iterative (inorder traversal, values must increase)</summary>

```rust
fn is_valid_bst(root: Node) -> bool {
    let (a, ri) = Arena::from_leetcode(&root);
    let mut stack: Vec<usize> = Vec::new();
    let mut current = ri;
    let mut prev = i64::MIN;
    loop {
        while let Some(i) = current {
            stack.push(i);
            current = a.left(i);
        }
        let Some(i) = stack.pop() else { break };
        let v = a.val(i) as i64;
        if v <= prev { return false; }
        prev = v;
        current = a.right(i);
    }
    true
}
```
</details>

---

### LC 297 — Serialize / Deserialize {#lc-297}

**Pattern:** Preorder encoding with "null" sentinels

```rust
fn serialize(root: &Node) -> String {
    let (a, ri) = Arena::from_leetcode(root);
    fn ser(a: &Arena, idx: Option<usize>) -> String {
        let Some(i) = idx else { return "null".to_string() };
        format!("{},{},{}", a.val(i), ser(a, a.left(i)), ser(a, a.right(i)))
    }
    ser(&a, ri)
}

fn deserialize(data: &str) -> Node {
    let tokens: Vec<&str> = data.split(',').collect();
    let mut pos = 0;
    fn build(tokens: &[&str], pos: &mut usize) -> Node {
        if *pos >= tokens.len() || tokens[*pos] == "null" {
            *pos += 1;
            return None;
        }
        let val: i32 = tokens[*pos].parse().unwrap();
        *pos += 1;
        let left = build(tokens, pos);
        let right = build(tokens, pos);
        Some(Rc::new(RefCell::new(TreeNode { val, left, right })))
    }
    build(&tokens, &mut pos)
}
```

**Key insight:** Serialize uses arena for clean reading. Deserialize builds
LeetCode nodes directly (constructing, not traversing — no arena needed).

---

### LC 102 — Level-Order Traversal {#lc-102}

**Pattern:** BFS with `VecDeque` of arena indices

```rust
fn level_order(root: Node) -> Vec<Vec<i32>> {
    let (a, ri) = Arena::from_leetcode(&root);
    let mut result = Vec::new();
    let Some(r) = ri else { return result };
    let mut queue = VecDeque::new();
    queue.push_back(r);
    while !queue.is_empty() {
        let size = queue.len();
        let mut level = Vec::with_capacity(size);
        for _ in 0..size {
            let i = queue.pop_front().unwrap();
            level.push(a.val(i));
            if let Some(l) = a.left(i) { queue.push_back(l); }
            if let Some(r) = a.right(i) { queue.push_back(r); }
        }
        result.push(level);
    }
    result
}
```

**Key insight:** The queue holds `usize` indices, not `Rc<RefCell<>>`. No cloning,
no borrowing. Snapshot `queue.len()` to separate levels.

---

### LC 112 — Path Sum {#lc-112}

**Pattern:** Subtract-and-check at leaf nodes

```rust
fn has_path_sum(root: Node, target: i32) -> bool {
    let (a, ri) = Arena::from_leetcode(&root);
    fn check(a: &Arena, idx: Option<usize>, remaining: i32) -> bool {
        let Some(i) = idx else { return false };
        let r = remaining - a.val(i);
        if a.left(i).is_none() && a.right(i).is_none() {
            return r == 0;
        }
        check(a, a.left(i), r) || check(a, a.right(i), r)
    }
    check(&a, ri, target)
}
```

<details>
<summary>Iterative (DFS stack carrying remaining sum)</summary>

```rust
fn has_path_sum(root: Node, target: i32) -> bool {
    let (a, ri) = Arena::from_leetcode(&root);
    let Some(r) = ri else { return false };
    let mut stack: Vec<(usize, i32)> = vec![(r, target)];
    while let Some((i, remaining)) = stack.pop() {
        let r = remaining - a.val(i);
        if a.left(i).is_none() && a.right(i).is_none() && r == 0 {
            return true;
        }
        if let Some(l) = a.left(i) { stack.push((l, r)); }
        if let Some(ri) = a.right(i) { stack.push((ri, r)); }
    }
    false
}
```
</details>

---

### LC 114 — Flatten to Linked List {#lc-114}

**Pattern:** Preorder collect → rebuild as right-chain

```rust
fn flatten(root: &mut Node) {
    let (a, ri) = Arena::from_leetcode(root);
    let Some(r) = ri else { return };

    let mut order = Vec::new();
    fn preorder(a: &Arena, idx: Option<usize>, order: &mut Vec<usize>) {
        let Some(i) = idx else { return };
        order.push(i);
        preorder(a, a.left(i), order);
        preorder(a, a.right(i), order);
    }
    preorder(&a, Some(r), &mut order);

    fn build_list(a: &Arena, order: &[usize], pos: usize) -> Node {
        if pos >= order.len() { return None; }
        Some(Rc::new(RefCell::new(TreeNode {
            val: a.val(order[pos]),
            left: None,
            right: build_list(a, order, pos + 1),
        })))
    }
    *root = build_list(&a, &order, 0);
}
```

**Key insight:** In-place flatten with `Rc<RefCell<>>` requires careful pointer
surgery with `.take()`. Arena makes it trivial: collect preorder indices,
rebuild as a right-only chain.

---

### LC 669 — Trim a BST (from → mutate arena → to) {#lc-669}

**Pattern:** The full round-trip — `from_leetcode`, rewire pointers in the
arena to remove out-of-range nodes, then `to_leetcode`.

Given a BST and bounds `[low, high]`, remove all nodes outside the range.

```rust
pub fn trim_bst(
    root: Option<Rc<RefCell<TreeNode>>>,
    low: i32,
    high: i32,
) -> Option<Rc<RefCell<TreeNode>>> {
    // ── Step 1: Convert input to arena ──
    let (mut a, ri) = Arena::from_leetcode(&root);

    // ── Step 2: Trim in arena — rewire pointers to skip out-of-range nodes ──
    fn trim(a: &mut Arena, idx: Option<usize>, lo: i32, hi: i32) -> Option<usize> {
        let i = idx?;
        if a.val(i) < lo {
            // Too small → skip this node and its left subtree
            return trim(a, a.right(i), lo, hi);
        }
        if a.val(i) > hi {
            // Too big → skip this node and its right subtree
            return trim(a, a.left(i), lo, hi);
        }
        // In range — keep this node, trim children
        let new_left = trim(a, a.left(i), lo, hi);
        let new_right = trim(a, a.right(i), lo, hi);
        a.set_left(i, new_left);    // rewire left pointer
        a.set_right(i, new_right);  // rewire right pointer
        Some(i)
    }
    let new_root = trim(&mut a, ri, low, high);

    // ── Step 3: Convert arena back to LeetCode type ──
    a.to_leetcode(new_root)
}
```

**Key insight:** The arena is mutated — `set_left`/`set_right` rewire the
tree structure in-place by changing index pointers. Out-of-range nodes are
simply never pointed to anymore. Then `to_leetcode` reads the rewired
arena and builds a clean output tree.

Note: the Arena struct needs `set_left` and `set_right` added:
```rust
fn set_left(&mut self, i: usize, l: Option<usize>) { self.nodes[i].1 = l; }
fn set_right(&mut self, i: usize, r: Option<usize>) { self.nodes[i].2 = r; }
```

---

### LC 1038 — BST to Greater Sum Tree (from → mutate arena → to) {#lc-1038}

**Pattern:** The full round-trip — `from_leetcode`, mutate values in the arena,
then `to_leetcode` to return the modified tree.

Every node's value becomes the sum of all values ≥ itself in the BST.

```rust
pub fn bst_to_gst(root: Option<Rc<RefCell<TreeNode>>>) -> Option<Rc<RefCell<TreeNode>>> {
    // ── Step 1: Convert input to arena ──
    let (mut a, ri) = Arena::from_leetcode(&root);

    // ── Step 2: Solve in arena (reverse inorder, accumulate sums) ──
    fn rev_inorder(a: &mut Arena, idx: Option<usize>, sum: &mut i32) {
        let Some(i) = idx else { return };
        let r = a.right(i);
        rev_inorder(a, r, sum);       // visit right first (larger values)
        *sum += a.val(i);             // accumulate running sum
        a.set_val(i, *sum);           // mutate node value in-place
        let l = a.left(i);
        rev_inorder(a, l, sum);       // then left (smaller values)
    }
    let mut sum = 0;
    rev_inorder(&mut a, ri, &mut sum);

    // ── Step 3: Convert arena back to LeetCode type ──
    a.to_leetcode(ri)
}
```

**Key insight:** The arena is mutable — `a.set_val(i, *sum)` writes the new
value directly into the arena node. No separate `new_vals` vec, no rebuild
function. After mutation, `a.to_leetcode(ri)` constructs a fresh
`Rc<RefCell<TreeNode>>` tree with the updated values.

This is the cleanest `from → mutate → to` round-trip: **3 lines of glue**
(from, solve, to) with the actual algorithm in between.

Note: the Arena struct needs `set_val` added:
```rust
fn set_val(&mut self, i: usize, v: i32) { self.nodes[i].0 = v; }
```

---

### LC 105 — Construct from Preorder + Inorder (build from scratch) {#lc-105}

**Pattern:** No input tree — build directly from arrays. No arena needed.

```rust
fn build_tree(preorder: Vec<i32>, inorder: Vec<i32>) -> Node {
    fn build(preorder: &[i32], inorder: &[i32]) -> Node {
        if preorder.is_empty() { return None; }
        let root_val = preorder[0];
        let mid = inorder.iter().position(|&v| v == root_val)?;
        Some(Rc::new(RefCell::new(TreeNode {
            val: root_val,
            left: build(&preorder[1..=mid], &inorder[..mid]),
            right: build(&preorder[mid + 1..], &inorder[mid + 1..]),
        })))
    }
    build(&preorder, &inorder)
}
```

**Key insight:** When there's no input tree, there's no arena. You build the
LeetCode `Rc<RefCell<>>` tree directly. Preorder gives the root, inorder
tells you how to split left/right. Slice indexing does the rest.

---

### LC 450 — Delete Node in BST (arena → selective rebuild → out) {#lc-450}

**Pattern:** Read BST structure from arena, rebuild skipping the deleted node.

```rust
fn delete_node(root: Node, key: i32) -> Node {
    let (a, ri) = Arena::from_leetcode(&root);
    fn build(a: &Arena, idx: Option<usize>, key: i32) -> Node {
        let i = idx?;
        if a.val(i) == key {
            match (a.left(i), a.right(i)) {
                (None, None) => None,
                (Some(l), None) => build(a, Some(l), i32::MIN),
                (None, Some(r)) => build(a, Some(r), i32::MIN),
                (Some(_), Some(r)) => {
                    let mut succ = r;
                    while let Some(sl) = a.left(succ) { succ = sl; }
                    Some(Rc::new(RefCell::new(TreeNode {
                        val: a.val(succ),
                        left: build(a, a.left(i), i32::MIN),
                        right: build(a, a.right(i), a.val(succ)),
                    })))
                }
            }
        } else {
            Some(Rc::new(RefCell::new(TreeNode {
                val: a.val(i),
                left: build(a, a.left(i), key),
                right: build(a, a.right(i), key),
            })))
        }
    }
    build(&a, ri, key)
}
```

**Key insight:** Finding the inorder successor is just `while let Some(sl) = a.left(succ)` —
pure index walking, no borrow conflicts. The `Rc<RefCell<>>` version needs nested
borrows and `.take()` chains for the same operation.

---

## Full LeetCode Template (Diameter) {#full-leetcode-template}

Copy-paste this as-is into LeetCode. It includes the Arena, converter,
and the solution — everything in one `impl Solution` block:

```rust
use std::rc::Rc;
use std::cell::RefCell;

// Definition for a binary tree node (provided by LeetCode)
// #[derive(Debug, PartialEq, Eq)]
// pub struct TreeNode {
//     pub val: i32,
//     pub left: Option<Rc<RefCell<TreeNode>>>,
//     pub right: Option<Rc<RefCell<TreeNode>>>,
// }

impl Solution {
    pub fn diameter_of_binary_tree(root: Option<Rc<RefCell<TreeNode>>>) -> i32 {
        // ── Arena (paste once, reuse for any problem) ──
        struct A(Vec<(i32, Option<usize>, Option<usize>)>);
        impl A {
            fn new() -> Self { A(Vec::new()) }
            fn from(root: &Option<Rc<RefCell<TreeNode>>>) -> (Self, Option<usize>) {
                let mut a = A::new(); let r = Self::b(&mut a, root); (a, r)
            }
            fn b(a: &mut A, n: &Option<Rc<RefCell<TreeNode>>>) -> Option<usize> {
                let rc = n.as_ref()?; let n = rc.borrow();
                let i = a.0.len(); a.0.push((n.val, None, None));
                let l = Self::b(a, &n.left); let r = Self::b(a, &n.right);
                a.0[i].1 = l; a.0[i].2 = r; Some(i)
            }
            fn l(&self, i: usize) -> Option<usize> { self.0[i].1 }
            fn r(&self, i: usize) -> Option<usize> { self.0[i].2 }
        }

        // ── Solution logic ──
        let (a, ri) = A::from(&root);
        fn dfs(a: &A, idx: Option<usize>, dia: &mut i32) -> i32 {
            let Some(i) = idx else { return 0 };
            let l = dfs(a, a.l(i), dia);
            let r = dfs(a, a.r(i), dia);
            *dia = (*dia).max(l + r);
            1 + l.max(r)
        }
        let mut d = 0;
        dfs(&a, ri, &mut d);
        d
    }
}
```

**To adapt for another problem:** keep the `struct A` block unchanged.
Replace only the solution logic section after `let (a, ri) = A::from(&root);`.

---

## The Universal Template

```rust
impl Solution {
    pub fn solve(root: Option<Rc<RefCell<TreeNode>>>) -> Answer {
        let (a, ri) = Arena::from_leetcode(&root);

        fn helper(a: &Arena, idx: Option<usize>) -> Result {
            let Some(i) = idx else { return base_case; };
            let val = a.val(i);
            let left = helper(a, a.left(i));
            let right = helper(a, a.right(i));
            // combine val, left, right → result
        }

        helper(&a, ri)
    }
}
```

**When you need to return a tree:** use `a.to_leetcode(idx)`.
**When you need mutation tracking:** use `&mut` side-channel (like diameter).
**When you need BFS:** use `VecDeque<usize>` with arena indices.

There is no tree problem where this pattern doesn't work.

---

## Complexity Cheat Sheet

| Operation | Time | Space |
|-----------|------|-------|
| Arena conversion (in + out) | O(n) each | O(n) |
| DFS traversal (in/pre/post) | O(n) | O(h) stack |
| BFS / level-order | O(n) | O(w) queue |
| LCA | O(n) | O(h) |
| Validate BST | O(n) | O(h) |
| Serialize / Deserialize | O(n) | O(n) |
| Diameter | O(n) | O(h) |

Where `n` = nodes, `h` = height (log n balanced, n worst case), `w` = max width.

The O(n) arena conversion is always dominated by the O(n) solve step — it never
changes your asymptotic complexity.

---

## Quick Reference: Arena vs Rc<RefCell<>>

```rust
// ─── Arena (clean) ───────────────────────
let Some(i) = idx else { return base; };
let val = a.val(i);
let left = solve(a, a.left(i));
let right = solve(a, a.right(i));

// ─── Rc<RefCell<>> (noisy) ───────────────
let Some(rc) = node else { return base; };
let n = rc.borrow();
let val = n.val;
let left = solve(n.left.clone());
let right = solve(n.right.clone());
```

Arena wins on every axis: less code, no `.borrow()`, no `.clone()`, no
lifetime issues, no runtime panics from double borrows.
