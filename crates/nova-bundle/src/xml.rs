//! Text for the XML the packers write (the WiX source, the AppStream
//! metainfo), escaped so markup in a value stays text.

/// `text` escaped for an XML attribute value or element text.
#[must_use]
pub fn escape(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&apos;"),
            c => escaped.push(c),
        }
    }
    escaped
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn markup_becomes_entities_and_the_rest_stays() {
        assert_eq!(
            escape(r#"EV <Nova> & "co" 's é"#),
            "EV &lt;Nova&gt; &amp; &quot;co&quot; &apos;s é"
        );
    }
}
