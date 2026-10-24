use serde::Serialize;

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub enum Indent {
    Two,
    #[default]
    Four,
    Tab,
}
impl Indent {
    pub const ALL: [Self; 3] = [Self::Two, Self::Four, Self::Tab];
    pub fn label(self) -> &'static str {
        match self {
            Self::Two => "2 空格",
            Self::Four => "4 空格",
            Self::Tab => "Tab",
        }
    }
    pub fn editor_tab(self) -> gpui_kit::component::input::TabSize {
        gpui_kit::component::input::TabSize {
            tab_size: if self == Self::Two { 2 } else { 4 },
            hard_tabs: self == Self::Tab,
        }
    }
    fn bytes(self) -> &'static [u8] {
        match self {
            Self::Two => b"  ",
            Self::Four => b"    ",
            Self::Tab => b"\t",
        }
    }
}
pub fn format(text: &str, compress: bool, indent: Indent) -> Result<String, String> {
    let value: serde_json::Value =
        serde_json::from_str(text).map_err(|e| format!("JSON 错误：{e}"))?;
    if compress {
        return serde_json::to_string(&value).map_err(|e| e.to_string());
    }
    let mut output = Vec::new();
    let formatter = serde_json::ser::PrettyFormatter::with_indent(indent.bytes());
    let mut serializer = serde_json::Serializer::with_formatter(&mut output, formatter);
    value
        .serialize(&mut serializer)
        .map_err(|e| e.to_string())?;
    String::from_utf8(output).map_err(|e| e.to_string())
}
#[cfg(test)]
pub fn convert(text: &str, compress: bool) -> Result<String, String> {
    format(text, compress, Indent::default())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn nested_arrays_and_closing_brackets_align_at_each_indent_width() {
        let source = r#"{"a":[{"b":{"c":1}},2],"d":{}}"#;
        let expected = "{\n    \"a\": [\n        {\n            \"b\": {\n                \"c\": 1\n            }\n        },\n        2\n    ],\n    \"d\": {}\n}";
        assert!(Indent::default() == Indent::Four);
        for (indent, width, hard_tabs) in [
            (Indent::Two, 2, false),
            (Indent::Four, 4, false),
            (Indent::Tab, 4, true),
        ] {
            let expected = expected.replace("    ", std::str::from_utf8(indent.bytes()).unwrap());
            let actual = format(source, false, indent).unwrap();
            assert_eq!(actual, expected);
            assert_eq!(format(&actual, false, indent).unwrap(), actual);
            assert_eq!(indent.editor_tab().tab_size, width);
            assert_eq!(indent.editor_tab().hard_tabs, hard_tabs);
        }
    }
    #[test]
    fn indentation_preserves_nested_values_and_escapes() {
        let source = r#"{"中文":{"items":[1,true,null,"line\n\"quote\""]}}"#;
        let expected: serde_json::Value = serde_json::from_str(source).unwrap();
        for indent in Indent::ALL {
            let pretty = format(source, false, indent).unwrap();
            let prefix = std::str::from_utf8(indent.bytes()).unwrap();
            assert!(
                pretty
                    .lines()
                    .nth(1)
                    .unwrap()
                    .starts_with(&format!("{prefix}\"中文\""))
            );
            assert!(
                pretty
                    .lines()
                    .nth(2)
                    .unwrap()
                    .starts_with(&prefix.repeat(2))
            );
            assert_eq!(
                serde_json::from_str::<serde_json::Value>(&pretty).unwrap(),
                expected
            );
            let compact = format(&pretty, true, indent).unwrap();
            assert!(!compact.contains('\n'));
            assert_eq!(
                serde_json::from_str::<serde_json::Value>(&compact).unwrap(),
                expected
            );
        }
        let error = format("{\n \"x\":\n}", false, Indent::Two).unwrap_err();
        assert!(error.contains("line 3"), "{error}");
    }
    #[test]
    fn json_grammar_and_highlights_are_enabled() {
        use gpui_kit::component::{
            highlighter::{HighlightTheme, SyntaxHighlighter},
            input::Rope,
        };
        let source = format(
            r#"{"title":"中文","items":[true,null,42,{"nested":1}]}"#,
            false,
            Indent::Two,
        )
        .unwrap();
        let mut highlighter = SyntaxHighlighter::new("json");
        assert!(highlighter.update(None, &Rope::from(source.as_str()), None));
        let root = highlighter
            .tree()
            .expect("JSON grammar is missing")
            .root_node();
        assert!(!root.has_error());
        assert!(root.end_position().row > 3);
        let styles =
            highlighter.styles(&(0..source.len()), HighlightTheme::default_light().as_ref());
        assert!(
            styles
                .iter()
                .filter(|(_, style)| style.color.is_some())
                .count()
                > 5
        );
    }
}
