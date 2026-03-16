use winnow::combinator::{opt, separated};
use winnow::prelude::*;
use winnow::token::take_while;

use super::tokens::{identifier, ws};

/// A single CSS-like style property, e.g. `fill:#f9f` or `stroke-width:4px`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StyleProperty {
    pub key: String,
    pub value: String,
}

/// A `classDef` statement: `classDef className fill:#f9f,stroke:#333`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClassDef {
    pub name: String,
    pub styles: Vec<StyleProperty>,
}

/// A `style` statement: `style nodeId fill:#f9f`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StyleStmt {
    pub ids: Vec<String>,
    pub styles: Vec<StyleProperty>,
}

/// A `class` statement: `class nodeId1,nodeId2 className`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClassApply {
    pub ids: Vec<String>,
    pub class_name: String,
}

/// Parse a style property key (allows hyphens: `stroke-width`).
fn style_key<'i>(input: &mut &'i str) -> ModalResult<&'i str> {
    take_while(1.., |c: char| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        .parse_next(input)
}

/// Parse a style property value (everything until `,` or end of statement).
fn style_value<'i>(input: &mut &'i str) -> ModalResult<&'i str> {
    take_while(1.., |c: char| c != ',' && c != ';' && c != '\n' && c != '\r')
        .parse_next(input)
}

/// Parse `key:value`.
fn style_property(input: &mut &str) -> ModalResult<StyleProperty> {
    (style_key, opt(ws), ':', opt(ws), style_value)
        .map(|(key, _, _, _, value): (&str, _, _, _, &str)| StyleProperty {
            key: key.to_string(),
            value: value.trim().to_string(),
        })
        .parse_next(input)
}

/// Parse comma-separated style properties: `fill:#f9f,stroke:#333,stroke-width:4px`.
pub fn style_properties(input: &mut &str) -> ModalResult<Vec<StyleProperty>> {
    separated(1.., style_property, (opt(ws), ',', opt(ws))).parse_next(input)
}

/// Parse comma-separated identifiers: `nodeId1,nodeId2`.
fn id_list(input: &mut &str) -> ModalResult<Vec<String>> {
    separated(1.., identifier.map(|s: &str| s.to_string()), (opt(ws), ',', opt(ws)))
        .parse_next(input)
}

/// Parse `classDef className fill:#f9f,stroke:#333`.
/// The `classDef` keyword should already be consumed.
pub fn class_def_body(input: &mut &str) -> ModalResult<ClassDef> {
    (
        ws,
        // "default" is a valid class name in mermaid
        take_while(1.., |c: char| c.is_ascii_alphanumeric() || c == '_' || c == '-'),
        ws,
        style_properties,
    )
        .map(|(_, name, _, styles): (_, &str, _, _)| ClassDef {
            name: name.to_string(),
            styles,
        })
        .parse_next(input)
}

/// Parse `style nodeId1,nodeId2 fill:#f9f`.
/// The `style` keyword should already be consumed.
pub fn style_stmt_body(input: &mut &str) -> ModalResult<StyleStmt> {
    (ws, id_list, ws, style_properties)
        .map(|(_, ids, _, styles)| StyleStmt { ids, styles })
        .parse_next(input)
}

/// Parse `class nodeId1,nodeId2 className`.
/// The `class` keyword should already be consumed.
pub fn class_apply_body(input: &mut &str) -> ModalResult<ClassApply> {
    (ws, id_list, ws, identifier)
        .map(|(_, ids, _, class_name): (_, _, _, &str)| ClassApply {
            ids,
            class_name: class_name.to_string(),
        })
        .parse_next(input)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_style_property() {
        let mut input = "fill:#f9f";
        let prop = style_property.parse_next(&mut input).unwrap();
        assert_eq!(prop.key, "fill");
        assert_eq!(prop.value, "#f9f");
    }

    #[test]
    fn parse_style_properties_list() {
        let mut input = "fill:#f9f,stroke:#333,stroke-width:4px";
        let props = style_properties.parse_next(&mut input).unwrap();
        assert_eq!(props.len(), 3);
        assert_eq!(props[0].key, "fill");
        assert_eq!(props[2].key, "stroke-width");
        assert_eq!(props[2].value, "4px");
    }

    #[test]
    fn parse_class_def() {
        let mut input = " myClass fill:#f9f,stroke:#333";
        let cd = class_def_body.parse_next(&mut input).unwrap();
        assert_eq!(cd.name, "myClass");
        assert_eq!(cd.styles.len(), 2);
    }

    #[test]
    fn parse_class_def_default() {
        let mut input = " default fill:#aaa";
        let cd = class_def_body.parse_next(&mut input).unwrap();
        assert_eq!(cd.name, "default");
    }

    #[test]
    fn parse_style_stmt() {
        let mut input = " A,B fill:#f9f";
        let ss = style_stmt_body.parse_next(&mut input).unwrap();
        assert_eq!(ss.ids, vec!["A", "B"]);
        assert_eq!(ss.styles.len(), 1);
    }

    #[test]
    fn parse_class_apply() {
        let mut input = " A,B myClass";
        let ca = class_apply_body.parse_next(&mut input).unwrap();
        assert_eq!(ca.ids, vec!["A", "B"]);
        assert_eq!(ca.class_name, "myClass");
    }
}
