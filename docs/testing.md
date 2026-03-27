# Testing Strategy

## 1. Testing Philosophy

Four principles drive how we test rusty-mermaid:

**Defense in depth.** No single test layer catches every class of bug. Parser
unit tests catch syntax regressions; property tests find edge cases humans miss;
fuzz targets find crash bugs from malformed input; golden regressions catch
visual drift. Each layer exists because the others have blind spots.

**No second-class citizens.** Every diagram type -- all 25 of them -- gets the
same treatment: parser tests, IR assertions, property tests, fuzz targets, scene
invariants, and SVG golden regression. The test infrastructure auto-discovers
golden files, so adding a new `.mmd` file automatically enrolls it in multiple
test layers.

**Deterministic by design.** BTreeMap and BTreeSet throughout (not HashMap) for
deterministic iteration order. No randomness in production code. Seeded layouts.
The only source of non-determinism is in proptest/fuzz, which is the point.

**1,793 tests, 0 unsafe, 0 production unwraps.** The codebase uses `Result<T>`
everywhere in production paths. Every `.unwrap()` lives in test code. The only
panics that can occur are logic bugs, not missing error handling.


## 2. Test Pyramid

Six layers, from fastest/narrowest to slowest/broadest.

```
                       ┌─────────────┐
                       │   Fuzz (28) │  Finds crashes from malformed input
                      ─┤             ├─
                     / └─────────────┘ \
                    /   Property (60)   \  Random inputs, invariant checks
                   ─────────────────────
                  /  SVG Golden (297)    \  Byte-exact rendering regression
                 ─────────────────────────
                / Scene Invariants (4×N)  \  Structural sanity across all goldens
               ─────────────────────────────
              /    IR Assertions (141)      \  Parsed structure matches expectations
             ─────────────────────────────────
            /     Parser Unit Tests (400+)    \  Individual functions, edge cases
           ─────────────────────────────────────
```


### Layer 1: Parser Unit Tests (400+ tests)

**What they test.** Individual parser functions: can we correctly turn mermaid
syntax into our IR? Each diagram type has `#[test]` functions inside its
`parser.rs` or corresponding test module, exercising normal cases, edge cases
(empty input, special characters, deeply nested structures), and error paths.

**Why they exist.** Parsers are the entry point for all user input. A regression
here means valid diagrams stop working. These are the cheapest tests to write and
the fastest to run -- the inner feedback loop during development.

**What they catch.** Syntax regressions, incorrect tokenization, missing edge
cases in grammar rules, off-by-one errors in position tracking.

**Example.** Parse `"flowchart LR\n  A --> B"` and verify 2 vertices, 1 edge,
correct source/destination IDs, left-to-right direction.


### Layer 2: IR Assertions (141 tests)

**What they test.** After parsing, the IR (intermediate representation) should
contain the right structure: correct node counts, edge counts, relationship
types, shapes, labels, field values, and connectivity. These tests parse golden
`.mmd` files and assert specific properties of the resulting IR.

**Why they exist.** A parser can produce *something* without producing the
*right thing*. Unit tests check that the parser doesn't crash; IR assertions
check that it produces semantically correct output. They catch bugs where the
parser silently drops a node, misidentifies a shape, or loses an edge label.

**What they catch.** Semantic parsing bugs: wrong shape assigned, missing
relationships, incorrect cardinality, dropped annotations, wrong edge types.

**Example.** Parse the `all_shapes` golden file and assert exactly 14 vertices,
13 edges, and that each vertex has the correct `Shape` variant (Rect,
RoundedRect, Stadium, Diamond, Circle, Hexagon, etc.).


### Layer 3: Scene Invariants (4 tests, each covering all golden files)

**What they test.** Every golden `.mmd` file is rendered to a `Scene`, then
checked for four universal invariants:

1. **Positive dimensions** -- scene width and height are > 0
2. **Non-empty** -- scene contains at least one element
3. **Finite coordinates** -- no NaN or Infinity anywhere (rects, text positions,
   path points, circle centers, polygon vertices, ellipse centers, group children)
4. **Reasonable size** -- no dimension exceeds 6000px (catches layout explosions)

**Why they exist.** These catch problems that don't require pixel-level
comparison. A layout algorithm that produces NaN coordinates, a rendering pass
that outputs an empty scene, or a force layout that explodes to infinity -- all
caught here without needing byte-exact SVG baselines.

**What they catch.** NaN propagation from arithmetic errors, layout explosions
(infinite coordinates), empty output from missing render branches, degenerate
dimensions from edge cases in spacing/padding logic.

**How discovery works.** The `all_golden_files()` function walks
`tests/golden/mmd/*/` and collects every `.mmd` file. Adding a new golden file
automatically enrolls it in all four invariant checks.


### Layer 4: SVG Golden Regression (297 byte-exact comparisons)

**What they test.** Each golden `.mmd` file is rendered through the full pipeline
(parse -> IR -> layout -> Scene -> SVG) and compared byte-for-byte against a
stored baseline SVG. Any difference -- a moved coordinate, a changed color, a
reordered attribute -- fails the test.

**Why they exist.** This is the most sensitive test layer. It catches *any*
rendering change, whether intentional or accidental. When you modify layout
spacing, font metrics, edge routing, or style resolution, these tests tell you
exactly which diagrams changed and where the first diff occurs.

**What they catch.** Visual regressions: shifted nodes, changed colors, altered
spacing, reordered elements, modified text positioning, wrong marker placement.

**Workflow.** When a rendering change is intentional:
```sh
UPDATE_GOLDEN_SVG=1 cargo test
```
This regenerates all baselines. The diff in version control shows exactly what
changed. Re-run without the env var to verify the new baselines are stable.

**Discovery mechanism.** Same as scene invariants -- `renderable_entries()` walks
`tests/golden/mmd/*/`, collects all `.mmd` files, tries to parse and render
each, and includes it if successful. The corresponding SVG baseline lives at
`tests/golden/svg/{type}/{name}.svg`.


### Layer 5: Property Tests (25 diagram + 6 dagre + 29 curve/geometry/viewport)

**What they test.** Randomly generated inputs tested against invariants.
For diagram types: generate random valid-ish mermaid input (random labels,
random node counts, random values) and verify the Scene has positive dimensions,
finite coordinates, and no panics. For dagre: random graphs with random
parameters, checking that layout produces a valid DAG, ranks respect minlen,
ranks start at zero, no overlap within ranks. For geometry: curve interpolation,
arc segments, viewport transforms (roundtrip, zoom preserves cursor point,
pan-then-unpan is identity).

**Why they exist.** Humans write tests for the cases they think of. Property
tests explore the cases they don't. The treemap degenerate rectangle bug was
found by proptest in under one second. The viewport roundtrip precision issue
was caught by random coordinate generation.

**What they catch.** Edge cases in layout algorithms (degenerate inputs, extreme
values), arithmetic precision bugs, assumption violations that only surface with
unusual combinations of parameters.

**Example.** Generate a random pie chart with 2-6 slices, random labels (1-15
alphanumeric chars), random values (1-1000). Parse, render to Scene, verify
positive dimensions and finite coordinates. Run 20 times with different seeds.


### Layer 6: Fuzz Targets (28 targets)

**What they test.** Every parser (25 targets), the dagre layout engine, the
force layout engine, and the graph operations module -- all under
AFL/libFuzzer. The parser targets feed arbitrary bytes through UTF-8 validation,
then into the parser. The dagre target uses `Arbitrary` to generate structured
graphs with random topology and layout parameters.

**Why they exist.** Fuzzers find crash bugs that no other technique reliably
catches. The UTF-8 char boundary panic -- where `ParseError::new()` sliced a
string at a non-char boundary -- was found by the fuzzer in under 10 seconds.
These targets enforce the contract: **parsers must never panic on any input,
only return Ok or Err.**

**What they catch.** Panics from malformed input, integer overflow, slice
boundary violations, infinite loops, stack overflow from deeply nested input.

**Targets.**
- 25 parser targets: one per diagram type (`flowchart_parse.rs`, `state_parse.rs`, etc.)
- `dagre_layout.rs`: structured fuzzing with `Arbitrary`-derived graph topology
- `force_layout.rs`: force-directed layout with random node counts and edges
- `graph_ops.rs`: graph data structure operations


## 3. Golden File Infrastructure

The golden file system is the backbone of regression testing. It uses a
convention-over-configuration approach with auto-discovery.

### Directory layout

```
tests/golden/
├── mmd/                     ← source files (human-authored)
│   ├── flowchart/
│   │   ├── single_node.mmd
│   │   ├── linear_3.mmd
│   │   ├── diamond.mmd
│   │   └── ...
│   ├── state/
│   ├── class/
│   └── {type}/
│       └── {name}.mmd
│
└── svg/                     ← baselines (generated, tracked in git)
    ├── flowchart/
    │   ├── single_node.svg
    │   ├── linear_3.svg
    │   └── ...
    └── {type}/
        └── {name}.svg
```

### Auto-discovery

Three test files independently walk `tests/golden/mmd/`:

| Test file | What it discovers | What it checks |
|-----------|------------------|----------------|
| `parse_golden_mmd.rs` | Every `.mmd` file via macros per type | Parser produces valid IR |
| `scene_invariants.rs` | Every `.mmd` file via directory walk | Scene dimensions, emptiness, coordinate finiteness, size bounds |
| `svg_golden.rs` | Every renderable `.mmd` via directory walk | Byte-exact SVG match against stored baseline |

The WASM gallery's `build.rs` also auto-discovers golden files to generate a
live rendering gallery.

**Adding a new test case is a single step:** drop a `.mmd` file in the right
subdirectory. The parse smoke test requires a macro invocation (one line), but
scene invariants and SVG golden tests pick it up automatically on the next run.


## 4. What Each Layer Catches

| Bug class | Parser unit | IR assertion | Scene invariant | SVG golden | Property test | Fuzz |
|-----------|:-----------:|:------------:|:---------------:|:----------:|:-------------:|:----:|
| Syntax regression | **1st** | | | | | |
| Wrong shape/type parsed | | **1st** | | x | | |
| Missing node or edge | | **1st** | | x | | |
| NaN in coordinates | | | **1st** | x | x | |
| Layout explosion (Inf) | | | **1st** | x | x | |
| Empty output | | | **1st** | x | | |
| Visual drift (spacing) | | | | **1st** | | |
| Color/style regression | | | | **1st** | | |
| Marker/arrow change | | | | **1st** | | |
| Degenerate input crash | | | | | **1st** | x |
| Extreme value edge case | | | | | **1st** | |
| Malformed input panic | | | | | | **1st** |
| UTF-8 boundary crash | | | | | | **1st** |
| Integer overflow | | | | | | **1st** |
| Infinite loop | | | | | | **1st** |

**1st** = this layer typically catches the bug first.
**x** = this layer also catches it, but usually after the primary.

The key insight: no single layer covers all rows. That is the point. Defense in
depth means accepting that each layer has blind spots, and designing the
combination to have none.


## 5. Adding Tests for a New Diagram Type

When you add a new diagram type (say `foo`), the test contract requires coverage
at every layer. This is not bureaucratic -- each layer has caught real bugs that
the others missed.

### Step 1: Golden files (5+ `.mmd` files)

Create `tests/golden/mmd/foo/` with at least 5 representative inputs:
- A minimal valid diagram (1-2 elements)
- A typical real-world diagram (5-10 elements)
- An edge case (empty labels, special characters, unicode)
- A stress case (many elements, deep nesting if applicable)
- A feature-specific case (whatever makes `foo` unique)

These files immediately enroll in scene invariants (auto-discovered) and SVG
golden regression (auto-discovered after first run generates baselines).

### Step 2: Parse golden tests (macro invocations)

Add a `parse_foo!` macro block in `parse_golden_mmd.rs` following the existing
pattern: read each `.mmd` file, parse it, assert the result is non-empty. One
macro definition, one invocation per golden file.

### Step 3: IR assertions (5+ tests)

Add tests in `ir_assertions.rs` that parse specific golden files and assert
structural properties: correct counts, correct types, correct relationships.
These are the tests that verify *semantic* correctness, not just "it didn't
crash."

### Step 4: Property test

Add a `foo_random` test in `proptest_scenes.rs`. Generate random valid-ish
`foo` diagram text using the `arb_label()`, `arb_id()`, and `arb_value()`
helpers. Assert `scene_is_valid()`.

### Step 5: Fuzz target

Create `fuzz/fuzz_targets/foo_parse.rs` following the one-file pattern: take
arbitrary bytes, validate UTF-8, call the parser, discard the result. The only
assertion is "the parser does not panic."

### Step 6: Generate SVG baselines

```sh
UPDATE_GOLDEN_SVG=1 cargo test
```

Review the generated SVGs visually. Commit them. They are now your regression
baseline.

### What you do NOT need to do

- No per-backend tests. The Scene abstraction means SVG, raster, gpui, and wgpu
  all render from the same Scene. If the Scene is correct, the backends work.
- No integration test boilerplate. Auto-discovery handles scene invariants and
  SVG regression.
- No manual test registration. The golden file convention handles it.
