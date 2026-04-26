use eframe::egui::{Color32, Vec2, load::Bytes, vec2};

#[derive(Clone)]
pub struct SvgAsset {
    uri: String,
    bytes: Bytes,
    size: Vec2,
}

impl SvgAsset {
    pub fn from_source(uri: String, svg_source: String) -> Result<Self, String> {
        let size = svg_size(&svg_source)?;
        Ok(Self {
            uri,
            bytes: Bytes::from(svg_source.into_bytes()),
            size,
        })
    }

    pub fn uri(&self) -> &str {
        &self.uri
    }

    pub fn bytes(&self) -> Bytes {
        self.bytes.clone()
    }

    pub fn size(&self) -> Vec2 {
        self.size
    }
}

pub fn apply_current_color(svg: &str, color: Color32) -> String {
    let Some(start) = svg.find("<svg") else {
        return svg.to_owned();
    };
    let Some(end) = svg[start..].find('>') else {
        return svg.to_owned();
    };
    let tag_end = start + end;
    let color_value = format!("#{:02x}{:02x}{:02x}", color.r(), color.g(), color.b());
    let svg_tag = &svg[start..=tag_end];

    if let Some(style_index) = svg_tag.find(" style=\"") {
        let value_start = start + style_index + " style=\"".len();
        if let Some(value_end_offset) = svg[value_start..].find('"') {
            let value_end = value_start + value_end_offset;
            let mut colored = String::with_capacity(svg.len() + 32);
            colored.push_str(&svg[..value_start]);
            let existing_style = svg[value_start..value_end].trim();
            if existing_style.is_empty() {
                colored.push_str(&format!("color:{color_value}"));
            } else if existing_style.ends_with(';') {
                colored.push_str(existing_style);
                colored.push_str(&format!("color:{color_value}"));
            } else {
                colored.push_str(existing_style);
                colored.push_str(&format!("; color:{color_value}"));
            }
            colored.push_str(&svg[value_end..]);
            return colored;
        }
    }

    let mut colored = String::with_capacity(svg.len() + 32);
    colored.push_str(&svg[..tag_end]);
    colored.push_str(&format!(" style=\"color:{color_value}\""));
    colored.push_str(&svg[tag_end..]);
    colored
}

fn svg_size(svg: &str) -> Result<Vec2, String> {
    let tree = resvg::usvg::Tree::from_str(svg, &resvg::usvg::Options::default())
        .map_err(|e| e.to_string())?;
    let size = tree.size();
    Ok(vec2(size.width(), size.height()))
}

#[cfg(test)]
mod tests {
    use super::apply_current_color;
    use eframe::egui::Color32;

    #[test]
    fn appends_style_when_missing() {
        let svg = "<svg viewBox=\"0 0 1 1\"><g /></svg>";
        let colored = apply_current_color(svg, Color32::from_rgb(1, 2, 3));

        assert!(colored.contains("style=\"color:#010203\""));
    }

    #[test]
    fn merges_with_existing_style() {
        let svg = "<svg style=\"vertical-align: -0.452ex;\" viewBox=\"0 0 1 1\"><g /></svg>";
        let colored = apply_current_color(svg, Color32::from_rgb(1, 2, 3));

        assert!(colored.contains("style=\"vertical-align: -0.452ex;color:#010203\""));
        assert_eq!(colored.matches("style=\"").count(), 1);
    }
}
