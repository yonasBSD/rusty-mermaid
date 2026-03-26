# Codebase Review — rusty-mermaid

Generated: 2026-03-26 (full audit)

## Stats

| Metric | Value |
|--------|-------|
| Production .rs files | 153 |
| Total tests | 1,743 |
| SVG goldens | 297 |
| Golden .mmd files | 300 |
| Property tests | 25 (all diagram types) + 6 dagre + 29 curve |
| Fuzz targets | 28 (all parsers + layout) |
| IR tests | 141 (5+ per type) |
| Compiler warnings | 0 |
| Unsafe blocks | 0 |
| Production `.unwrap()` (diagrams) | 0 |

---

## 1. Font Size Consistency — 25 hardcoded font sizes

**HIGH PRIORITY.** These should use `theme.font_size_*` fields.

| File | Count | Hardcoded values | Should use |
|------|-------|-----------------|------------|
| `c4/mod.rs` | 10 | 9.0, 10.0, 13.0 | `font_size_small`, `font_size_edge_label`, `font_size_node` |
| `architecture/mod.rs` | 3 | 9.0, 11.0, 12.0 | `font_size_small`, `font_size_edge_label` |
| `journey/mod.rs` | 3 | 11.0 | `font_size_edge_label` |
| `ishikawa/mod.rs` | 2 | 9.0, 14.0 | `font_size_small`, `font_size_node` |
| `quadrant/mod.rs` | 2 | 11.0, 16.0 | `font_size_edge_label`, `font_size_title` |
| `radar/mod.rs` | 2 | 9.0, 11.0 | `font_size_small`, `font_size_edge_label` |
| `block/mod.rs` | 1 | 10.0 | `font_size_edge_label` |
| `treemap/mod.rs` | 1 | 11.0 | `font_size_edge_label` |
| `class/bridge.rs` | 1 | 11.0 | `font_size_small` |

---

## 2. Long Functions (>80 lines)

| File | Function | Lines | Notes |
|------|----------|-------|-------|
| `class/parser.rs` | `peek_class_declaration` | 363 | Complex parser — needs breakup |
| `block/parser.rs` | `parse_block_body` | 256 | Parser — could extract helpers |
| `block/parser.rs` | `parse_edge` | 206 | Parser |
| `sequence/parser.rs` | multiple | 200+ | Parser |
| `c4/mod.rs` | `render_element` | ~100 | Rendering |
| `er/mod.rs` | `render_crowsfoot` | 107 | Crow's foot marker rendering |

Most are parsers — inherently sequential, hard to split without fragmenting logic.

---

## 3. Functions with >4 Parameters (18 found)

Mostly rendering functions with pattern `(scene, geometry..., theme)`:

| File | Function | Params |
|------|----------|--------|
| `journey/mod.rs` | `render_face` | 6 |
| `xychart/mod.rs` | `draw_y_axis` | 6 |
| `timeline/mod.rs` | `render_section_label_left` | 6 |
| `kanban/mod.rs` | `render_cards` | 6 |
| `dagre/position/bk.rs` | `scan_type2` | 10 |
| `dagre/position/bk.rs` | `horizontal_compaction` | 9 |

Dagre functions are ported from JS reference — changing them diverges from upstream.

---

## 4. `.unwrap()` in Production Code

| Crate | Count | Risk | Notes |
|-------|-------|------|-------|
| diagrams | 0 | None | All eliminated |
| dagre | ~45 | Low | Graph node lookups where dagre controls IDs |
| core | ~5 | Low | Font loading `.expect()`, not `.unwrap()` |
| svg | 0 | None | |
| raster | 0 | None | |

**One risky case:** `curve.rs:831` — `partial_cmp(b).unwrap()` panics on NaN. In test code only.

---

## 5. Test Coverage Gaps

### Backend test gaps

| Backend | Unit tests | Integration tests | Status |
|---------|-----------|-------------------|--------|
| SVG | 7 | 6 (e2e) | Adequate |
| Raster | 0 | 0 | **Missing** |
| GPUI | 0 | 0 | **Missing** |
| WGPU | 0 | 4 (gallery) | Minimal |

### Parser unit test gaps

19 diagram parsers rely on fuzz + golden tests but have no dedicated unit tests for edge cases. The 6 mature parsers (flowchart, state, sequence, class, er, requirement) have 13-54 unit tests each.

---

## 6. Naming Inconsistencies

| Pattern | Where | Convention |
|---------|-------|-----------|
| `render_*` | SVG, raster, diagrams | CPU backends |
| `paint_*` | gpui | GPU backends |
| `build_*` | wgpu/vello | Scene builders |
| `draw_*` | xychart (`draw_y_axis`, `draw_plots`) | Should be `render_*` |

`draw_*` in xychart breaks the `render_*` convention used by all other diagram renderers.

---

## 7. Performance Notes

- SVG marker dedup uses `Vec::contains()` — O(n²) but n < 100, negligible
- No `unsafe` blocks anywhere
- No O(n²) patterns in hot rendering paths
- `BTreeMap` used deliberately for deterministic output (documented in memory)

---

## 8. Architecture Health

| Aspect | Status |
|--------|--------|
| Crate DAG | **Clean** — no cycles |
| Scene as universal contract | **Clean** |
| Theme propagation | **Clean** — all renderers accept Theme |
| Feature gates (facade) | **Clean** — no defaults, all opt-in |
| Module structure (25 types) | **Consistent** — ir.rs + parser.rs + mod.rs |
| Test extraction (*_tests.rs) | **Consistent** — 18 files extracted |
| Named constants | **Clean** — palette.rs shared |
| Error types | **Clean** — ParseError throughout |

---

## 9. Action Items

### High priority

| # | Action | Impact | Effort |
|---|--------|--------|--------|
| 1 | Replace 25 hardcoded font sizes with `theme.font_size_*` | Consistency | 30 min |
| 2 | Add raster backend tests (render rect, circle, text, full diagram) | Coverage | 1 hour |
| 3 | Break up `peek_class_declaration` (363 lines) | Readability | 1 hour |

### Medium priority

| # | Action | Impact | Effort |
|---|--------|--------|--------|
| 4 | Rename xychart `draw_*` → `render_*` for convention consistency | Naming | 15 min |
| 5 | Add gpui backend smoke tests | Coverage | 30 min |
| 6 | Add parser unit tests for 19 under-tested diagram types | Coverage | 2 hours |
| 7 | Replace dagre `.unwrap()` with `.expect("context")` | Debuggability | 1 hour |

### Low priority

| # | Action | Impact | Effort |
|---|--------|--------|--------|
| 8 | Break up block/parser.rs long functions (256, 206 lines) | Readability | 1 hour |
| 9 | Reduce 6-param rendering functions | Readability | 30 min |
| 10 | Add SVG marker dedup HashSet optimization | Performance | 15 min |
