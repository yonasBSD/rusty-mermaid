    use super::*;

    #[test]
    fn parse_identifier() {
        assert_eq!(identifier.parse_peek("hello world"), Ok((" world", "hello")));
        assert_eq!(identifier.parse_peek("_foo"), Ok(("", "_foo")));
        assert!(identifier.parse_peek("123").is_err());
    }

    #[test]
    fn parse_node_id() {
        assert_eq!(node_id.parse_peek("my-node rest"), Ok((" rest", "my-node")));
        assert_eq!(node_id.parse_peek("A123"), Ok(("", "A123")));
        assert!(node_id.parse_peek("-bad").is_err());
    }

    #[test]
    fn parse_quoted_string() {
        assert_eq!(quoted_string.parse_peek("\"hello world\""), Ok(("", "hello world")));
        assert_eq!(
            quoted_string.parse_peek("\"<b>Bold</b>\" rest"),
            Ok((" rest", "<b>Bold</b>"))
        );
    }

    #[test]
    fn parse_direction_keywords() {
        assert_eq!(direction.parse_peek("TB"), Ok(("", Direction::TB)));
        assert_eq!(direction.parse_peek("TD"), Ok(("", Direction::TB)));
        assert_eq!(direction.parse_peek("LR"), Ok(("", Direction::LR)));
        assert_eq!(direction.parse_peek("RL"), Ok(("", Direction::RL)));
        assert_eq!(direction.parse_peek("BT"), Ok(("", Direction::BT)));
    }

    #[test]
    fn parse_text_until_delimiter() {
        let mut input = "hello world]rest";
        let result = text_until(']', &mut input).unwrap();
        assert_eq!(result, "hello world");
        assert_eq!(input, "]rest");
    }

    #[test]
    fn text_until_handles_nested_brackets() {
        let mut input = "a [nested] end]rest";
        let result = text_until(']', &mut input).unwrap();
        assert_eq!(result, "a [nested] end");
    }

    #[test]
    fn strip_tags() {
        assert_eq!(strip_html_tags("<b>Bold</b>"), "Bold");
        assert_eq!(strip_html_tags("Line 1<br/>Line 2"), "Line 1\nLine 2");
        assert_eq!(
            strip_html_tags("<i>italic</i> and <b>bold</b>"),
            "italic and bold"
        );
        assert_eq!(strip_html_tags("no tags"), "no tags");
    }

    #[test]
    fn parse_style_class() {
        assert_eq!(
            style_class.parse_peek(":::myClass rest"),
            Ok((" rest", "myClass"))
        );
    }

    #[test]
    fn skip_whitespace_and_comments() {
        let mut input = "  \n  %% comment\n  rest";
        skip.parse_next(&mut input).unwrap();
        assert_eq!(input, "rest");
    }

    #[test]
    fn line_comment_stops_at_newline() {
        let mut input = "%% this is a comment\nnext";
        line_comment.parse_next(&mut input).unwrap();
        assert_eq!(input, "\nnext");
    }

    #[test]
    fn unescape_basic() {
        assert_eq!(unescape_unicode(r"\u00e9"), "é");
        assert_eq!(unescape_unicode(r"\u2615"), "☕");
        assert_eq!(unescape_unicode(r"Caf\u00e9 \u2615"), "Café ☕");
    }

    #[test]
    fn unescape_no_escapes() {
        assert_eq!(unescape_unicode("hello"), "hello");
        assert_eq!(unescape_unicode("Café ☕"), "Café ☕");
    }

    #[test]
    fn unescape_partial_sequence() {
        // Incomplete \u sequences left as-is
        assert_eq!(unescape_unicode(r"\u00"), r"\u00");
        assert_eq!(unescape_unicode(r"\u"), r"\u");
        assert_eq!(unescape_unicode(r"\uzzz"), r"\uzzz");
    }

    #[test]
    fn unescape_cjk_and_symbols() {
        assert_eq!(unescape_unicode(r"\u4f60\u597d"), "你好");
        assert_eq!(unescape_unicode(r"\u03b1 + \u03b2"), "α + β");
    }
