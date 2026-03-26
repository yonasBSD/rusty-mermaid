    use super::*;

    // ── Positive tests ──

    #[test]
    fn parse_empty_diagram() {
        let d = parse("erDiagram").unwrap();
        assert!(d.entities.is_empty());
    }

    #[test]
    fn parse_entity_with_attributes() {
        let d = parse("erDiagram\n    CUSTOMER {\n        string name\n        int age\n    }").unwrap();
        assert_eq!(d.entities.len(), 1);
        assert_eq!(d.entities[0].attributes.len(), 2);
        assert_eq!(d.entities[0].attributes[0].attr_type, "string");
        assert_eq!(d.entities[0].attributes[0].name, "name");
    }

    #[test]
    fn parse_attribute_with_pk() {
        let d = parse("erDiagram\n    T {\n        int id PK\n    }").unwrap();
        assert_eq!(d.entities[0].attributes[0].keys, vec![KeyType::PrimaryKey]);
    }

    #[test]
    fn parse_attribute_with_multiple_keys() {
        let d = parse("erDiagram\n    T {\n        int id PK,FK\n    }").unwrap();
        assert_eq!(d.entities[0].attributes[0].keys, vec![KeyType::PrimaryKey, KeyType::ForeignKey]);
    }

    #[test]
    fn parse_attribute_with_comment() {
        let d = parse("erDiagram\n    T {\n        int id PK \"Primary key\"\n    }").unwrap();
        assert_eq!(d.entities[0].attributes[0].comment.as_deref(), Some("Primary key"));
    }

    #[test]
    fn parse_entity_with_alias() {
        let d = parse("erDiagram\n    CUST[Customer] {\n        string name\n    }").unwrap();
        assert_eq!(d.entities[0].alias.as_deref(), Some("Customer"));
    }

    #[test]
    fn parse_relationship_identifying() {
        let d = parse("erDiagram\n    CUSTOMER ||--o{ ORDER : places").unwrap();
        assert_eq!(d.relationships.len(), 1);
        assert_eq!(d.relationships[0].entity_a, "CUSTOMER");
        assert_eq!(d.relationships[0].entity_b, "ORDER");
        assert_eq!(d.relationships[0].cardinality_a, Cardinality::ExactlyOne);
        assert_eq!(d.relationships[0].cardinality_b, Cardinality::ZeroOrMore);
        assert_eq!(d.relationships[0].identification, Identification::Identifying);
        assert_eq!(d.relationships[0].label.as_deref(), Some("places"));
    }

    #[test]
    fn parse_relationship_non_identifying() {
        let d = parse("erDiagram\n    A }|..|{ B : has").unwrap();
        assert_eq!(d.relationships[0].cardinality_a, Cardinality::OneOrMore);
        assert_eq!(d.relationships[0].cardinality_b, Cardinality::OneOrMore);
        assert_eq!(d.relationships[0].identification, Identification::NonIdentifying);
    }

    #[test]
    fn parse_exactly_one_both_sides() {
        let d = parse("erDiagram\n    A ||--|| B : is").unwrap();
        assert_eq!(d.relationships[0].cardinality_a, Cardinality::ExactlyOne);
        assert_eq!(d.relationships[0].cardinality_b, Cardinality::ExactlyOne);
    }

    #[test]
    fn parse_zero_or_one() {
        let d = parse("erDiagram\n    A o|--|| B : optional").unwrap();
        assert_eq!(d.relationships[0].cardinality_a, Cardinality::ZeroOrOne);
        assert_eq!(d.relationships[0].cardinality_b, Cardinality::ExactlyOne);
    }

    #[test]
    fn parse_one_or_more_right() {
        let d = parse("erDiagram\n    A ||--|{ B : has").unwrap();
        assert_eq!(d.relationships[0].cardinality_b, Cardinality::OneOrMore);
    }

    #[test]
    fn parse_text_alias_cardinality() {
        let d = parse("erDiagram\n    A one or more to zero or more B : rel").unwrap();
        assert_eq!(d.relationships[0].cardinality_a, Cardinality::OneOrMore);
        assert_eq!(d.relationships[0].cardinality_b, Cardinality::ZeroOrMore);
        assert_eq!(d.relationships[0].identification, Identification::Identifying);
    }

    #[test]
    fn parse_text_alias_only_one() {
        let d = parse("erDiagram\n    A only one to zero or one B : rel").unwrap();
        assert_eq!(d.relationships[0].cardinality_a, Cardinality::ExactlyOne);
        assert_eq!(d.relationships[0].cardinality_b, Cardinality::ZeroOrOne);
    }

    #[test]
    fn parse_text_alias_optionally_to() {
        let d = parse("erDiagram\n    A only one optionally to zero or more B : rel").unwrap();
        assert_eq!(d.relationships[0].identification, Identification::NonIdentifying);
    }

    #[test]
    fn parse_auto_create_entities_from_relationship() {
        let d = parse("erDiagram\n    A ||--o{ B : has").unwrap();
        assert_eq!(d.entities.len(), 2);
        assert!(d.entity("A").is_some());
        assert!(d.entity("B").is_some());
    }

    #[test]
    fn parse_direction() {
        let d = parse("erDiagram\n    direction LR").unwrap();
        assert_eq!(d.direction, Direction::LR);
    }

    #[test]
    fn parse_comments_ignored() {
        let d = parse("erDiagram\n    %% comment\n    CUSTOMER {\n        string name\n    }").unwrap();
        assert_eq!(d.entities.len(), 1);
    }

    #[test]
    fn parse_multiple_relationships() {
        let input = "erDiagram\n    A ||--o{ B : r1\n    B ||--|{ C : r2";
        let d = parse(input).unwrap();
        assert_eq!(d.relationships.len(), 2);
        assert_eq!(d.entities.len(), 3);
    }

    #[test]
    fn parse_complex_diagram() {
        let input = "\
erDiagram
    CUSTOMER {
        string name PK
        string email UK
    }
    ORDER {
        int orderId PK
        string status
    }
    LINE-ITEM {
        int quantity
        float price
    }
    CUSTOMER ||--o{ ORDER : places
    ORDER ||--|{ LINE-ITEM : contains";
        let d = parse(input).unwrap();
        assert_eq!(d.entities.len(), 3);
        assert_eq!(d.relationships.len(), 2);
        assert_eq!(d.entities[0].attributes.len(), 2);
    }

    #[test]
    fn parse_entity_css_class() {
        let d = parse("erDiagram\n    ENTITY:::highlight {\n        int id\n    }").unwrap();
        assert_eq!(d.entities[0].css_classes, vec!["highlight"]);
    }

    #[test]
    fn parse_relationship_no_label() {
        let d = parse("erDiagram\n    A ||--|| B").unwrap();
        assert!(d.relationships[0].label.is_none());
    }

    // ── Negative tests ──

    #[test]
    fn reject_empty() {
        assert!(parse("").is_err());
    }

    #[test]
    fn reject_wrong_header() {
        assert!(parse("classDiagram\n    class Foo").is_err());
    }

    #[test]
    fn reject_whitespace_only() {
        assert!(parse("   \n\n  ").is_err());
    }
