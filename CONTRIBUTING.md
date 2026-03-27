# Contributing to rusty-mermaid

## Getting Started

```sh
git clone https://github.com/base58ed/rusty-mermaid.git
cd rusty-mermaid
cargo test  # 1,793 tests — should all pass
```

## Project Structure

```
crates/
├── core/          — Scene, Primitive, Theme, geometry, text measurement
├── graph/         — Graph<N,E> directed multigraph
├── dagre/         — Sugiyama layout engine (ported from dagre.js)
├── diagrams/      — 25 diagram types: parser → IR → layout → Scene
│   └── src/{type}/
│       ├── ir.rs      — data model
│       ├── parser.rs  — mermaid syntax → IR
│       ├── bridge.rs  — IR → dagre → layout (graph-based types)
│       └── mod.rs     — layout → Scene primitives
├── svg/           — Scene → SVG string
├── raster/        — Scene → PNG (tiny-skia)
├── viewport/      — pan/zoom state + coordinate transforms
├── gpui-backend/  — Scene → gpui canvas (Zed)
├── wgpu-backend/  — Scene → vello (WebGPU/Metal)
└── rusty-mermaid/ — facade crate with feature gates
```

## Adding a Diagram Type

1. Create `crates/diagrams/src/{type}/` with `ir.rs`, `parser.rs`, `mod.rs`
2. Wire `detect()` + `render_to_scene()` in `lib.rs`
3. Add golden `.mmd` files to `tests/golden/mmd/{type}/` (5+ files)
4. Run `UPDATE_GOLDEN_SVG=1 cargo test` to generate SVG goldens
5. Add a fuzz target in `fuzz/fuzz_targets/{type}_parse.rs`
6. Add a proptest in `crates/diagrams/tests/proptest_scenes.rs`
7. Add IR tests in `{type}/ir.rs` (5+ tests)

No per-backend work needed — the Scene contract handles it.

## Conventions

### Naming

| Context | Convention | Example |
|---------|-----------|---------|
| Diagram renderers | `render_*` | `render_nodes`, `render_edges` |
| gpui backend | `paint_*` | `paint_rect`, `paint_text` |
| wgpu backend | `build_*` | `build_vello_scene` |

### Code Style

- **No hardcoded colors** — use `theme.*` fields
- **No hardcoded font sizes** — use `theme.font_size_*`
- **No `.unwrap()` in production code** — use `?`, `.expect("context")`, or `let Some(x) = ... else { return }`
- **Named constants** for magic numbers — shared ones in `common/palette.rs`
- **Functions ≤80 lines** — extract helpers, use orchestrator pattern
- **Functions ≤4 params** — bundle related params into structs
- **`BTreeMap`/`BTreeSet`** for deterministic output (not `HashMap`)
- **Tests in `*_tests.rs`** files (not inline) for files with >100 lines of tests

### Theme

All renderers read from `Theme`. Available fields:

```
Colors: node_fill, node_stroke, node_text, edge_stroke, edge_label_text,
        edge_label_bg, composite_fill, note_fill, subgraph_fill,
        grid_stroke, muted_text, face_fill, detail_stroke, background

Font sizes: font_size_node (14), font_size_label (13),
            font_size_edge_label (12), font_size_small (11),
            font_size_tiny (9), font_size_title (16)
```

### Shared Palette

`common/palette.rs` provides:

```rust
palette_color(idx)           // 8-color rotating palette
tint_color(color, ratio)     // white-blend for glassy fills
DARK_GRAY, MEDIUM_GRAY, GRAY, LIGHT_GRAY, GRID_GRAY  // named grays
BORDER_RADIUS, STROKE_WIDTH, DOTTED_PATTERN            // shared constants
```

## Testing

```sh
cargo test                              # all 1,793 tests
cargo test --test svg_golden            # SVG byte-exact regression
UPDATE_GOLDEN_SVG=1 cargo test          # regenerate SVG goldens after rendering changes
cargo fuzz run {target} -- -max_len=4096  # fuzz a parser
```

### Test types per diagram

| Type | Required |
|------|----------|
| Parser unit tests | 12+ per type |
| IR tests | 5+ per type |
| Golden .mmd files | 5+ per type |
| SVG golden regression | auto-generated |
| Property test | 1 per type |
| Fuzz target | 1 per type |

## Pull Request Checklist

- [ ] `cargo test` passes (all 1,793+ tests)
- [ ] `cargo fmt --all -- --check` passes
- [ ] `cargo clippy --workspace -- -D warnings` passes
- [ ] No new `.unwrap()` in production code
- [ ] No hardcoded colors or font sizes — use theme fields
- [ ] SVG goldens regenerated if rendering changed
- [ ] New diagram types include all required test types
