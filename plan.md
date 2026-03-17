# rusty-mermaid: Implementation Plan

Port of mermaid/dagre diagramming to idiomatic Rust.
All 26 mermaid diagram types. SVG rendering first, gpui later.

---

## Architecture

```
                         ┌────────────────────────────────────┐
                         │          diagrams crate            │
                         │  26 feature-gated diagram modules  │
                         │  each: parse(text) → IR → Scene    │
                         │                                    │
                         │  ┌────────────┐  ┌──────────────┐  │
                         │  │ flowchart  │  │  sequence     │  │
                         │  │ state      │  │  gantt        │  │
                         │  │ class      │  │  pie          │  │
                         │  │ er         │  │  sankey       │  │
                         │  │ requirement│  │  xychart ...  │  │
                         │  │ mindmap    │  │  (custom      │  │
                         │  │            │  │   layout)     │  │
                         │  │ (use dagre)│  │               │  │
                         │  └─────┬──────┘  └───────┬───────┘  │
                         └────────┼─────────────────┼──────────┘
                                  │                 │
                    ┌─────────────┘     produces Scene
                    ▼                       │
              ┌───────────┐                 │
              │   dagre   │                 │
              │ (Sugiyama)│                 │
              └─────┬─────┘                 │
                    │                       │
         ┌──────────┤                       │
         ▼          ▼                       ▼
    ┌─────────┐ ┌─────────┐          ┌──────────┐
    │  graph  │ │  core   │◀─────────│  core    │
    │(multigr)│ │(types,  │          │(Scene,   │
    └────┬────┘ │ geom)   │          │ traits)  │
         │      └─────────┘          └────┬─────┘
         │            ▲                   │
         └────────────┘            consumed by
                                          │
                                ┌─────────┼─────────┐
                                ▼                   ▼
                          ┌──────────┐       ┌──────────┐
                          │   svg    │       │   gpui   │
                          │(backend) │       │ (future) │
                          └──────────┘       └──────────┘
```

### 5 Crates

```
core       zero deps           shared types, geometry, Scene primitives, traits
graph      → core              directed multigraph + compound hierarchy
dagre      → core, graph       Sugiyama layout (used by 6 diagram types)
diagrams   → core, graph?, dagre?   all 26 diagrams: parse + layout → Scene
svg        → core              Scene → SVG string
```

`graph` and `dagre` are optional deps of `diagrams` — only pulled in by
feature flags for graph-based diagram types. Non-graph diagrams (pie, gantt,
sequence, etc.) compile with zero graph/dagre overhead.

### Static Dispatch Everywhere

```rust
// core — associated type, monomorphized
pub trait Renderer {
    type Output;
    fn render(&self, scene: &Scene) -> Self::Output;
}

// TextMeasure — generic parameter, not dyn
pub trait TextMeasure {
    fn measure(&self, text: &str, style: &TextStyle) -> (f64, f64);
}

// dagre — generic over TextMeasure, no vtable
pub fn layout<T: TextMeasure>(
    graph: &mut Graph<NodeLabel, EdgeLabel>,
    config: &DagreConfig,
    text: &T,
) -> Scene;

// diagrams — each module returns concrete Scene, no boxing
pub fn to_scene(ir: &FlowDiagram, config: &DagreConfig) -> Scene;
```

No `Box<dyn>` in the hot path. The caller's choice of renderer and text
measurer is resolved at compile time.

---

## All 26 Diagram Types

### Graph-Based (use dagre — feature-gated)

| # | Diagram | Mermaid keyword | Layout | Complexity |
|---|---------|-----------------|--------|------------|
| 1 | Flowchart | `graph`, `flowchart` | Sugiyama (dagre) | Complex |
| 2 | State | `stateDiagram-v2` | Sugiyama (dagre) | Complex |
| 3 | Class | `classDiagram` | Sugiyama (dagre) | Moderate |
| 4 | ER | `erDiagram` | Sugiyama (dagre) | Moderate |
| 5 | Requirement | `requirementDiagram` | Sugiyama (dagre) | Moderate |
| 6 | Mindmap | `mindmap` | Sugiyama (dagre) | Moderate |

### Non-Graph (custom layout — no dagre dependency)

| # | Diagram | Mermaid keyword | Layout strategy | Complexity |
|---|---------|-----------------|-----------------|------------|
| 7 | Sequence | `sequenceDiagram` | Sequential constraint solving | Complex |
| 8 | Gantt | `gantt` | Time-axis positioning | Moderate |
| 9 | Pie | `pie` | Polar arc computation | Trivial |
| 10 | XY Chart | `xychart-beta` | Axis/scale positioning | Moderate |
| 11 | Sankey | `sankey-beta` | Sankey flow algorithm | Moderate |
| 12 | Timeline | `timeline` | Section-based vertical | Moderate |
| 13 | Git | `gitGraph` | Lane-based commit layout | Moderate |
| 14 | Kanban | `kanban` | Column stacking | Trivial |
| 15 | Block | `block-beta` | Custom hierarchical grid | Moderate |
| 16 | C4 | `C4Context` | Wrapping grid with bounds | Moderate |
| 17 | Architecture | `architecture-beta` | Force-directed (custom) | Complex |
| 18 | Ishikawa | `---\nconfig:\n  fishbone` | Spine + angled bones (trig) | Moderate |
| 19 | Packet | `packet-beta` | Bit-field grid | Trivial |
| 20 | Quadrant | `quadrantChart` | 2D cartesian | Trivial |
| 21 | Radar | `radar-beta` | Polar grid | Moderate |
| 22 | Treemap | `treemap-beta` | Hierarchical rectangular partition | Moderate |
| 23 | TreeView | `treeView` | Depth-first vertical | Trivial |
| 24 | User Journey | `journey` | Actor + task timeline | Moderate |
| 25 | Venn | `venn-beta` | Set overlap positioning | Moderate |
| 26 | Info | `info` | Static text (version) | Trivial |

---

## Workspace Layout

```
rusty-mermaid/
├── Cargo.toml
├── plan.md
├── crates/
│   ├── core/                          # rusty-mermaid-core
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── types.rs               # Point, BBox, Color, Direction
│   │       ├── style.rs               # Style, TextStyle, FontWeight
│   │       ├── scene.rs               # Scene, Primitive, PathSegment, Transform
│   │       ├── shape.rs               # Shape enum (#[non_exhaustive])
│   │       ├── curve.rs               # CurveType enum + bezier math
│   │       ├── marker.rs              # MarkerType enum (arrowheads)
│   │       ├── geometry.rs            # intersection (rect, circle, ellipse, polygon)
│   │       ├── text.rs                # TextMeasure trait + SimpleTextMeasure
│   │       └── renderer.rs            # Renderer trait
│   │
│   ├── graph/                         # rusty-mermaid-graph
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── id.rs                  # NodeId, EdgeId
│   │       ├── graph.rs               # Graph<N, E>
│   │       ├── compound.rs            # parent/children hierarchy
│   │       └── traversal.rs           # DFS, BFS, topo sort, pre/post order
│   │
│   ├── dagre/                         # rusty-mermaid-dagre
│   │   └── src/
│   │       ├── lib.rs                 # pub fn layout<T: TextMeasure>(...) → Scene
│   │       ├── config.rs              # DagreConfig
│   │       ├── pipeline.rs            # 26-step orchestrator
│   │       ├── labels.rs              # NodeLabel, EdgeLabel, Rank, Order, etc.
│   │       ├── util.rs
│   │       ├── acyclic/
│   │       │   ├── mod.rs
│   │       │   ├── dfs_fas.rs
│   │       │   └── greedy_fas.rs
│   │       ├── rank/
│   │       │   ├── mod.rs
│   │       │   ├── longest_path.rs
│   │       │   ├── feasible_tree.rs
│   │       │   └── network_simplex.rs
│   │       ├── order/
│   │       │   ├── mod.rs
│   │       │   ├── init_order.rs
│   │       │   ├── barycenter.rs
│   │       │   ├── cross_count.rs
│   │       │   ├── sort_subgraph.rs
│   │       │   ├── resolve_conflicts.rs
│   │       │   └── constraints.rs
│   │       ├── position/
│   │       │   ├── mod.rs
│   │       │   ├── bk.rs
│   │       │   └── y_coords.rs
│   │       ├── normalize.rs
│   │       ├── nesting.rs
│   │       ├── border_segments.rs
│   │       ├── parent_dummy_chains.rs
│   │       ├── coord_system.rs
│   │       └── self_edges.rs
│   │
│   ├── diagrams/                      # rusty-mermaid-diagrams
│   │   └── src/
│   │       ├── lib.rs                 # DiagramKind, detect(), render_to_scene()
│   │       ├── common/
│   │       │   ├── mod.rs             # shared parsing utils
│   │       │   ├── tokens.rs          # whitespace, identifiers, strings
│   │       │   ├── styling.rs         # classDef, style, class statements
│   │       │   └── error.rs           # ParseError with span
│   │       │
│   │       │  # ─── graph-based (feature: dagre) ───
│   │       ├── flowchart/
│   │       │   ├── mod.rs             # parse() → IR, to_scene() → Scene
│   │       │   ├── ir.rs              # FlowDiagram, FlowVertex, FlowEdge, FlowSubGraph
│   │       │   ├── parser.rs          # winnow parser
│   │       │   └── bridge.rs          # IR → Graph<NodeLabel, EdgeLabel> → dagre
│   │       ├── state/
│   │       │   ├── mod.rs
│   │       │   ├── ir.rs
│   │       │   ├── parser.rs
│   │       │   └── bridge.rs
│   │       ├── class/
│   │       │   └── ...
│   │       ├── er/
│   │       │   └── ...
│   │       ├── requirement/
│   │       │   └── ...
│   │       ├── mindmap/
│   │       │   └── ...
│   │       │
│   │       │  # ─── non-graph (custom layout) ───
│   │       ├── sequence/
│   │       │   ├── mod.rs             # parse() → IR, to_scene() → Scene
│   │       │   ├── ir.rs              # actors, messages, loops, notes
│   │       │   ├── parser.rs
│   │       │   └── layout.rs          # sequential positioning → Scene
│   │       ├── gantt/
│   │       │   ├── mod.rs
│   │       │   ├── ir.rs
│   │       │   ├── parser.rs
│   │       │   └── layout.rs          # time-axis positioning → Scene
│   │       ├── pie/
│   │       │   ├── mod.rs
│   │       │   ├── ir.rs
│   │       │   ├── parser.rs
│   │       │   └── layout.rs          # arc computation → Scene
│   │       ├── ... (19 more)
│   │       │
│   │       └── info/
│   │           └── mod.rs             # trivial: just returns Scene with text
│   │
│   └── svg/                           # rusty-mermaid-svg
│       └── src/
│           ├── lib.rs                 # impl Renderer for SvgRenderer
│           ├── document.rs            # XML builder, <svg> wrapper
│           ├── primitive.rs           # Primitive → SVG element dispatch
│           ├── path.rs                # PathSegment → d-string
│           ├── markers.rs             # MarkerType → <marker> defs
│           └── style.rs              # Style → CSS attributes
│
├── tests/
│   ├── golden/                        # input files + expected JSON positions
│   ├── dagre_compat.rs                # layout vs JS dagre
│   └── e2e.rs                         # text → Scene → SVG
│
└── examples/
    ├── flowchart.rs
    └── all_diagrams.rs
```

---

## `rusty-mermaid-core` — Shared Foundation

Zero dependencies. Everything flows through here.

### Types

```rust
// types.rs
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point { pub x: f64, pub y: f64 }

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BBox { pub x: f64, pub y: f64, pub width: f64, pub height: f64 }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Direction { TB, BT, LR, RL }
```

### Style

```rust
// style.rs
#[derive(Debug, Clone, Default)]
pub struct Style {
    pub fill: Option<Color>,
    pub stroke: Option<Color>,
    pub stroke_width: Option<f64>,
    pub stroke_dasharray: Option<Vec<f64>>,
    pub opacity: Option<f64>,
    pub css_classes: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct TextStyle {
    pub font_size: f64,
    pub font_family: String,
    pub fill: Option<Color>,
    pub font_weight: FontWeight,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Color { pub r: u8, pub g: u8, pub b: u8, pub a: u8 }
```

### Scene: Drawing Primitives IR

```rust
// scene.rs — the contract between layout and rendering
pub struct Scene {
    pub width: f64,
    pub height: f64,
    pub primitives: Vec<Primitive>,
}

pub enum Primitive {
    Rect { bbox: BBox, rx: f64, ry: f64, style: Style },
    Circle { center: Point, radius: f64, style: Style },
    Ellipse { center: Point, rx: f64, ry: f64, style: Style },
    Path {
        segments: Vec<PathSegment>,
        style: Style,
        marker_start: Option<MarkerType>,
        marker_end: Option<MarkerType>,
    },
    Text { position: Point, content: String, anchor: TextAnchor, style: TextStyle },
    Polygon { points: Vec<Point>, style: Style },
    Group { transform: Transform, children: Vec<Primitive> },
    Arc { center: Point, inner_r: f64, outer_r: f64,
          start_angle: f64, end_angle: f64, style: Style },
}

pub enum PathSegment {
    MoveTo(Point),
    LineTo(Point),
    CubicTo { cp1: Point, cp2: Point, to: Point },
    QuadTo { cp: Point, to: Point },
    ArcTo { rx: f64, ry: f64, rotation: f64, large_arc: bool, sweep: bool, to: Point },
    Close,
}

pub enum Transform {
    Translate(f64, f64),
    Scale(f64, f64),
    Rotate { degrees: f64, cx: f64, cy: f64 },
    Identity,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextAnchor { Start, Middle, End }
```

### Shape, Curve, Marker Enums

```rust
// shape.rs
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Shape {
    // Flowchart
    Rect, RoundedRect, Stadium, Subroutine, Cylinder,
    Circle, DoubleCircle, Diamond, Hexagon,
    Parallelogram, ParallelogramAlt, Trapezoid, TrapezoidAlt,
    // State
    StateStart, StateEnd, ForkJoin, Choice,
    // Class/ER
    ClassBox, ErEntity,
    // Generic
    Note, Cloud, Document,
}

// curve.rs
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum CurveType {
    #[default] Linear,
    Basis, Cardinal, MonotoneX, MonotoneY,
    CatmullRom, Natural, Step, StepBefore, StepAfter,
    BumpX, BumpY, Rounded,
}

// marker.rs
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MarkerType {
    ArrowPoint, ArrowBarb, ArrowOpen,
    Circle, Cross,
    Aggregation, Composition, Dependency,
}
```

### Geometry (intersection math)

```rust
// geometry.rs — used by dagre (edge clipping) and any diagram needing intersection
pub fn intersect_rect(bbox: &BBox, point: Point) -> Point;
pub fn intersect_circle(center: Point, radius: f64, point: Point) -> Point;
pub fn intersect_ellipse(center: Point, rx: f64, ry: f64, point: Point) -> Point;
pub fn intersect_polygon(vertices: &[Point], center: Point, target: Point) -> Point;
```

### Curve Math (control point computation)

```rust
// curve.rs (continued)
/// Convert a sequence of points + curve type → Vec<PathSegment>
/// This is pure math — no rendering. Both SVG and gpui use the segments.
pub fn interpolate(points: &[Point], curve: CurveType) -> Vec<PathSegment>;
```

### Traits

```rust
// renderer.rs
pub trait Renderer {
    type Output;
    fn render(&self, scene: &Scene) -> Self::Output;
}

// text.rs
pub trait TextMeasure {
    fn measure(&self, text: &str, style: &TextStyle) -> (f64, f64);
}

pub struct SimpleTextMeasure { pub avg_char_width: f64 }
```

---

## `rusty-mermaid-graph`

Unchanged from prior plan. `Graph<N, E>` with compound support + traversals.

---

## `rusty-mermaid-dagre`

Sugiyama layout. 26-step pipeline. Returns `Scene`.

Internal types (`Rank`, `Order`, `Weight`, `MinLen`, `DummyKind`, `NodeLabel`,
`EdgeLabel`) are `pub(crate)` — not part of the public API.

Public API:
```rust
pub fn layout<T: TextMeasure>(
    graph: &mut Graph<NodeLabel, EdgeLabel>,
    config: &DagreConfig,
    text: &T,
) -> Scene;

pub struct DagreConfig { ... }  // re-exports Direction from core
pub struct NodeLabel { ... }    // user constructs these
pub struct EdgeLabel { ... }
```

---

## `rusty-mermaid-diagrams`

The integration crate. Each diagram is a feature-gated module.

```rust
// lib.rs
#[cfg(feature = "flowchart")] pub mod flowchart;
#[cfg(feature = "state")]     pub mod state;
#[cfg(feature = "class")]     pub mod class;
#[cfg(feature = "er")]        pub mod er;
#[cfg(feature = "requirement")] pub mod requirement;
#[cfg(feature = "mindmap")]   pub mod mindmap;
#[cfg(feature = "sequence")]  pub mod sequence;
#[cfg(feature = "gantt")]     pub mod gantt;
#[cfg(feature = "pie")]       pub mod pie;
// ... all 26

pub mod common;  // always compiled: shared parsing, error types

#[non_exhaustive]
pub enum DiagramKind {
    Flowchart, State, Class, Er, Requirement, Mindmap,
    Sequence, Gantt, Pie, Sankey, XyChart, Timeline,
    Git, Kanban, Block, C4, Architecture, Ishikawa,
    Packet, Quadrant, Radar, Treemap, TreeView,
    UserJourney, Venn, Info,
}

/// Detect diagram type from first line.
pub fn detect(input: &str) -> Option<DiagramKind>;

/// Unified entry: parse + layout → Scene.
pub fn to_scene(input: &str) -> Result<Scene, DiagramError>;
```

Each module's contract:
```rust
// e.g. diagrams/flowchart/mod.rs
pub fn parse(input: &str) -> Result<FlowDiagram, ParseError>;
pub fn to_scene(diagram: &FlowDiagram, config: &DagreConfig) -> Scene;
```

### Graph-Based Module Structure (6 types)

```
flowchart/
  mod.rs       — pub fn parse(), pub fn to_scene()
  ir.rs        — FlowDiagram, FlowVertex, FlowEdge, FlowSubGraph
  parser.rs    — winnow combinators
  bridge.rs    — IR → Graph<NodeLabel, EdgeLabel>, calls dagre::layout()
```

`bridge.rs` is where IR maps to graph + dagre. This code is per-diagram because
each diagram type has different shape mappings, edge semantics, and compound
node rules. But the bridge pattern is the same across all 6 — call
`dagre::layout()` and get back a `Scene`.

**Label parsing**: node labels can contain inline HTML for text decoration:
`<br/>` (line break), `<b>`, `<i>`, `<code>`, `<u>`, `<s>`, `<sub>`, `<sup>`.
The parser must preserve these in the IR. The text measurer strips tags when
computing width but counts `<br/>` for multi-line height. The SVG renderer
maps tags to `<tspan>` attributes.

### Non-Graph Module Structure (20 types)

```
pie/
  mod.rs       — pub fn parse(), pub fn to_scene()
  ir.rs        — PieChart { title, sections: Vec<(String, f64)> }
  parser.rs    — winnow combinators
  layout.rs    — compute arcs → Vec<Primitive> → Scene
```

`layout.rs` does the diagram-specific positioning math and directly builds
`Scene` from `core::Primitive`. No graph, no dagre.

---

## `rusty-mermaid-svg`

Thin crate. Walks `Scene.primitives`, emits SVG XML.

```rust
pub struct SvgRenderer { pub theme: Option<Theme> }

impl Renderer for SvgRenderer {
    type Output = String;
    fn render(&self, scene: &Scene) -> String { ... }
}
```

Pattern-matches on each `Primitive` variant. ~800 lines total.
Trivial for a `gpui` crate to do the same with GPU primitives.

---

## What's Shared vs Unique — Zero Repetition

| Code | Lives in | Used by |
|------|----------|---------|
| `Point`, `BBox`, `Style`, `Color` | core | everything |
| `Scene`, `Primitive`, `PathSegment` | core | all layouts → all renderers |
| `Shape`, `CurveType`, `MarkerType` | core | parsers, dagre, renderers |
| Intersection math | core/geometry | dagre, any custom layout |
| Curve interpolation (→ PathSegment) | core/curve | dagre, custom layouts |
| `TextMeasure` trait | core/text | dagre, custom layouts |
| `Renderer` trait | core/renderer | svg, gpui |
| `Graph<N,E>` + traversals | graph | dagre, graph-based diagrams |
| Sugiyama algorithm | dagre | 6 graph-based diagrams |
| Parsing utilities | diagrams/common | all 26 parsers |
| Per-diagram grammar | diagrams/xxx/parser | that diagram only |
| Per-diagram IR | diagrams/xxx/ir | that diagram only |
| Per-diagram layout | diagrams/xxx/bridge or layout | that diagram only |
| Primitive → SVG | svg | SVG output |
| Primitive → gpui | gpui (future) | gpui output |

**Nothing is duplicated.** Shared math is in `core`. Shared graph ops are in
`graph`. Shared layout algorithm is in `dagre`. Per-diagram logic stays
per-diagram. Rendering backends share the `Scene` contract.

---

## Testing Strategy

Full details in `TESTING.md` (gitignored, lives alongside this plan).

**Six layers**: unit tests → property tests (proptest) → golden tests
→ fuzz tests (nightly, `cargo-fuzz`) → visual tests (human inspection) → integration tests (e2e).

### Golden tests — .mmd as single source of truth

```
tests/golden/
├── mmd/              ← 18 hand-written .mmd files (source of truth)
├── expected/         ← derived JSON with dagre positions (node x/y/rank/order, edge points)
└── generate.js       ← reads mmd/ → parses → dagre layout → writes expected/
```

Flow: `.mmd` → `generate.js` (parses + dagre layout) → `expected/*.json`.
Rust tests: `.mmd` → winnow parse → dagre layout → compare against `expected/*.json`.
Same input, two implementations, ±1.0 pixel tolerance.

To regenerate: `cd tests/golden && npm install @dagrejs/dagre @dagrejs/graphlib && node generate.js`

### Other layers

- **Fuzz targets**: `fuzz/fuzz_targets/` — structured fuzzing with `Arbitrary` derive
- **Visual gallery**: `tests/visual/gallery.html` — serves SVGs for human review
- **Side-by-side**: our SVG vs mermaid.js SVG for same `.mmd` input (Phase 3)

Testing is integrated into every phase below — each implementation item includes
its tests, and every phase ends with a code review checkpoint.

---

## Implementation Sequence

Each item: implement → test → diff review → LGTM → commit.
No batching multiple items into one commit.

```
Phase 0: core + graph                        ≈  800 lines   Week 1
  [x] 0.1  core: types (Point, BBox, Color) + unit tests
  [x] 0.1r ── code review + LGTM ──
  [x] 0.2  core: style (Style, TextStyle) + unit tests
  [x] 0.2r ── code review + LGTM ──
  [x] 0.3  core: scene (Scene, Primitive, PathSegment, Transform) + unit tests
  [x] 0.3r ── code review + LGTM ──
  [x] 0.4  core: shape, curve, marker enums + unit tests
  [x] 0.5  core: geometry (intersection functions) + unit tests
  [x] 0.6  core: curve interpolation → PathSegment + unit tests
  [x] 0.7  core: Renderer trait, TextMeasure trait + SimpleTextMeasure + unit tests
  [x] 0.8  graph: NodeId, EdgeId, IdGen + unit tests
  [x] 0.9  graph: Graph<N, E> (add/remove/query/compound) + unit tests
  [x] 0.10 graph: traversal (DFS, BFS, topo, pre/post) + unit tests
  [x] 0.11 fuzz: enable fuzz_graph_ops target
  [x] 0.11r ── code review + LGTM ──

Phase 1a: dagre — acyclic + rank             ≈  600 lines   Week 2
  [x] 1.1  config.rs + labels.rs + util.rs
  [x] 1.2  acyclic: dfs_fas + greedy_fas + unit tests + proptest (acyclic_produces_dag)
  [x] 1.3  rank: longest_path + unit tests
  [x] 1.4  rank: feasible_tree + unit tests
  [x] 1.5  rank: network_simplex + unit tests + proptest (rank_respects_minlen)
  [x] 1.5r ── code review + LGTM ──

Phase 1b: dagre — normalize + nesting        ≈  400 lines   Week 3
  [x] 1.7  normalize.rs + unit tests + proptest (normalize_all_unit_length)
  [x] 1.8  nesting.rs + unit tests
  [x] 1.9  border_segments.rs + unit tests
  [x] 1.10 parent_dummy_chains.rs + unit tests
  [x] 1.10r ── code review + LGTM ──

Phase 1c: dagre — order                      ≈  500 lines   Week 3-4
  [x] 1.12 init_order + unit tests
  [x] 1.13 barycenter + cross_count + unit tests
  [x] 1.14 resolve_conflicts, sort_subgraph, constraints + unit tests
  [x] 1.15 order/mod.rs (sweep orchestrator) + proptest (order_reduces_crossings)
  [x] 1.15r ── code review + LGTM ──

Phase 1d: dagre — position + pipeline        ≈  800 lines   Week 4-5
  [x] 1.16 position/y_coords + bk (Brandes-Köpf) + unit tests
  [x] 1.17 coord_system, self_edges + unit tests
  [x] 1.18 pipeline.rs + proptest (layout_no_overlap_in_rank)
  [ ] 1.19 fuzz: enable fuzz_dagre_layout target
  [x] 1.19r ── code review + LGTM ──

Phase 2: diagrams — flowchart + state        ≈ 1400 lines   Week 5-6
  [x] 2.1  common/ (tokens, styling, error) + unit tests
  [x] 2.2  flowchart/ir.rs + unit tests
  [x] 2.3  flowchart/parser.rs (winnow) + unit tests + proptest (parse_never_panics)
  [x] 2.4  flowchart/bridge.rs + unit tests
  [x] 2.5  state/ir.rs + unit tests
  [x] 2.6  state/parser.rs (winnow) + unit tests
  [x] 2.7  state/bridge.rs + unit tests
  [x] 2.8  lib.rs (detect, to_scene) + unit tests
  [x] 2.9  golden tests: all 25 .mmd → winnow parse + dagre → compare vs expected/*.json
  [ ] 2.10 fuzz: enable fuzz_flowchart_parse + add fuzz_state_parse
  [x] 2.10r ── code review + LGTM ──

Phase 3: svg + visual verification           (done)
  [x] 3.1  svg crate: document, primitive, path, markers, style
  [x] 3.2  end-to-end: mermaid text → Scene → SVG
  [x] 3.3  visual gallery: all golden .mmd → SVG + HTML index

Phase 5: flowchart feature parity
  Each item: implement → test → gallery .mmd → diff review → commit.

  5a — Shape rendering (parser captures Shape; renderer draws all as rounded rects)
  [x] 5.1  Propagate Shape from IR through bridge to renderer
            - FlowVertex.shape already in IR
            - bridge: pass shape to NodeLayout
            - renderer: dispatch on shape
            + test: all_shapes.mmd renders distinct shapes in gallery
  [x] 5.1r ── visual review ──
  [x] 5.2  Render diamond (rhombus path)
            + gallery: diamond_flow.mmd should show actual diamonds
  [x] 5.3  Render stadium (left/right semicircle caps)
  [x] 5.4  Render circle and double-circle
  [x] 5.5  Render hexagon (6-point polygon)
  [x] 5.6  Render parallelogram + alt (skewed rects)
  [x] 5.7  Render trapezoid + alt (angled top/bottom)
  [x] 5.8  Render cylinder (elliptical top/bottom caps)
  [x] 5.9  Render subroutine (double vertical bars)
  [x] 5.10 Render asymmetric shape (flag/banner)
  [x] 5.10r ── visual review: all_shapes.mmd shows all 14 shapes ──

  5b — Edge rendering
  [x] 5.11 Render arrow markers: --o (circle end), --x (cross end)
            - markers exist in core; wire them from IR stroke/arrow fields
            + gallery: arrows.mmd shows all arrow types
  [x] 5.12 Render thick edges (stroke-width > default)
  [x] 5.13 Render open edges (--- no arrowhead)
  [x] 5.14 Render bidirectional arrows (<-->)
  [ ] 5.14r ── visual review ──

  5c — Subgraph direction
  [ ] 5.15 Wire subgraph `direction LR/TB/etc` to dagre layout
            - parser already captures direction in FlowSubGraph
            - bridge needs per-subgraph dagre config (mermaid does
              independent dagre layout per subgraph — evaluate if
              we can approximate with coord_system or need nested layout)
            + gallery: subgraph_direction.mmd shows LR inside TD
  [ ] 5.15r ── visual review ──

  5d — Style application
  [x] 5.16 Apply classDef fill/stroke/stroke-width to node rendering
            - classDef + class statements already parsed into IR
            - resolve class → style map, merge onto node Style
            + gallery: style_classdef.mmd shows colored nodes
  [x] 5.17 Apply inline style statements (`style A fill:#f9f`)
            + gallery: style_inline.mmd shows styled nodes
  [x] 5.18 Apply :::className inline syntax
  [x] 5.19 linkStyle for edge coloring (parse + apply)
            + gallery: new edge_styles.mmd
  [ ] 5.19r ── visual review ──

  5e — Remaining flowchart gaps
  [x] 5.20 Edge label positioning: place at path midpoint with
            background rect (match mermaid's label-on-edge look)
  [ ] 5.21 Markdown in labels (`**bold**`, `_italic_`) — parse to
            inline spans, render as tspan with font-weight/style
  [ ] 5.22 Multi-line labels: support `<br/>` in node text → multi-
            line tspan rendering in SVG
  [ ] 5.22r ── visual review ──

Phase 6: state diagram feature parity
  Each item: implement → test → gallery .mmd → diff review → commit.

  6a — Styling
  [ ] 6.1  Parse classDef / class / style in state grammar
  [ ] 6.2  Apply styles to state rendering (fill, stroke, stroke-width)
            + gallery: new state_styled.mmd
  [ ] 6.2r ── visual review ──

  6b — Transition labels
  [ ] 6.3  Parse guard conditions `A --> B : event [guard]`
            - currently everything after `:` is a flat label string
            - parse into structured { event, guard, action } if present
  [ ] 6.4  Parse action/effect syntax `A --> B : event / action`
  [ ] 6.5  Render structured transition labels
            + gallery: new state_guards.mmd
  [ ] 6.5r ── visual review ──

  6c — Missing state types
  [ ] 6.6  History states: parse `<<history>>`, add StateKind::History
            - render as circle with "H" label
            + gallery: new state_history.mmd
  [ ] 6.7  Note positions: support `note top of` / `note bottom of`
            (currently only left/right)
  [ ] 6.7r ── visual review ──

  6d — Concurrent regions
  [ ] 6.8  Render concurrent region dividers (`--`) inside composites
            - parser already handles `--`, needs visual rendering
            + gallery: new state_concurrent.mmd
  [ ] 6.8r ── visual review ──

Phase 7: text measurement
  [ ] 7.1  Font metrics table: embed width table for default monospace
            font (Intel One Mono or fallback). Per-glyph widths for
            ASCII, average for non-ASCII. Replace char-counting.
  [ ] 7.2  Multi-line measurement: properly handle line breaks,
            return (max_line_width, n_lines * line_height)
  [ ] 7.3  HTML-aware measurement: strip tags but respect <br/> for
            height; ignore <b>/<i> width differences (monospace)
  [ ] 7.4  Validate: compare layout positions before/after on all
            gallery .mmd files; ensure no regressions > 2px
  [ ] 7.4r ── visual review ──

Phase 8: curve interpolation (remaining)
  [ ] 8.1  Cardinal spline interpolation
  [ ] 8.2  CatmullRom interpolation
  [ ] 8.3  MonotoneX / MonotoneY interpolation
  [ ] 8.4  Natural cubic spline interpolation
  [ ] 8.4r ── visual review ──

Phase 9: remaining diagrams + gpui                    (discuss plan separately)
  [ ] 9.x  class, ER, sequence, gantt, pie, mindmap, etc.
  [ ] 9.y  gpui crate: impl Renderer for GpuiRenderer
```

---

## Open Questions

1. **Text measurement**: Currently char-counting heuristic. Phase 7 will
   embed per-glyph width tables for the default monospace font. Callers
   with real font access (gpui, browser/WASM) provide their own
   `TextMeasure` impl.

2. **WASM**: Keep `std` for now. Core and graph have no inherent `std` dep
   beyond `HashMap`. Add `no_std` + `hashbrown` feature flag later.

3. **ELK**: Would be a new crate producing `Scene` from `Graph`. Zero changes to
   existing crates — the `Scene` contract handles it.

4. **Incremental layout**: Defer to v2. Keep dagre data structures amenable to
   partial recomputation.

5. **Force-directed layout** (architecture, mindmap): Currently listed as dagre
   but mermaid actually uses cytoscape/fcose. May need a separate
   `force-layout` crate or feature-gated physics simulation in diagrams.

6. **Subgraph direction**: Mermaid runs independent dagre layouts per subgraph
   with its own direction. Evaluate whether we can approximate this within a
   single dagre pass or need nested layout calls (Phase 5.15).
