# rusty-mermaid

Mermaid diagram rendering in pure Rust. Parse mermaid syntax, lay out with dagre, render to SVG/PNG/PDF/GPU.

**25 diagram types. 5 rendering backends. Zero unsafe code.**

[Gallery](https://base58ed.github.io/rusty-mermaid/gallery.html) — see all 297 rendered diagrams with source code.

## Install

```toml
[dependencies]
rusty-mermaid = { version = "0.1", features = ["svg"] }
```

No default features — pick only the backends you need:

| Feature    | Output                        | Use case              |
|------------|-------------------------------|-----------------------|
| `svg`      | `to_svg()` → SVG string       | Web, export           |
| `raster`   | `to_png()` → PNG bytes         | CLI, thumbnails       |
| `wgpu`     | Vello scene builder (WebGPU)   | Browser, native GPU   |
| `gpui`     | Canvas element (gpui/Zed)      | Zed editor            |
| `viewport` | Pan/zoom state + transforms    | Interactive apps      |

```toml
# SVG + PNG
rusty-mermaid = { version = "0.1", features = ["svg", "raster"] }

# GPU browser rendering
rusty-mermaid = { version = "0.1", features = ["wgpu"] }

# Parse only — no rendering backend
rusty-mermaid = "0.1"
```

## Usage

```rust
// Parse and render to SVG
let svg = rusty_mermaid::to_svg(
r#"flowchart LR
    A[Start] --> B[End]
"#)?;

// Dark theme
let svg = rusty_mermaid::to_svg_themed(input, &Theme::dark())?;

// Parse to Scene (backend-agnostic IR)
let scene = rusty_mermaid::render(input)?;
let kind = rusty_mermaid::detect(input); // Some(DiagramKind::Flowchart)
```

## Supported Diagrams

25 diagram types with full mermaid.js syntax parity. See the [gallery](https://base58ed.github.io/rusty-mermaid/gallery.html) for rendered examples with source code.

| Category | Types |
|----------|-------|
| **Graph** | flowchart, state, sequence, class, ER, requirement |
| **Chart** | pie, xychart, gantt, radar, quadrant, sankey |
| **Tree** | mindmap, treeview, treemap |
| **Flow** | timeline, journey, kanban, gitgraph |
| **Specialized** | C4, architecture, ishikawa, packet, block, venn |

## Architecture

```
rusty-mermaid (facade — feature-gated re-exports)
├── core       — Scene, Primitive, Theme, geometry, text measurement
├── graph      — Graph<N,E> directed multigraph with compound hierarchy
├── dagre      — Sugiyama layout engine (rank, order, position)
├── diagrams   — 25 parsers + renderers → Scene
├── svg        — Scene → SVG string
├── raster     — Scene → PNG (tiny-skia, CPU)
├── viewport   — Pan/zoom state, coordinate transforms
├── wgpu       — Scene → vello (WebGPU/Metal/Vulkan)
└── gpui       — Scene → gpui canvas (Zed editor)
```

All backends consume `&Scene` — add a diagram type once, get all backends for free.

## Themes

Built-in light and dark themes. All colors, font sizes, and stroke widths read from `Theme`.

```rust
let light = rusty_mermaid::to_svg(input)?;
let dark = rusty_mermaid::to_svg_themed(input, &Theme::dark())?;
```

## Testing

1,793 tests:

- **300 golden `.mmd` files** across all 25 diagram types
- **297 SVG goldens** — byte-exact rendering regression
- **25 property tests** — randomized scene invariants
- **28 fuzz targets** — every parser + layout engine
- **141 IR tests** — intermediate representation coverage

```sh
cargo test                    # all tests
cargo test --test svg_golden  # SVG regression only
```

## WASM Gallery

Interactive GPU-rendered gallery in the browser:

```sh
cd examples/wasm-gallery
wasm-pack build --target web
cd www && python3 -m http.server 8080
```

Supports pan/zoom (scroll + drag), dark/light toggle, all 300 diagrams.

## License

[MIT](LICENSE)
