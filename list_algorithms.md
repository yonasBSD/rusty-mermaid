# Linked List Algorithms — Rust Power Reference

Every solution uses the simplest correct approach: **Vec roundtrip** for most
problems, **arena** for cycles/multi-pointer, **direct Box** for classic
pointer surgery (reverse, merge).

---

## Table of Contents

1. [Strategy](#strategy)
2. [Drop-in Converters](#drop-in-converters)
3. [Core Techniques](#core-techniques)
4. [Solved Problems](#solved-problems)
   - [LC 206 — Reverse Linked List](#lc-206)
   - [LC 21 — Merge Two Sorted Lists](#lc-21)
   - [LC 141 — Linked List Cycle (Arena)](#lc-141)
   - [LC 19 — Remove Nth From End](#lc-19)
   - [LC 234 — Palindrome Linked List](#lc-234)
   - [LC 23 — Merge K Sorted Lists](#lc-23)
   - [LC 2 — Add Two Numbers](#lc-2)
   - [LC 328 — Odd Even Linked List](#lc-328)
   - [LC 143 — Reorder List](#lc-143)
   - [LC 148 — Sort List](#lc-148)
   - [LC 24 — Swap Nodes in Pairs](#lc-24)
   - [LC 61 — Rotate List](#lc-61)
   - [LC 86 — Partition List](#lc-86)
   - [LC 82 — Remove Duplicates II](#lc-82)
5. [Full LeetCode Template (Merge Two Lists)](#full-leetcode-template-list)
6. [Complexity Cheat Sheet](#complexity-cheat-sheet)

---

## Strategy

LeetCode gives you `Option<Box<ListNode>>` — exclusive ownership, no shared refs.

| Strategy | When | Problems |
|----------|------|----------|
| **Vec roundtrip** | Default for ~80% of problems | palindrome, sort, remove nth, reorder, merge k, add two, odd-even |
| **Direct Box** | Classic pointer surgery patterns | reverse, merge two sorted |
| **Arena** | Cycles, multi-pointer, intersection | cycle detection, Floyd's |

**Start with Vec.** Only use Box surgery or Arena when Vec can't express it.

---

## Drop-in Converters

### Vec (primary — use for most problems)

```rust
type List = Option<Box<ListNode>>;

fn to_vec(head: &List) -> Vec<i32> {
    let mut result = Vec::new();
    let mut cur = head;
    while let Some(node) = cur {
        result.push(node.val);
        cur = &node.next;
    }
    result
}

fn from_vec(vals: &[i32]) -> List {
    let mut head: List = None;
    for &v in vals.iter().rev() {
        head = Some(Box::new(ListNode { val: v, next: head }));
    }
    head
}
```

### Arena (for cycles / multi-pointer)

```rust
struct Arena {
    nodes: Vec<(i32, Option<usize>)>,
}

impl Arena {
    fn new() -> Self { Arena { nodes: Vec::new() } }

    fn from_leetcode(head: &List) -> (Self, Option<usize>) {
        let mut arena = Arena::new();
        let mut indices = Vec::new();
        let mut cur = head;
        while let Some(node) = cur {
            let idx = arena.nodes.len();
            arena.nodes.push((node.val, None));
            indices.push(idx);
            cur = &node.next;
        }
        for i in 0..indices.len().saturating_sub(1) {
            arena.nodes[indices[i]].1 = Some(indices[i + 1]);
        }
        (arena, indices.first().copied())
    }

    fn to_leetcode(&self, head: Option<usize>) -> List {
        let mut result: List = None;
        let mut indices = Vec::new();
        let mut cur = head;
        while let Some(i) = cur {
            indices.push(i);
            cur = self.nodes[i].1;
        }
        for &i in indices.iter().rev() {
            result = Some(Box::new(ListNode { val: self.nodes[i].0, next: result }));
        }
        result
    }

    fn val(&self, i: usize) -> i32 { self.nodes[i].0 }
    fn next(&self, i: usize) -> Option<usize> { self.nodes[i].1 }
    fn set_next(&mut self, i: usize, next: Option<usize>) { self.nodes[i].1 = next; }
    fn push(&mut self, val: i32) -> usize {
        let idx = self.nodes.len();
        self.nodes.push((val, None));
        idx
    }
}
```

---

## Core Techniques

### In-Place Reverse (Box)

The three-pointer technique — one of two patterns worth memorizing with Box:

```rust
fn reverse(head: List) -> List {
    let mut prev: List = None;
    let mut curr = head;
    while let Some(mut node) = curr {
        curr = node.next.take();   // save next
        node.next = prev;          // reverse pointer
        prev = Some(node);         // advance prev
    }
    prev
}
```

### Recursive Merge (Box)

The other Box pattern worth memorizing — pick the smaller head:

```rust
fn merge(l1: List, l2: List) -> List {
    match (l1, l2) {
        (None, r) => r,
        (l, None) => l,
        (Some(mut a), Some(mut b)) => {
            if a.val <= b.val {
                a.next = merge(a.next, Some(b));
                Some(a)
            } else {
                b.next = merge(Some(a), b.next);
                Some(b)
            }
        }
    }
}
```

### The Vec Template

```rust
fn solve(head: List) -> List {
    let mut v = to_vec(&head);
    // ... full Vec API: index, sort, slice, iterate ...
    from_vec(&v)
}
```

---

## Solved Problems

### LC 206 — Reverse Linked List {#lc-206}

**Approach:** Direct Box — the canonical `.take()` pattern

```rust
fn reverse_list(head: List) -> List {
    let mut prev: List = None;
    let mut curr = head;
    while let Some(mut node) = curr {
        curr = node.next.take();
        node.next = prev;
        prev = Some(node);
    }
    prev
}
```

<details>
<summary>Arena round-trip (from_leetcode → mutate → to_leetcode)</summary>

```rust
fn reverse_list(head: List) -> List {
    let (mut a, h) = Arena::from_leetcode(&head);
    let Some(h) = h else { return None };
    let mut prev: Option<usize> = None;
    let mut curr = Some(h);
    while let Some(c) = curr {
        let next = a.next(c);
        a.set_next(c, prev);  // reverse pointer in-place
        prev = Some(c);
        curr = next;
    }
    a.to_leetcode(prev)  // convert back with new head
}
```

Same three-pointer algorithm, but on arena indices. `set_next` is just an
array write — no ownership transfer needed.
</details>

---

### LC 21 — Merge Two Sorted Lists {#lc-21}

**Approach:** Direct Box — recursive pick-smaller

```rust
fn merge_two_lists(l1: List, l2: List) -> List {
    match (l1, l2) {
        (None, r) => r,
        (l, None) => l,
        (Some(mut a), Some(mut b)) => {
            if a.val <= b.val {
                a.next = merge_two_lists(a.next, Some(b));
                Some(a)
            } else {
                b.next = merge_two_lists(Some(a), b.next);
                Some(b)
            }
        }
    }
}
```

---

### LC 141 — Linked List Cycle (Arena) {#lc-141}

**Approach:** Arena — `Box` can't form cycles, arena indices can

```rust
fn has_cycle(arena: &Arena, head: Option<usize>) -> bool {
    let mut slow = head;
    let mut fast = head;
    loop {
        slow = slow.and_then(|i| arena.next(i));
        fast = fast.and_then(|i| arena.next(i)).and_then(|i| arena.next(i));
        match (slow, fast) {
            (None, _) | (_, None) => return false,
            (Some(s), Some(f)) if s == f => return true,
            _ => {}
        }
    }
}
```

```rust
// Build a cycle for testing:
let mut arena = Arena::new();
let n0 = arena.push(1);
let n1 = arena.push(2);
let n2 = arena.push(3);
arena.set_next(n0, Some(n1));
arena.set_next(n1, Some(n2));
arena.set_next(n2, Some(n0)); // cycle!
assert!(has_cycle(&arena, Some(n0)));
```

---

### LC 19 — Remove Nth Node From End {#lc-19}

**Approach:** Vec — trivial index arithmetic

```rust
fn remove_nth_from_end(head: List, n: i32) -> List {
    let mut v = to_vec(&head);
    let idx = v.len() - n as usize;
    v.remove(idx);
    from_vec(&v)
}
```

---

### LC 234 — Palindrome Linked List {#lc-234}

**Approach:** Vec — one-liner

```rust
fn is_palindrome(head: List) -> bool {
    let v = to_vec(&head);
    v.iter().eq(v.iter().rev())
}
```

---

### LC 23 — Merge K Sorted Lists {#lc-23}

**Approach:** Vec — flatten all, sort, rebuild

```rust
fn merge_k_lists(lists: Vec<List>) -> List {
    let mut vals: Vec<i32> = lists.iter().flat_map(|l| to_vec(l)).collect();
    vals.sort();
    from_vec(&vals)
}
```

---

### LC 2 — Add Two Numbers {#lc-2}

**Approach:** Vec — digit-by-digit with carry

```rust
fn add_two_numbers(l1: List, l2: List) -> List {
    let mut v1 = to_vec(&l1);
    let mut v2 = to_vec(&l2);
    let max_len = v1.len().max(v2.len());
    v1.resize(max_len, 0);
    v2.resize(max_len, 0);

    let mut carry = 0;
    let mut result = Vec::with_capacity(max_len + 1);
    for i in 0..max_len {
        let sum = v1[i] + v2[i] + carry;
        result.push(sum % 10);
        carry = sum / 10;
    }
    if carry > 0 { result.push(carry); }
    from_vec(&result)
}
```

---

### LC 328 — Odd Even Linked List {#lc-328}

**Approach:** Vec — partition by index parity

```rust
fn odd_even_list(head: List) -> List {
    let v = to_vec(&head);
    let odd: Vec<i32> = v.iter().step_by(2).copied().collect();
    let even: Vec<i32> = v.iter().skip(1).step_by(2).copied().collect();
    let mut result = odd;
    result.extend(even);
    from_vec(&result)
}
```

---

### LC 143 — Reorder List {#lc-143}

**Approach:** Vec — interleave from both ends

```rust
fn reorder_list(head: &mut List) {
    let v = to_vec(head);
    if v.len() <= 2 { return; }
    let mut result = Vec::with_capacity(v.len());
    let (mut lo, mut hi) = (0, v.len() - 1);
    while lo <= hi {
        result.push(v[lo]);
        if lo != hi { result.push(v[hi]); }
        lo += 1;
        if hi == 0 { break; }
        hi -= 1;
    }
    *head = from_vec(&result);
}
```

**Key insight:** With Vec you have random access. Doing this in-place on a
singly-linked list requires finding the middle, reversing the second half,
then merging — far more complex for the same result.

---

### LC 148 — Sort List {#lc-148}

**Approach:** Vec — delegate to Rust's sort

```rust
fn sort_list(head: List) -> List {
    let mut v = to_vec(&head);
    v.sort();
    from_vec(&v)
}
```

---

### LC 24 — Swap Nodes in Pairs {#lc-24}

**Approach:** Vec — swap adjacent pairs by index

```rust
fn swap_pairs(head: List) -> List {
    let mut v = to_vec(&head);
    for i in (0..v.len()).step_by(2) {
        if i + 1 < v.len() { v.swap(i, i + 1); }
    }
    from_vec(&v)
}
```

---

### LC 61 — Rotate List {#lc-61}

**Approach:** Vec — split and rejoin

```rust
fn rotate_right(head: List, k: i32) -> List {
    let v = to_vec(&head);
    if v.is_empty() { return None; }
    let k = k as usize % v.len();
    if k == 0 { return from_vec(&v); }
    let (left, right) = v.split_at(v.len() - k);
    let mut rotated = right.to_vec();
    rotated.extend_from_slice(left);
    from_vec(&rotated)
}
```

**Key insight:** `split_at` + rejoin is O(n). The linked list version needs
to find the split point by walking, then re-linking — same complexity, more code.

---

### LC 86 — Partition List {#lc-86}

**Approach:** Vec — stable partition by filter

```rust
fn partition(head: List, x: i32) -> List {
    let v = to_vec(&head);
    let less: Vec<i32> = v.iter().copied().filter(|&val| val < x).collect();
    let geq: Vec<i32> = v.iter().copied().filter(|&val| val >= x).collect();
    let mut result = less;
    result.extend(geq);
    from_vec(&result)
}
```

**Key insight:** Two-pass filter preserves relative order within each group
(stable partition). The linked list version needs two dummy heads and pointer
surgery — same result, triple the code.

---

### LC 82 — Remove Duplicates from Sorted List II {#lc-82}

**Approach:** Vec — count occurrences, keep uniques

```rust
fn delete_duplicates(head: List) -> List {
    let v = to_vec(&head);
    let mut counts = std::collections::HashMap::new();
    for &val in &v { *counts.entry(val).or_insert(0) += 1; }
    let result: Vec<i32> = v.into_iter().filter(|val| counts[val] == 1).collect();
    from_vec(&result)
}
```

---

## Full LeetCode Template (Merge Two Lists) {#full-leetcode-template-list}

Copy-paste this as-is into LeetCode. Self-contained with `to_vec` and `from_vec`:

```rust
// Definition for singly-linked list (provided by LeetCode)
// #[derive(PartialEq, Eq, Clone, Debug)]
// pub struct ListNode {
//     pub val: i32,
//     pub next: Option<Box<ListNode>>
// }

impl Solution {
    pub fn merge_two_lists(
        list1: Option<Box<ListNode>>,
        list2: Option<Box<ListNode>>,
    ) -> Option<Box<ListNode>> {
        // ── Converters (paste once, reuse for any problem) ──
        fn to_vec(head: &Option<Box<ListNode>>) -> Vec<i32> {
            let mut r = Vec::new();
            let mut c = head;
            while let Some(n) = c { r.push(n.val); c = &n.next; }
            r
        }
        fn from_vec(vals: &[i32]) -> Option<Box<ListNode>> {
            let mut h: Option<Box<ListNode>> = None;
            for &v in vals.iter().rev() {
                h = Some(Box::new(ListNode { val: v, next: h }));
            }
            h
        }

        // ── Solution logic ──
        let mut v = to_vec(&list1);
        v.extend(to_vec(&list2));
        v.sort();
        from_vec(&v)
    }
}
```

**To adapt for another problem:** keep the `to_vec` / `from_vec` functions unchanged.
Replace only the solution logic section.

---

## Complexity Cheat Sheet

| Operation | Time | Space |
|-----------|------|-------|
| `to_vec` / `from_vec` | O(n) each | O(n) |
| Reverse (in-place Box) | O(n) | O(1) |
| Merge two sorted | O(n + m) | O(1) stack |
| Merge K sorted (sort) | O(N log N) | O(N) |
| Cycle detection (Floyd) | O(n) | O(1) |
| Sort (via vec) | O(n log n) | O(n) |
| Arena conversion | O(n) each | O(n) |

---

## Quick Reference: Rust List Idioms

```rust
// Iterate without consuming
let mut cur = &head;
while let Some(node) = cur {
    println!("{}", node.val);
    cur = &node.next;
}

// .take() — the key to Box list surgery
let next = node.next.take();  // node.next is now None

// Build from back to front
let mut head: List = None;
for &v in [3, 2, 1].iter() {
    head = Some(Box::new(ListNode { val: v, next: head }));
}
// head → 1 → 2 → 3
```
