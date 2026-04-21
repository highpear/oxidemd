use std::fs;
use std::path::Path;

use pulldown_cmark::{Options, Parser, html};

pub fn write_html_export(source_path: &Path, output_path: &Path) -> Result<(), String> {
    let markdown = fs::read_to_string(source_path).map_err(|error| error.to_string())?;
    let title = source_path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("OxideMD Export");
    let html = markdown_to_html_document(&markdown, title);

    fs::write(output_path, html).map_err(|error| error.to_string())
}

fn markdown_to_html_document(markdown: &str, title: &str) -> String {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);

    let parser = Parser::new_ext(markdown, options);
    let mut body = String::new();
    html::push_html(&mut body, parser);

    format!(
        r#"<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>{}</title>
<style>
:root {{
  color-scheme: light;
  --text: #1f2933;
  --muted: #5f6b7a;
  --border: #d7dde5;
  --background: #ffffff;
  --code-background: #f4f6f8;
  --quote-background: #f7f9fb;
  --link: #1f6feb;
}}
body {{
  margin: 40px auto;
  max-width: 760px;
  padding: 0 24px 48px;
  background: var(--background);
  color: var(--text);
  font-family: system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
  line-height: 1.65;
}}
h1, h2, h3, h4, h5, h6 {{
  line-height: 1.25;
  margin: 1.6em 0 0.55em;
}}
p, ul, ol, blockquote, table, pre {{
  margin: 0 0 1.1em;
}}
a {{
  color: var(--link);
}}
code, pre {{
  font-family: ui-monospace, SFMono-Regular, Consolas, "Liberation Mono", monospace;
}}
code {{
  border-radius: 4px;
  background: var(--code-background);
  padding: 0.1em 0.3em;
}}
pre {{
  overflow-x: auto;
  border: 1px solid var(--border);
  border-radius: 8px;
  background: var(--code-background);
  padding: 14px 16px;
}}
pre code {{
  background: transparent;
  padding: 0;
}}
blockquote {{
  border-left: 4px solid var(--border);
  background: var(--quote-background);
  color: var(--muted);
  padding: 10px 16px;
}}
table {{
  border-collapse: collapse;
  display: block;
  overflow-x: auto;
}}
th, td {{
  border: 1px solid var(--border);
  padding: 6px 10px;
}}
img {{
  max-width: 100%;
  height: auto;
}}
</style>
</head>
<body>
{}
</body>
</html>
"#,
        escape_html(title),
        body
    )
}

fn escape_html(value: &str) -> String {
    let mut escaped = String::new();

    for character in value.chars() {
        match character {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&#39;"),
            _ => escaped.push(character),
        }
    }

    escaped
}

#[cfg(test)]
mod tests {
    use super::markdown_to_html_document;

    #[test]
    fn html_export_renders_markdown_blocks() {
        let html = markdown_to_html_document(
            "# Title\n\nA **bold** paragraph.\n\n| A | B |\n| - | - |\n| 1 | 2 |\n",
            "sample.md",
        );

        assert!(html.contains("<h1>Title</h1>"));
        assert!(html.contains("<strong>bold</strong>"));
        assert!(html.contains("<table>"));
    }

    #[test]
    fn html_export_escapes_document_title() {
        let html = markdown_to_html_document("# Title", "a <b>& \"quote\".md");

        assert!(html.contains("<title>a &lt;b&gt;&amp; &quot;quote&quot;.md</title>"));
    }
}
