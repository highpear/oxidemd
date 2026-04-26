use std::fs;
use std::path::Path;

use pulldown_cmark::{html, Options, Parser};

pub fn write_html_export(source_path: &Path, output_path: &Path) -> Result<(), String> {
    let markdown = fs::read_to_string(source_path).map_err(|error| error.to_string())?;
    let title = source_path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("OxideMD Export");
    let base_href = html_base_href(source_path);
    let html = markdown_to_html_document(&markdown, title, base_href.as_deref());

    fs::write(output_path, html).map_err(|error| error.to_string())
}

fn markdown_to_html_document(markdown: &str, title: &str, base_href: Option<&str>) -> String {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);

    let parser = Parser::new_ext(markdown, options);
    let mut body = String::new();
    html::push_html(&mut body, parser);
    let base_tag = base_href
        .map(|href| format!(r#"<base href="{}">"#, escape_html(href)))
        .unwrap_or_default();

    format!(
        r#"<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>{}</title>
{}
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
        base_tag,
        body
    )
}

fn html_base_href(source_path: &Path) -> Option<String> {
    let source_dir = source_path.parent().unwrap_or_else(|| Path::new("."));
    let source_dir = source_dir.canonicalize().ok()?;

    Some(file_url_for_directory(&source_dir))
}

fn file_url_for_directory(path: &Path) -> String {
    let mut path = path.to_string_lossy().replace('\\', "/");
    path = normalize_windows_file_url_path(&path);

    if !path.ends_with('/') {
        path.push('/');
    }

    if path.starts_with("//") {
        format!("file:{}", percent_encode_file_url_path(&path))
    } else {
        format!("file:///{}", percent_encode_file_url_path(&path))
    }
}

fn normalize_windows_file_url_path(path: &str) -> String {
    if let Some(rest) = path.strip_prefix("//?/UNC/") {
        format!("//{rest}")
    } else if let Some(rest) = path.strip_prefix("//?/") {
        rest.to_owned()
    } else {
        path.to_owned()
    }
}

fn percent_encode_file_url_path(path: &str) -> String {
    let mut encoded = String::new();

    for byte in path.bytes() {
        if is_file_url_path_byte(byte) {
            encoded.push(byte as char);
        } else {
            encoded.push_str(&format!("%{byte:02X}"));
        }
    }

    encoded
}

fn is_file_url_path_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~' | b'/' | b':')
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
    use super::{
        file_url_for_directory, markdown_to_html_document, normalize_windows_file_url_path,
    };
    use std::path::Path;

    #[test]
    fn html_export_renders_markdown_blocks() {
        let html = markdown_to_html_document(
            "# Title\n\nA **bold** paragraph.\n\n| A | B |\n| - | - |\n| 1 | 2 |\n",
            "sample.md",
            None,
        );

        assert!(html.contains("<h1>Title</h1>"));
        assert!(html.contains("<strong>bold</strong>"));
        assert!(html.contains("<table>"));
    }

    #[test]
    fn html_export_escapes_document_title() {
        let html = markdown_to_html_document("# Title", "a <b>& \"quote\".md", None);

        assert!(html.contains("<title>a &lt;b&gt;&amp; &quot;quote&quot;.md</title>"));
    }

    #[test]
    fn html_export_includes_base_href_when_available() {
        let html = markdown_to_html_document(
            "![Image](./assets/image.png)",
            "sample.md",
            Some("file:///C:/Docs/Markdown/"),
        );

        assert!(html.contains(r#"<base href="file:///C:/Docs/Markdown/">"#));
        assert!(html.contains(r#"<img src="./assets/image.png" alt="Image" />"#));
    }

    #[test]
    fn file_url_for_directory_encodes_spaces_and_fragments() {
        let href = file_url_for_directory(Path::new(r"C:\Docs\Markdown Files\#drafts"));

        assert_eq!(href, "file:///C:/Docs/Markdown%20Files/%23drafts/");
    }

    #[test]
    fn normalize_windows_file_url_path_removes_verbatim_prefix() {
        assert_eq!(
            normalize_windows_file_url_path("//?/C:/Docs/Markdown"),
            "C:/Docs/Markdown"
        );
        assert_eq!(
            normalize_windows_file_url_path("//?/UNC/server/share/Docs"),
            "//server/share/Docs"
        );
    }
}
