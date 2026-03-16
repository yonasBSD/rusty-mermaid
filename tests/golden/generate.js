#!/usr/bin/env node
//
// Generates golden test expected output from .mmd files.
//
// Flow:
//   1. Read each .mmd file from mmd/
//   2. Parse with mermaid → extract graph (nodes, edges, subgraphs)
//   3. Layout with dagre → capture node positions and edge points
//   4. Write expected output to expected/<name>.json
//
// Usage:
//   cd tests/golden
//   npm install mermaid @dagrejs/dagre @dagrejs/graphlib
//   node generate.js
//
// Output format:
// {
//   "mmd": "<original text>",
//   "config": { "rankdir": "TB", ... },
//   "output": {
//     "nodes": [{ "id": "A", "x": .., "y": .., "width": .., "height": .., "rank": .., "order": .. }],
//     "edges": [{ "src": "A", "dst": "B", "points": [{"x":.., "y":..}], "x": .., "y": .. }],
//     "width": ..,
//     "height": ..
//   }
// }
//
// .mmd files are the single source of truth.
// This script derives the expected JSON — never edit JSON by hand.

const fs = require("fs");
const path = require("path");
const dagre = require("@dagrejs/dagre");
const graphlib = require("@dagrejs/graphlib");

// ── Parse mermaid flowchart syntax (minimal subset) ────────
//
// We parse just enough of the flowchart/graph syntax to extract
// nodes, edges, subgraphs, and direction. This avoids depending
// on the full mermaid package (which needs a browser/JSDOM).
//
// Supported:
//   graph TD / graph LR / graph BT / graph RL / flowchart TD / etc.
//   A[label] --> B[label]
//   A -->|label| B
//   A --> B --> C (chained)
//   subgraph id[label] ... end
//   {label} for diamond shapes
//
// Not supported (add as needed):
//   style, classDef, click, %%comments, :::class

function parseMmd(text) {
  const lines = text.split("\n").map(l => l.trim()).filter(l => l && !l.startsWith("%%"));
  const nodes = new Map(); // id → { id, width, height }
  const edges = [];
  const parents = {}; // child → parent
  const subgraphStack = [];
  let rankdir = "TB";

  // Strip HTML tags for visual width estimation
  function stripHtml(text) {
    return text.replace(/<[^>]+>/g, "");
  }

  // Count <br/> and <br> to estimate multi-line height
  function countLines(text) {
    return (text.match(/<br\s*\/?>/gi) || []).length + 1;
  }

  // Default node sizing: estimate from label, accounting for HTML
  function ensureNode(id, label, shape) {
    if (nodes.has(id)) return;
    const text = label || id;
    const visibleText = stripHtml(text);
    const lines = countLines(text);
    // Rough sizing: ~8px per char width, ~20px per line height
    const longestLine = visibleText.split(/\n/).reduce(
      (max, line) => Math.max(max, line.length), 0
    ) || visibleText.length;
    const width = Math.max(40, longestLine * 8 + 20);
    const baseHeight = shape === "diamond" ? Math.max(60, width * 0.75) : 40;
    const height = baseHeight + Math.max(0, lines - 1) * 20;
    nodes.set(id, { id, width, height, label: text });
    // If inside a subgraph, set parent
    if (subgraphStack.length > 0) {
      parents[id] = subgraphStack[subgraphStack.length - 1];
    }
  }

  // Parse node reference: A, A["label"], A[label], A{label}, A(label)
  // Quoted labels: ["text with <br/> and <i>html</i>"]
  function parseNodeRef(token) {
    // Try quoted bracket label first: id["..."]
    const quoted = token.match(/^([a-zA-Z_][\w]*)\["([^"]*)"\]/);
    if (quoted) {
      return { id: quoted[1], label: quoted[2], shape: "rect" };
    }
    // Unquoted: id[label], id{label}, id(label)
    const m = token.match(/^([a-zA-Z_][\w]*)(?:\[([^\]]*)\]|\{([^}]*)\}|\(([^)]*)\))?/);
    if (!m) return null;
    const id = m[1];
    const label = m[2] || m[3] || m[4] || id;
    const shape = m[3] !== undefined ? "diamond" : "rect";
    return { id, label, shape };
  }

  for (const line of lines) {
    // Direction line
    const dirMatch = line.match(/^(?:graph|flowchart)\s+(TD|TB|LR|RL|BT)/i);
    if (dirMatch) {
      rankdir = dirMatch[1].toUpperCase();
      if (rankdir === "TD") rankdir = "TB";
      continue;
    }

    // Subgraph: supports `subgraph id[label]`, `subgraph "multi word"`, `subgraph multi word`
    const subMatch = line.match(/^subgraph\s+(?:"([^"]+)"|(\w+)(?:\[([^\]]*)\])?\s*(.*)?)/);
    if (subMatch) {
      // Quoted name, or single-word id, or single-word id with trailing words as label
      const quotedName = subMatch[1];
      const wordId = subMatch[2];
      const bracketLabel = subMatch[3];
      const trailingWords = subMatch[4]?.trim();

      let id, label;
      if (quotedName) {
        // subgraph "Undertow HTTP Server" → id derived from first word, label is full name
        id = quotedName.replace(/\s+/g, "_");
        label = quotedName;
      } else if (bracketLabel) {
        // subgraph cluster[Group]
        id = wordId;
        label = bracketLabel;
      } else if (trailingWords) {
        // subgraph Undertow HTTP Server → id = "Undertow_HTTP_Server"
        const fullName = wordId + " " + trailingWords;
        id = fullName.replace(/\s+/g, "_");
        label = fullName;
      } else {
        // subgraph cluster
        id = wordId;
        label = wordId;
      }

      ensureNode(id, label, "rect");
      nodes.get(id).width = 0;
      nodes.get(id).height = 0;
      subgraphStack.push(id);
      continue;
    }

    if (line === "end" && subgraphStack.length > 0) {
      subgraphStack.pop();
      continue;
    }

    // Standalone node declaration (no edge): e.g. "A[label]"
    const standaloneRef = parseNodeRef(line);
    if (standaloneRef && !line.includes("-->") && !line.includes("---") && !line.includes("-.->")) {
      ensureNode(standaloneRef.id, standaloneRef.label, standaloneRef.shape);
      continue;
    }

    // Edge lines: split on arrow operators (longer patterns first)
    // Supports: -->, --->, -.->  -..->  --
    const edgeParts = line.split(/(---+>|-\.+->|-.->|-->|--+)/);
    if (edgeParts.length >= 3) {
      const nodeTokens = [];
      const edgeLabels = [];

      for (let i = 0; i < edgeParts.length; i++) {
        const part = edgeParts[i].trim();
        if (!part) continue;
        if (part.match(/^---+>$|^-\.+->$|^-.->$|^-->$|^--+$/)) {
          edgeLabels.push(null);
          continue;
        }
        // Check for edge label: |label| prefix
        const labelMatch = part.match(/^\|([^|]*)\|\s*(.*)/);
        if (labelMatch) {
          // Attach label to previous edge
          if (edgeLabels.length > 0) {
            edgeLabels[edgeLabels.length - 1] = labelMatch[1];
          }
          if (labelMatch[2]) {
            const ref = parseNodeRef(labelMatch[2]);
            if (ref) nodeTokens.push(ref);
          }
        } else {
          const ref = parseNodeRef(part);
          if (ref) nodeTokens.push(ref);
        }
      }

      for (const ref of nodeTokens) {
        ensureNode(ref.id, ref.label, ref.shape);
      }
      for (let i = 0; i < nodeTokens.length - 1; i++) {
        const edge = { src: nodeTokens[i].id, dst: nodeTokens[i + 1].id };
        const label = edgeLabels[i];
        if (label) {
          edge.labelWidth = label.length * 7 + 10;
          edge.labelHeight = 20;
        }
        edges.push(edge);
      }
    }
  }

  return { rankdir, nodes: [...nodes.values()], edges, parents };
}

// ── Layout ─────────────────────────────────────────────────

function runLayout(parsed, mmdText) {
  const config = { rankdir: parsed.rankdir, nodesep: 50, ranksep: 50 };

  const g = new graphlib.Graph({ multigraph: true, compound: true })
    .setGraph(config)
    .setDefaultNodeLabel(() => ({}))
    .setDefaultEdgeLabel(() => ({ weight: 1, minlen: 1 }));

  for (const n of parsed.nodes) {
    g.setNode(n.id, { width: n.width, height: n.height, label: n.id });
  }
  for (const e of parsed.edges) {
    g.setEdge(e.src, e.dst, {
      weight: e.weight || 1,
      minlen: e.minlen || 1,
      width: e.labelWidth || 0,
      height: e.labelHeight || 0,
    });
  }
  for (const [child, parent] of Object.entries(parsed.parents)) {
    g.setParent(child, parent);
  }

  dagre.layout(g);

  const nodes = g.nodes().map(v => {
    const n = g.node(v);
    return { id: v, x: n.x, y: n.y, width: n.width, height: n.height, rank: n.rank, order: n.order };
  });
  const edges = g.edges().map(e => {
    const edge = g.edge(e);
    return { src: e.v, dst: e.w, points: edge.points, x: edge.x, y: edge.y };
  });
  const graph = g.graph();

  return {
    mmd: mmdText,
    config,
    output: { nodes, edges, width: graph.width, height: graph.height },
  };
}

// ── Generate ───────────────────────────────────────────────

const mmdDir = path.join(__dirname, "mmd");
const outDir = path.join(__dirname, "expected");
fs.mkdirSync(outDir, { recursive: true });

const files = fs.readdirSync(mmdDir).filter(f => f.endsWith(".mmd")).sort();
let count = 0;

for (const file of files) {
  const mmdText = fs.readFileSync(path.join(mmdDir, file), "utf-8").trim();
  const name = path.basename(file, ".mmd");

  const parsed = parseMmd(mmdText);
  const result = runLayout(parsed, mmdText);

  const outPath = path.join(outDir, `${name}.json`);
  fs.writeFileSync(outPath, JSON.stringify(result, null, 2) + "\n");
  console.log(`  ${file} → ${name}.json (${parsed.nodes.length} nodes, ${parsed.edges.length} edges)`);
  count++;
}

console.log(`\nGenerated ${count} golden expected files in ${outDir}/`);
