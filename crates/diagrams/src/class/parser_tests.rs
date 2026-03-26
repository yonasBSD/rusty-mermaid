use super::*;

// ── Positive tests ──

#[test]
fn parse_empty_diagram() {
    let d = parse("classDiagram").unwrap();
    assert!(d.classes.is_empty());
    assert!(d.relationships.is_empty());
}

#[test]
fn parse_single_class() {
    let d = parse("classDiagram\n    class Animal").unwrap();
    assert_eq!(d.classes.len(), 1);
    assert_eq!(d.classes[0].id, "Animal");
}

#[test]
fn parse_class_with_generic() {
    let d = parse("classDiagram\n    class List~T~").unwrap();
    assert_eq!(d.classes[0].generic_type.as_deref(), Some("T"));
}

#[test]
fn parse_class_with_multi_generic() {
    let d = parse("classDiagram\n    class Map~KeyType, ValueType~").unwrap();
    assert_eq!(
        d.classes[0].generic_type.as_deref(),
        Some("KeyType, ValueType")
    );
}

#[test]
fn parse_class_with_label() {
    let d = parse("classDiagram\n    class C1[\"My Class\"]").unwrap();
    assert_eq!(d.classes[0].label.as_deref(), Some("My Class"));
}

#[test]
fn parse_class_with_body() {
    let d = parse(
        "classDiagram\n    class Animal {\n        +String name\n        +getAge() int\n    }",
    )
    .unwrap();
    assert_eq!(d.classes[0].members.len(), 1);
    assert_eq!(d.classes[0].methods.len(), 1);
    assert_eq!(d.classes[0].members[0].name, "String name");
    assert_eq!(d.classes[0].methods[0].name, "getAge");
}

#[test]
fn parse_class_body_annotation() {
    let d = parse("classDiagram\n    class Shape {\n        <<interface>>\n        +draw()\n    }")
        .unwrap();
    assert_eq!(d.classes[0].annotations, vec!["interface"]);
    assert_eq!(d.classes[0].methods.len(), 1);
}

#[test]
fn parse_class_inline_annotation() {
    let d = parse("classDiagram\n    class Shape <<abstract>>").unwrap();
    assert_eq!(d.classes[0].annotations, vec!["abstract"]);
}

#[test]
fn parse_standalone_annotation() {
    let d = parse("classDiagram\n    class Shape\n    <<interface>> Shape").unwrap();
    assert_eq!(d.classes[0].annotations, vec!["interface"]);
}

#[test]
fn parse_class_css_class() {
    let d = parse("classDiagram\n    class Foo:::highlight").unwrap();
    assert_eq!(d.classes[0].css_classes, vec!["highlight"]);
}

#[test]
fn parse_visibility_members() {
    let d = parse("classDiagram\n    class Foo {\n        +publicField\n        -privateField\n        #protectedField\n        ~packageField\n    }").unwrap();
    assert_eq!(d.classes[0].members[0].visibility, Visibility::Public);
    assert_eq!(d.classes[0].members[1].visibility, Visibility::Private);
    assert_eq!(d.classes[0].members[2].visibility, Visibility::Protected);
    assert_eq!(d.classes[0].members[3].visibility, Visibility::Package);
}

#[test]
fn parse_classifier_members() {
    let d = parse("classDiagram\n    class Foo {\n        +staticMethod()$\n        +abstractMethod()*\n    }").unwrap();
    assert_eq!(d.classes[0].methods[0].classifier, Classifier::Static);
    assert_eq!(d.classes[0].methods[1].classifier, Classifier::Abstract);
}

#[test]
fn parse_method_with_return_type() {
    let d = parse("classDiagram\n    class Foo {\n        +getName() String\n    }").unwrap();
    let m = &d.classes[0].methods[0];
    assert_eq!(m.name, "getName");
    assert_eq!(m.return_type.as_deref(), Some("String"));
    assert_eq!(m.parameters.as_deref(), Some(""));
}

#[test]
fn parse_method_with_params() {
    let d = parse("classDiagram\n    class Foo {\n        +setName(name String)\n    }").unwrap();
    let m = &d.classes[0].methods[0];
    assert_eq!(m.parameters.as_deref(), Some("name String"));
}

#[test]
fn parse_extension_relationship() {
    let d = parse("classDiagram\n    Animal <|-- Dog").unwrap();
    assert_eq!(d.relationships.len(), 1);
    assert_eq!(d.relationships[0].from_id, "Animal");
    assert_eq!(d.relationships[0].to_id, "Dog");
    assert_eq!(d.relationships[0].from_type, Some(RelationType::Extension));
    assert_eq!(d.relationships[0].line_type, LineType::Solid);
}

#[test]
fn parse_composition_relationship() {
    let d = parse("classDiagram\n    Car *-- Wheel").unwrap();
    assert_eq!(
        d.relationships[0].from_type,
        Some(RelationType::Composition)
    );
}

#[test]
fn parse_aggregation_relationship() {
    let d = parse("classDiagram\n    Fleet o-- Car").unwrap();
    assert_eq!(
        d.relationships[0].from_type,
        Some(RelationType::Aggregation)
    );
}

#[test]
fn parse_dependency_relationship() {
    let d = parse("classDiagram\n    Class1 --> Class2").unwrap();
    assert_eq!(d.relationships[0].to_type, Some(RelationType::Dependency));
}

#[test]
fn parse_dotted_extension() {
    let d = parse("classDiagram\n    Shape ..|> Circle").unwrap();
    assert_eq!(d.relationships[0].to_type, Some(RelationType::Extension));
    assert_eq!(d.relationships[0].line_type, LineType::Dotted);
}

#[test]
fn parse_plain_association() {
    let d = parse("classDiagram\n    A -- B").unwrap();
    assert!(d.relationships[0].from_type.is_none() && d.relationships[0].to_type.is_none());
    assert_eq!(d.relationships[0].line_type, LineType::Solid);
}

#[test]
fn parse_relationship_with_label() {
    let d = parse("classDiagram\n    Animal <|-- Dog : inherits").unwrap();
    assert_eq!(d.relationships[0].label.as_deref(), Some("inherits"));
}

#[test]
fn parse_relationship_with_cardinality() {
    let d = parse("classDiagram\n    Car \"1\" *-- \"many\" Wheel : has").unwrap();
    assert_eq!(d.relationships[0].cardinality_from.as_deref(), Some("1"));
    assert_eq!(d.relationships[0].cardinality_to.as_deref(), Some("many"));
    assert_eq!(d.relationships[0].label.as_deref(), Some("has"));
}

#[test]
fn parse_colon_member_attribute() {
    let d = parse("classDiagram\n    class Animal\n    Animal : +String name").unwrap();
    assert_eq!(d.classes[0].members.len(), 1);
    assert_eq!(d.classes[0].members[0].visibility, Visibility::Public);
}

#[test]
fn parse_colon_member_method() {
    let d = parse("classDiagram\n    class Animal\n    Animal : +eat() void").unwrap();
    assert_eq!(d.classes[0].methods.len(), 1);
    assert!(d.classes[0].methods[0].is_method());
}

#[test]
fn parse_namespace_basic() {
    let d = parse(
        "classDiagram\n    namespace MyApp {\n        class User\n        class Admin\n    }",
    )
    .unwrap();
    assert_eq!(d.namespaces.len(), 1);
    assert_eq!(d.namespaces[0].id, "MyApp");
    assert_eq!(d.namespaces[0].class_ids.len(), 2);
    assert_eq!(d.classes[0].namespace.as_deref(), Some("MyApp"));
}

#[test]
fn parse_namespace_dotted() {
    let d = parse("classDiagram\n    namespace Com.Example {\n        class Foo\n    }").unwrap();
    assert_eq!(d.namespaces[0].id, "Com.Example");
}

#[test]
fn parse_note_standalone() {
    let d = parse("classDiagram\n    note \"Important info\"").unwrap();
    assert_eq!(d.notes.len(), 1);
    assert_eq!(d.notes[0].text, "Important info");
    assert!(d.notes[0].class_id.is_none());
}

#[test]
fn parse_note_for_class() {
    let d = parse("classDiagram\n    class Animal\n    note for Animal \"Represents all animals\"")
        .unwrap();
    assert_eq!(d.notes[0].class_id.as_deref(), Some("Animal"));
}

#[test]
fn parse_direction_lr() {
    let d = parse("classDiagram\n    direction LR\n    class A").unwrap();
    assert_eq!(d.direction, Direction::LR);
}

#[test]
fn parse_comments_ignored() {
    let d = parse("classDiagram\n    %% comment\n    class Animal\n    %% another").unwrap();
    assert_eq!(d.classes.len(), 1);
}

#[test]
fn parse_class_def_styling() {
    let d =
        parse("classDiagram\n    class A\n    classDef highlight fill:#f9f,stroke:#333").unwrap();
    assert_eq!(d.class_defs.len(), 1);
}

#[test]
fn parse_multiple_relationships() {
    let input = "classDiagram\n    A <|-- B\n    B *-- C\n    C o-- D\n    D --> E";
    let d = parse(input).unwrap();
    assert_eq!(d.relationships.len(), 4);
}

#[test]
fn parse_body_separators_ignored() {
    let d = parse("classDiagram\n    class Foo {\n        field1\n        --\n        method1()\n        ..\n        method2()\n    }").unwrap();
    assert_eq!(d.classes[0].members.len(), 1);
    assert_eq!(d.classes[0].methods.len(), 2);
}

#[test]
fn parse_complex_diagram() {
    let input = "\
classDiagram
    class Animal {
        +String name
        +int age
        +makeSound()*
    }
    class Dog {
        +String breed
        +bark() void
    }
    class Cat {
        +String color
        +purr() void
    }
    Animal <|-- Dog : extends
    Animal <|-- Cat : extends
    Dog \"1\" o-- \"many\" Toy : plays with";
    let d = parse(input).unwrap();
    assert_eq!(d.classes.len(), 4, "Animal, Dog, Cat, Toy");
    assert_eq!(d.relationships.len(), 3);
    assert_eq!(d.classes[0].methods[0].classifier, Classifier::Abstract);
}

// ── Negative tests ──

#[test]
fn reject_empty_input() {
    assert!(parse("").is_err());
}

#[test]
fn reject_wrong_header() {
    assert!(parse("flowchart TD\n    A --> B").is_err());
}

#[test]
fn reject_whitespace_only() {
    assert!(parse("   \n\n  ").is_err());
}

#[test]
fn parse_v2_header() {
    let d = parse("classDiagram-v2\n    class Foo").unwrap();
    assert_eq!(d.classes.len(), 1);
}

#[test]
fn auto_create_classes_from_relationship() {
    let d = parse("classDiagram\n    A <|-- B").unwrap();
    assert_eq!(d.classes.len(), 2);
    assert!(d.class("A").is_some());
    assert!(d.class("B").is_some());
}

#[test]
fn member_display_text_roundtrip() {
    let m = parse_member_string("+getName(id int) String");
    assert_eq!(m.visibility, Visibility::Public);
    assert_eq!(m.name, "getName");
    assert_eq!(m.parameters.as_deref(), Some("id int"));
    assert_eq!(m.return_type.as_deref(), Some("String"));
    assert_eq!(m.display_text(), "+getName(id int) : String");
}
