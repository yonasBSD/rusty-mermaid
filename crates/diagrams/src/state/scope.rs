use std::collections::{BTreeMap, HashSet};

use rusty_mermaid_core::{Shape, SimpleTextMeasure, Style, TextMeasure, TextStyle};
use rusty_mermaid_dagre::{EdgeLabel, NodeLabel};
use rusty_mermaid_graph::{Graph, NodeId};

use super::bridge::{
    CHOICE_SIZE, FORK_JOIN_HEIGHT, FORK_JOIN_WIDTH, PADDING_X, PADDING_Y, START_END_SIZE,
};
use super::ir::{StateDiagram, StateKind, StateNode, StateNote, StateTransition};

/// Process one scope (top-level or inside a composite): create nodes, pseudo-states,
/// edges, and compound parent relationships.
#[allow(clippy::too_many_arguments)]
/// Mutable state threaded through recursive scope building.
pub(super) struct ScopeCtx<'c, M: TextMeasure> {
    pub(super) graph: &'c mut Graph<NodeLabel, EdgeLabel>,
    pub(super) id_map: &'c mut BTreeMap<String, NodeId>,
    pub(super) synthetic_ids: &'c mut HashSet<String>,
    pub(super) measurer: &'c M,
    pub(super) style: &'c TextStyle,
}

pub(super) fn add_scope<'a, M: TextMeasure>(
    states: &'a [StateNode],
    transitions: &'a [StateTransition],
    parent: Option<(NodeId, &str)>,
    ctx: &mut ScopeCtx<'_, M>,
    all_transitions: &mut Vec<&'a StateTransition>,
) {
    let scope_prefix = parent.map(|(_, id)| format!("{id}.")).unwrap_or_default();
    let start_key = format!("{scope_prefix}[*]_start");
    let end_key = format!("{scope_prefix}[*]_end");

    add_pseudo_states(transitions, parent, &start_key, &end_key, ctx);
    add_state_nodes(states, transitions, parent, ctx, all_transitions);
    wire_edges(transitions, &start_key, &end_key, ctx, all_transitions);
}

pub(super) fn add_pseudo_states<M: TextMeasure>(
    transitions: &[StateTransition],
    parent: Option<(NodeId, &str)>,
    start_key: &str,
    end_key: &str,
    ctx: &mut ScopeCtx<'_, M>,
) {
    for (key, has) in [
        (start_key, transitions.iter().any(|t| t.src == "[*]")),
        (end_key, transitions.iter().any(|t| t.dst == "[*]")),
    ] {
        if has {
            let nid = ctx
                .graph
                .add_node(NodeLabel::new(START_END_SIZE, START_END_SIZE));
            if let Some((parent_nid, _)) = parent {
                ctx.graph.set_parent(nid, parent_nid);
            }
            ctx.id_map.insert(key.to_string(), nid);
        }
    }
}

pub(super) fn add_state_nodes<'a, M: TextMeasure>(
    states: &'a [StateNode],
    transitions: &'a [StateTransition],
    parent: Option<(NodeId, &str)>,
    ctx: &mut ScopeCtx<'_, M>,
    all_transitions: &mut Vec<&'a StateTransition>,
) {
    for s in states {
        let (width, height) = match &s.kind {
            StateKind::Fork | StateKind::Join => (FORK_JOIN_WIDTH, FORK_JOIN_HEIGHT),
            StateKind::Choice => (CHOICE_SIZE, CHOICE_SIZE),
            StateKind::Start | StateKind::End => (START_END_SIZE, START_END_SIZE),
            StateKind::History => (START_END_SIZE, START_END_SIZE),
            StateKind::Normal => {
                let text = s.label.as_deref().unwrap_or(&s.id);
                let ts = ctx.measurer.measure(text, ctx.style);
                (ts.width + PADDING_X * 2.0, ts.height + PADDING_Y * 2.0)
            }
            StateKind::Composite {
                children,
                transitions: inner_trans,
                regions,
                ..
            } => {
                add_composite_state(
                    s,
                    children,
                    inner_trans,
                    regions,
                    transitions,
                    parent,
                    ctx,
                    all_transitions,
                );
                continue;
            }
        };

        let nid = ctx.graph.add_node(NodeLabel::new(width, height));
        ctx.id_map.insert(s.id.clone(), nid);
        if let Some((parent_nid, _)) = parent {
            ctx.graph.set_parent(nid, parent_nid);
        }
    }
}

pub(super) fn add_composite_state<'a, M: TextMeasure>(
    s: &'a StateNode,
    children: &'a [StateNode],
    inner_trans: &'a [StateTransition],
    regions: &'a [super::ir::ConcurrentRegion],
    outer_transitions: &'a [StateTransition],
    parent: Option<(NodeId, &str)>,
    ctx: &mut ScopeCtx<'_, M>,
    all_transitions: &mut Vec<&'a StateTransition>,
) {
    let nid = ctx.graph.add_node(NodeLabel::new(0.0, 0.0));
    ctx.id_map.insert(s.id.clone(), nid);
    if let Some((parent_nid, _)) = parent {
        ctx.graph.set_parent(nid, parent_nid);
    }

    let inner_start_key = format!("{}.[*]_start", s.id);
    let inner_end_key = format!("{}.[*]_end", s.id);

    if regions.is_empty() {
        add_scope(
            children,
            inner_trans,
            Some((nid, &s.id)),
            ctx,
            all_transitions,
        );
        for child in children {
            if let Some(&child_nid) = ctx.id_map.get(child.id.as_str()) {
                if ctx.graph.parent(child_nid).is_none() {
                    ctx.graph.set_parent(child_nid, nid);
                }
            }
        }
    } else {
        add_concurrent_regions(
            s,
            regions,
            nid,
            &inner_start_key,
            &inner_end_key,
            outer_transitions,
            ctx,
            all_transitions,
        );
    }

    // Synthetic exit if composite is an edge source but has no inner [*]_end
    if !ctx.id_map.contains_key(&inner_end_key) && outer_transitions.iter().any(|t| t.src == s.id) {
        let end_nid = ctx
            .graph
            .add_node(NodeLabel::new(START_END_SIZE, START_END_SIZE));
        ctx.graph.set_parent(end_nid, nid);
        ctx.synthetic_ids.insert(inner_end_key.clone());
        ctx.id_map.insert(inner_end_key, end_nid);
        if let Some(last) = children.last() {
            if let Some(&child_nid) = ctx.id_map.get(last.id.as_str()) {
                ctx.graph.add_edge(child_nid, end_nid, EdgeLabel::default());
            }
        }
    }
}

pub(super) fn add_concurrent_regions<'a, M: TextMeasure>(
    s: &'a StateNode,
    regions: &'a [super::ir::ConcurrentRegion],
    compound_nid: NodeId,
    inner_start_key: &str,
    inner_end_key: &str,
    outer_transitions: &'a [StateTransition],
    ctx: &mut ScopeCtx<'_, M>,
    all_transitions: &mut Vec<&'a StateTransition>,
) {
    for (i, region) in regions.iter().enumerate() {
        let region_nid = ctx.graph.add_node(NodeLabel::new(0.0, 0.0));
        let region_key = format!("{}._region_{}", s.id, i);
        ctx.graph.set_parent(region_nid, compound_nid);
        ctx.synthetic_ids.insert(region_key.clone());
        ctx.id_map.insert(region_key.clone(), region_nid);

        add_scope(
            &region.children,
            &region.transitions,
            Some((region_nid, &region_key)),
            ctx,
            all_transitions,
        );

        for child in &region.children {
            if let Some(&child_nid) = ctx.id_map.get(child.id.as_str()) {
                if ctx.graph.parent(child_nid).is_none() {
                    ctx.graph.set_parent(child_nid, region_nid);
                }
            }
        }
    }

    // Compound-level entry connecting to all region starts
    let region_starts: Vec<NodeId> = (0..regions.len())
        .filter_map(|i| {
            let sk = format!("{}._region_{}.[*]_start", s.id, i);
            ctx.id_map.get(&sk).copied()
        })
        .collect();
    if !region_starts.is_empty() && !ctx.id_map.contains_key(inner_start_key) {
        let entry_nid = ctx
            .graph
            .add_node(NodeLabel::new(START_END_SIZE, START_END_SIZE));
        ctx.graph.set_parent(entry_nid, compound_nid);
        ctx.synthetic_ids.insert(inner_start_key.to_string());
        ctx.id_map.insert(inner_start_key.to_string(), entry_nid);
        for &rs in &region_starts {
            ctx.graph.add_edge(entry_nid, rs, EdgeLabel::default());
        }
    }

    // Compound-level exit connecting from all regions' last children
    let is_src = outer_transitions.iter().any(|t| t.src == s.id);
    if is_src && !ctx.id_map.contains_key(inner_end_key) {
        let exit_nid = ctx
            .graph
            .add_node(NodeLabel::new(START_END_SIZE, START_END_SIZE));
        ctx.graph.set_parent(exit_nid, compound_nid);
        ctx.synthetic_ids.insert(inner_end_key.to_string());
        ctx.id_map.insert(inner_end_key.to_string(), exit_nid);
        for region in regions {
            if let Some(last) = region.children.last() {
                if let Some(&cn) = ctx.id_map.get(last.id.as_str()) {
                    ctx.graph.add_edge(cn, exit_nid, EdgeLabel::default());
                }
            }
        }
    }
}

pub(super) fn wire_edges<'a, M: TextMeasure>(
    transitions: &'a [StateTransition],
    start_key: &str,
    end_key: &str,
    ctx: &mut ScopeCtx<'_, M>,
    all_transitions: &mut Vec<&'a StateTransition>,
) {
    for t in transitions {
        let mut src_key = if t.src == "[*]" {
            start_key.to_string()
        } else {
            t.src.clone()
        };
        let mut dst_key = if t.dst == "[*]" {
            end_key.to_string()
        } else {
            t.dst.clone()
        };

        // Redirect: edge FROM composite → use inner [*]_end
        if t.src != "[*]" {
            let inner_end = format!("{}.[*]_end", t.src);
            if ctx.id_map.contains_key(&inner_end) {
                src_key = inner_end;
            }
        }
        // Redirect: edge TO composite → use inner [*]_start
        if t.dst != "[*]" {
            let inner_start = format!("{}.[*]_start", t.dst);
            if ctx.id_map.contains_key(&inner_start) {
                dst_key = inner_start;
            }
        }

        let Some(&src) = ctx.id_map.get(&src_key) else {
            continue;
        };
        let Some(&dst) = ctx.id_map.get(&dst_key) else {
            continue;
        };

        let mut label = EdgeLabel::default();
        if let Some(text) = &t.label {
            let ts = ctx.measurer.measure(text, ctx.style);
            label.width = ts.width;
            label.height = ts.height;
        }
        ctx.graph.add_edge(src, dst, label);
        all_transitions.push(t);
    }
}

/// Return the number of concurrent regions for a state (0 if not concurrent).
pub(super) fn region_count(states: &[super::ir::StateNode], id: &str) -> usize {
    for s in states {
        if s.id == id {
            if let StateKind::Composite { regions, .. } = &s.kind {
                return regions.len();
            }
            return 0;
        }
        if let StateKind::Composite { children, .. } = &s.kind {
            let c = region_count(children, id);
            if c > 0 {
                return c;
            }
        }
    }
    0
}

/// Collect all notes from the diagram, including those inside composites.
pub(super) fn collect_all_notes(diagram: &StateDiagram) -> Vec<&StateNote> {
    let mut result = Vec::new();
    for note in &diagram.notes {
        result.push(note);
    }
    fn collect_from_states<'a>(states: &'a [StateNode], result: &mut Vec<&'a StateNote>) {
        for s in states {
            if let StateKind::Composite {
                notes, children, ..
            } = &s.kind
            {
                for note in notes {
                    result.push(note);
                }
                collect_from_states(children, result);
            }
        }
    }
    collect_from_states(&diagram.states, &mut result);
    result
}

/// Resolve classDef + class + style into a per-state Style map.
pub(super) fn resolve_state_styles(diagram: &StateDiagram) -> BTreeMap<&str, Style> {
    fn flatten_states(states: &[StateNode]) -> Vec<(&str, &[String])> {
        let mut out = Vec::new();
        for s in states {
            out.push((s.id.as_str(), s.classes.as_slice()));
            if let StateKind::Composite { children, .. } = &s.kind {
                out.extend(flatten_states(children));
            }
        }
        out
    }
    let entities = flatten_states(&diagram.states);
    crate::common::rendering::resolve_entity_styles(
        entities.into_iter(),
        &diagram.class_defs,
        &diagram.style_stmts,
    )
}
