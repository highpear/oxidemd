use crate::svg::SvgAsset;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EmbeddedSvgContentKind {
    Math,
    Diagram,
}

#[derive(Clone)]
pub struct EmbeddedSvgContent {
    kind: EmbeddedSvgContentKind,
    asset: SvgAsset,
    source_text: String,
}

impl EmbeddedSvgContent {
    pub fn new(kind: EmbeddedSvgContentKind, asset: SvgAsset, source_text: String) -> Self {
        Self {
            kind,
            asset,
            source_text,
        }
    }

    pub fn kind(&self) -> EmbeddedSvgContentKind {
        self.kind
    }

    pub fn asset(&self) -> &SvgAsset {
        &self.asset
    }

    pub fn source_text(&self) -> &str {
        &self.source_text
    }

    pub fn source_action(&self) -> EmbeddedSourceAction<'_> {
        EmbeddedSourceAction::new(self.source_text())
    }
}

#[derive(Clone, Copy)]
pub struct EmbeddedSourceAction<'a> {
    source_text: &'a str,
}

impl<'a> EmbeddedSourceAction<'a> {
    pub fn new(source_text: &'a str) -> Self {
        Self { source_text }
    }

    pub fn source_text(&self) -> &'a str {
        self.source_text
    }
}

#[cfg(test)]
mod tests {
    use super::{EmbeddedSourceAction, EmbeddedSvgContent, EmbeddedSvgContentKind};
    use crate::svg::SvgAsset;

    #[test]
    fn content_tracks_kind_and_source_action_separately_from_svg_asset() {
        let asset = SvgAsset::from_source(
            "bytes://test.svg".to_owned(),
            "<svg viewBox=\"0 0 1 1\"><g /></svg>".to_owned(),
        )
        .expect("valid test SVG");
        let content =
            EmbeddedSvgContent::new(EmbeddedSvgContentKind::Math, asset, "x^2".to_owned());

        assert_eq!(content.kind(), EmbeddedSvgContentKind::Math);
        assert_eq!(content.asset().uri(), "bytes://test.svg");
        assert_eq!(content.source_action().source_text(), "x^2");
    }

    #[test]
    fn source_action_can_wrap_fallback_source_text() {
        let action = EmbeddedSourceAction::new("fallback");

        assert_eq!(action.source_text(), "fallback");
    }
}
