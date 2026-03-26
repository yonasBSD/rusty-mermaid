use super::*;

fn parse_and_layout(input: &str) -> LayoutResult {
    let diagram = super::super::parser::parse(input).unwrap();
    layout(&diagram)
}

#[test]
fn single_class_produces_one_node() {
    let r = parse_and_layout("classDiagram\n    class Animal");
    assert_eq!(r.classes.len(), 1);
    assert_eq!(r.classes[0].id, "Animal");
    assert!(r.classes[0].width > 0.0);
    assert!(r.classes[0].height > 0.0);
}

#[test]
fn two_classes_with_relationship() {
    let r = parse_and_layout("classDiagram\n    Animal <|-- Dog");
    assert_eq!(r.classes.len(), 2);
    assert_eq!(r.edges.len(), 1);
    assert_eq!(r.edges[0].from_type, Some(RelationType::Extension));
}

#[test]
fn class_with_members_taller() {
    let r1 = parse_and_layout("classDiagram\n    class Foo");
    let r2 = parse_and_layout(
        "classDiagram\n    class Foo {\n        +a\n        +b\n        +c\n    }",
    );
    assert!(
        r2.classes[0].height > r1.classes[0].height,
        "class with 3 members should be taller"
    );
}

#[test]
fn class_with_methods_taller() {
    let r1 = parse_and_layout("classDiagram\n    class Foo");
    let r2 =
        parse_and_layout("classDiagram\n    class Foo {\n        +doA()\n        +doB()\n    }");
    assert!(
        r2.classes[0].height > r1.classes[0].height,
        "class with 2 methods should be taller"
    );
}

#[test]
fn min_width_enforced() {
    let r = parse_and_layout("classDiagram\n    class A");
    assert!(r.classes[0].width >= MIN_CLASS_WIDTH);
}

#[test]
fn namespace_produces_compound() {
    let r = parse_and_layout(
        "classDiagram\n    namespace MyApp {\n        class User\n        class Admin\n    }",
    );
    assert_eq!(r.classes.len(), 2);
    assert_eq!(r.namespaces.len(), 1);
    assert_eq!(r.namespaces[0].id, "MyApp");
}

#[test]
fn edge_routing_clips_at_boundary() {
    let r = parse_and_layout("classDiagram\n    A <|-- B");
    let edge = &r.edges[0].edge;
    // Edge points should not be at the center of nodes
    let a = &r.classes.iter().find(|c| c.id == "A").unwrap();
    let first = edge.points.first().unwrap();
    let at_center = (first.x - a.x).abs() < 0.1 && (first.y - a.y).abs() < 0.1;
    assert!(
        !at_center,
        "edge start should be clipped to node boundary, not center"
    );
}

#[test]
fn section_heights_computed() {
    let r = parse_and_layout(
        "classDiagram\n    class Foo {\n        +field1\n        +method1()\n    }",
    );
    let c = &r.classes[0];
    assert!(c.title_height > 0.0);
    assert!(c.members_height > 0.0);
    assert!(c.methods_height > 0.0);
    let total = c.title_height + c.members_height + c.methods_height;
    assert!(
        (total - c.height).abs() < 1.0,
        "section heights should sum to total height"
    );
}

#[test]
fn annotation_widens_box() {
    let r = parse_and_layout(
        "classDiagram\n    class Color {\n        <<enumeration>>\n        RED\n    }",
    );
    let c = &r.classes[0];
    // <<enumeration>> (17 chars) should force box wider than just "Color" (5 chars) + "RED" (3 chars)
    let measurer = SimpleTextMeasure::default();
    let style = TextStyle::default();
    let ann_w = measurer.measure("<<enumeration>>", &style).width;
    assert!(
        c.width >= ann_w,
        "box width {} should contain annotation width {ann_w}",
        c.width
    );
}

#[test]
fn cardinality_preserved() {
    let r = parse_and_layout("classDiagram\n    A \"1\" *-- \"many\" B : has");
    assert_eq!(r.edges[0].cardinality_from.as_deref(), Some("1"));
    assert_eq!(r.edges[0].cardinality_to.as_deref(), Some("many"));
}

#[test]
fn direction_lr() {
    let r = parse_and_layout("classDiagram\n    direction LR\n    A <|-- B");
    // In LR layout, nodes should be side by side (x differs more than y)
    let a = r.classes.iter().find(|c| c.id == "A").unwrap();
    let b = r.classes.iter().find(|c| c.id == "B").unwrap();
    let dx = (a.x - b.x).abs();
    let dy = (a.y - b.y).abs();
    assert!(
        dx > dy,
        "LR layout: horizontal distance ({dx}) should exceed vertical ({dy})"
    );
}
