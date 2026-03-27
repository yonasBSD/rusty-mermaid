pub mod common;

#[cfg(feature = "flowchart")]
pub mod flowchart;

#[cfg(feature = "state")]
pub mod state;

#[cfg(feature = "sequence")]
pub mod sequence;

#[cfg(feature = "class")]
pub mod class;

#[cfg(feature = "er")]
pub mod er;

#[cfg(feature = "requirement")]
pub mod requirement;

#[cfg(feature = "pie")]
pub mod pie;

#[cfg(feature = "timeline")]
pub mod timeline;

#[cfg(feature = "kanban")]
pub mod kanban;

#[cfg(feature = "gantt")]
pub mod gantt;

#[cfg(feature = "gitgraph")]
pub mod gitgraph;

#[cfg(feature = "xychart")]
pub mod xychart;

#[cfg(feature = "mindmap")]
pub mod mindmap;

#[cfg(feature = "sankey")]
pub mod sankey;

#[cfg(feature = "packet")]
pub mod packet;

#[cfg(feature = "quadrant")]
pub mod quadrant;

#[cfg(feature = "venn")]
pub mod venn;

#[cfg(feature = "radar")]
pub mod radar;

#[cfg(feature = "user-journey")]
pub mod journey;

#[cfg(feature = "treeview")]
pub mod treeview;

#[cfg(feature = "ishikawa")]
pub mod ishikawa;

#[cfg(feature = "treemap")]
pub mod treemap;

#[cfg(feature = "block")]
pub mod block;

#[cfg(feature = "c4")]
pub mod c4;

#[cfg(feature = "architecture")]
pub mod architecture;

pub use common::error::ParseError;

/// Supported diagram types.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DiagramKind {
    Flowchart,
    State,
    Sequence,
    Class,
    Er,
    Requirement,
    Pie,
    Timeline,
    Kanban,
    Gantt,
    GitGraph,
    XyChart,
    Mindmap,
    Sankey,
    Packet,
    Quadrant,
    Venn,
    Radar,
    UserJourney,
    TreeView,
    Ishikawa,
    Treemap,
    Block,
    C4,
    Architecture,
}

/// Detect the diagram type from the first non-empty, non-comment line.
pub fn detect(input: &str) -> Option<DiagramKind> {
    let line = input
        .lines()
        .map(str::trim)
        .find(|l| !l.is_empty() && !l.starts_with("%%"))?;

    match line {
        l if l.starts_with("graph") || l.starts_with("flowchart") => Some(DiagramKind::Flowchart),
        l if l.starts_with("stateDiagram") => Some(DiagramKind::State),
        l if l.starts_with("sequenceDiagram") => Some(DiagramKind::Sequence),
        l if l.starts_with("classDiagram") => Some(DiagramKind::Class),
        l if l.starts_with("erDiagram") => Some(DiagramKind::Er),
        l if l.starts_with("requirementDiagram") => Some(DiagramKind::Requirement),
        l if l.starts_with("pie") => Some(DiagramKind::Pie),
        l if l.starts_with("timeline") => Some(DiagramKind::Timeline),
        l if l.starts_with("kanban") => Some(DiagramKind::Kanban),
        l if l.starts_with("gantt") => Some(DiagramKind::Gantt),
        l if l.starts_with("gitGraph") => Some(DiagramKind::GitGraph),
        l if l.starts_with("xychart") => Some(DiagramKind::XyChart),
        l if l.starts_with("mindmap") => Some(DiagramKind::Mindmap),
        l if l.starts_with("sankey") => Some(DiagramKind::Sankey),
        l if l.starts_with("packet") => Some(DiagramKind::Packet),
        l if l.starts_with("quadrantChart") => Some(DiagramKind::Quadrant),
        l if l.starts_with("venn") => Some(DiagramKind::Venn),
        l if l.starts_with("radar") => Some(DiagramKind::Radar),
        l if l.starts_with("journey") => Some(DiagramKind::UserJourney),
        l if l.starts_with("treeView") => Some(DiagramKind::TreeView),
        l if l.starts_with("ishikawa") => Some(DiagramKind::Ishikawa),
        l if l.starts_with("treemap") => Some(DiagramKind::Treemap),
        l if l.starts_with("C4") => Some(DiagramKind::C4),
        l if l.starts_with("architecture") => Some(DiagramKind::Architecture),
        l if l.starts_with("block") => Some(DiagramKind::Block),
        _ => None,
    }
}

/// Strip mermaid directives, accessibility metadata, and leading comments.
///
/// Handles:
/// - `%%{init: ...}%%` configuration directives
/// - `accTitle: ...` accessibility title
/// - `accDescr: ...` or `accDescr { ... }` accessibility description
/// - `%%` comment lines before the diagram keyword
fn preprocess(input: &str) -> String {
    let mut lines: Vec<&str> = Vec::new();
    let mut in_acc_block = false;

    for line in input.lines() {
        let trimmed = line.trim();

        // Skip %%{...}%% directives
        if trimmed.starts_with("%%{") && trimmed.ends_with("}%%") {
            continue;
        }

        // Skip accTitle / accDescr single-line
        if trimmed.starts_with("accTitle:") || trimmed.starts_with("accDescr:") {
            continue;
        }

        // Handle accDescr { ... } multi-line block
        if trimmed.starts_with("accDescr") && trimmed.contains('{') {
            in_acc_block = true;
            continue;
        }
        if in_acc_block {
            if trimmed.contains('}') {
                in_acc_block = false;
            }
            continue;
        }

        lines.push(line);
    }

    lines.join("\n")
}

/// Unified entry: parse + layout → Scene.
#[cfg(any(
    feature = "flowchart",
    feature = "state",
    feature = "sequence",
    feature = "class",
    feature = "er",
    feature = "requirement",
    feature = "pie",
    feature = "timeline",
    feature = "kanban",
    feature = "gantt",
    feature = "gitgraph",
    feature = "xychart",
    feature = "mindmap",
    feature = "sankey",
    feature = "packet",
    feature = "quadrant",
    feature = "venn",
    feature = "radar",
    feature = "user-journey",
    feature = "treeview",
    feature = "ishikawa",
    feature = "treemap",
    feature = "block",
    feature = "c4",
    feature = "architecture"
))]
pub fn render_to_scene(
    input: &str,
    theme: &rusty_mermaid_core::Theme,
) -> Result<rusty_mermaid_core::Scene, ParseError> {
    let cleaned = preprocess(input);
    let input = &cleaned;

    let kind = detect(input).ok_or_else(|| {
        ParseError::new(common::error::ParseErrorKind::UnexpectedToken, 0..0, input)
    })?;

    match kind {
        #[cfg(feature = "flowchart")]
        DiagramKind::Flowchart => {
            let diagram = flowchart::parser::parse(input)?;
            let layout = flowchart::bridge::layout(&diagram);
            Ok(flowchart::to_scene(&layout, theme))
        }
        #[cfg(feature = "state")]
        DiagramKind::State => {
            let diagram = state::parser::parse(input)?;
            let layout = state::bridge::layout(&diagram);
            Ok(state::to_scene(&layout, theme))
        }
        #[cfg(feature = "sequence")]
        DiagramKind::Sequence => {
            let diagram = sequence::parser::parse(input)?;
            let layout = sequence::layout::layout(
                &diagram,
                &rusty_mermaid_core::SimpleTextMeasure::default(),
            );
            Ok(sequence::to_scene(&layout, theme))
        }
        #[cfg(feature = "class")]
        DiagramKind::Class => {
            let diagram = class::parser::parse(input)?;
            let layout = class::bridge::layout(&diagram);
            Ok(class::to_scene(&layout, theme))
        }
        #[cfg(feature = "er")]
        DiagramKind::Er => {
            let diagram = er::parser::parse(input)?;
            let layout = er::bridge::layout(&diagram);
            Ok(er::to_scene(&layout, theme))
        }
        #[cfg(feature = "requirement")]
        DiagramKind::Requirement => {
            let diagram = requirement::parser::parse(input)?;
            let layout = requirement::bridge::layout(&diagram);
            Ok(requirement::to_scene(&layout, theme))
        }
        #[cfg(feature = "pie")]
        DiagramKind::Pie => {
            let chart = pie::parser::parse(input)?;
            Ok(pie::to_scene(&chart, theme))
        }
        #[cfg(feature = "timeline")]
        DiagramKind::Timeline => {
            let diagram = timeline::parser::parse(input)?;
            Ok(timeline::to_scene(&diagram, theme))
        }
        #[cfg(feature = "kanban")]
        DiagramKind::Kanban => {
            let board = kanban::parser::parse(input)?;
            Ok(kanban::to_scene(&board, theme))
        }
        #[cfg(feature = "gantt")]
        DiagramKind::Gantt => {
            let chart = gantt::parser::parse(input)?;
            Ok(gantt::to_scene(&chart, theme))
        }
        #[cfg(feature = "gitgraph")]
        DiagramKind::GitGraph => {
            let graph = gitgraph::parser::parse(input)?;
            Ok(gitgraph::to_scene(&graph, theme))
        }
        #[cfg(feature = "xychart")]
        DiagramKind::XyChart => {
            let chart = xychart::parser::parse(input)?;
            Ok(xychart::to_scene(&chart, theme))
        }
        #[cfg(feature = "mindmap")]
        DiagramKind::Mindmap => {
            let diagram = mindmap::parser::parse(input)?;
            Ok(mindmap::to_scene(&diagram, theme))
        }
        #[cfg(feature = "sankey")]
        DiagramKind::Sankey => {
            let diagram = sankey::parser::parse(input)?;
            Ok(sankey::to_scene(&diagram, theme))
        }
        #[cfg(feature = "packet")]
        DiagramKind::Packet => {
            let diagram = packet::parser::parse(input)?;
            Ok(packet::to_scene(&diagram, theme))
        }
        #[cfg(feature = "quadrant")]
        DiagramKind::Quadrant => {
            let chart = quadrant::parser::parse(input)?;
            Ok(quadrant::to_scene(&chart, theme))
        }
        #[cfg(feature = "venn")]
        DiagramKind::Venn => {
            let diagram = venn::parser::parse(input)?;
            Ok(venn::to_scene(&diagram, theme))
        }
        #[cfg(feature = "radar")]
        DiagramKind::Radar => {
            let chart = radar::parser::parse(input)?;
            Ok(radar::to_scene(&chart, theme))
        }
        #[cfg(feature = "user-journey")]
        DiagramKind::UserJourney => {
            let diagram = journey::parser::parse(input)?;
            Ok(journey::to_scene(&diagram, theme))
        }
        #[cfg(feature = "treeview")]
        DiagramKind::TreeView => {
            let tree = treeview::parser::parse(input)?;
            Ok(treeview::to_scene(&tree, theme))
        }
        #[cfg(feature = "ishikawa")]
        DiagramKind::Ishikawa => {
            let diagram = ishikawa::parser::parse(input)?;
            Ok(ishikawa::to_scene(&diagram, theme))
        }
        #[cfg(feature = "treemap")]
        DiagramKind::Treemap => {
            let diagram = treemap::parser::parse(input)?;
            Ok(treemap::to_scene(&diagram, theme))
        }
        #[cfg(feature = "block")]
        DiagramKind::Block => {
            let diagram = block::parser::parse(input)?;
            Ok(block::to_scene(&diagram, theme))
        }
        #[cfg(feature = "c4")]
        DiagramKind::C4 => {
            let diagram = c4::parser::parse(input)?;
            Ok(c4::to_scene(&diagram, theme))
        }
        #[cfg(feature = "architecture")]
        DiagramKind::Architecture => {
            let diagram = architecture::parser::parse(input)?;
            Ok(architecture::to_scene(&diagram, theme))
        }
        #[allow(unreachable_patterns)]
        _ => Err(ParseError::new(
            common::error::ParseErrorKind::UnexpectedToken,
            0..0,
            input,
        )),
    }
}

#[cfg(test)]
mod lib_tests;
