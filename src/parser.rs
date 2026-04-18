use pulldown_cmark::{CodeBlockKind, Event, HeadingLevel, LinkType, Options, Parser, Tag, TagEnd};

pub struct MarkdownDocument {
    pub blocks: Vec<Block>,
}

#[derive(Clone)]
pub struct InlineContent {
    pub spans: Vec<InlineSpan>,
}

#[derive(Clone)]
pub enum InlineSpan {
    Text(String),
    Strong(String),
    Emphasis(String),
    Code(String),
    Link { text: String, destination: String },
    LineBreak,
}

pub enum Block {
    Heading {
        level: HeadingLevel,
        content: InlineContent,
    },
    Paragraph(InlineContent),
    UnorderedList(Vec<InlineContent>),
    OrderedList {
        start: u64,
        items: Vec<InlineContent>,
    },
    BlockQuote(Vec<InlineContent>),
    CodeBlock {
        language: Option<String>,
        code: String,
    },
}

pub fn parse_markdown(input: &str) -> MarkdownDocument {
    let mut parser = Parser::new_ext(input, Options::empty()).peekable();
    let mut blocks = Vec::new();

    while let Some(event) = parser.next() {
        match event {
            Event::Start(Tag::Heading { level, .. }) => {
                let content = collect_inline_content(&mut parser, TagEnd::Heading(level));
                if !content.is_empty() {
                    blocks.push(Block::Heading { level, content });
                }
            }
            Event::Start(Tag::Paragraph) => {
                let content = collect_inline_content(&mut parser, TagEnd::Paragraph);
                if !content.is_empty() {
                    blocks.push(Block::Paragraph(content));
                }
            }
            Event::Start(Tag::List(start)) => {
                let ordered_start = start.unwrap_or(1);
                let items = collect_list_items(&mut parser);

                if start.is_some() {
                    blocks.push(Block::OrderedList {
                        start: ordered_start,
                        items,
                    });
                } else {
                    blocks.push(Block::UnorderedList(items));
                }
            }
            Event::Start(Tag::BlockQuote(_)) => {
                let lines = collect_blockquote_lines(&mut parser);
                if !lines.is_empty() {
                    blocks.push(Block::BlockQuote(lines));
                }
            }
            Event::Start(Tag::CodeBlock(kind)) => {
                let language = match kind {
                    CodeBlockKind::Fenced(name) => {
                        let trimmed = name.trim();
                        if trimmed.is_empty() {
                            None
                        } else {
                            Some(trimmed.to_owned())
                        }
                    }
                    CodeBlockKind::Indented => None,
                };

                let code = collect_text_until(&mut parser, TagEnd::CodeBlock);
                blocks.push(Block::CodeBlock { language, code });
            }
            _ => {}
        }
    }

    MarkdownDocument { blocks }
}

fn collect_text_until<'a, I>(parser: &mut I, end_tag: TagEnd) -> String
where
    I: Iterator<Item = Event<'a>>,
{
    let mut text = String::new();

    for event in parser.by_ref() {
        match event {
            Event::End(tag) if tag == end_tag => break,
            Event::Text(value) | Event::Code(value) => text.push_str(value.as_ref()),
            Event::SoftBreak | Event::HardBreak => text.push('\n'),
            _ => {}
        }
    }

    text.trim().to_owned()
}

fn collect_inline_content<'a, I>(parser: &mut I, end_tag: TagEnd) -> InlineContent
where
    I: Iterator<Item = Event<'a>>,
{
    let mut spans = Vec::new();

    while let Some(event) = parser.next() {
        match event {
            Event::End(tag) if tag == end_tag => break,
            Event::Text(value) => push_text_span(&mut spans, value.as_ref()),
            Event::Code(value) => spans.push(InlineSpan::Code(value.to_string())),
            Event::SoftBreak | Event::HardBreak => spans.push(InlineSpan::LineBreak),
            Event::Start(Tag::Strong) => {
                let text = collect_text_until(parser, TagEnd::Strong);
                if !text.is_empty() {
                    spans.push(InlineSpan::Strong(text));
                }
            }
            Event::Start(Tag::Emphasis) => {
                let text = collect_text_until(parser, TagEnd::Emphasis);
                if !text.is_empty() {
                    spans.push(InlineSpan::Emphasis(text));
                }
            }
            Event::Start(Tag::Link {
                link_type,
                dest_url,
                ..
            }) => {
                let link = collect_link_span(parser, link_type, dest_url.to_string());
                if let Some(link) = link {
                    spans.push(link);
                }
            }
            _ => {}
        }
    }

    InlineContent { spans }
}

fn collect_list_items<'a, I>(parser: &mut std::iter::Peekable<I>) -> Vec<InlineContent>
where
    I: Iterator<Item = Event<'a>>,
{
    let mut items = Vec::new();

    while let Some(event) = parser.next() {
        match event {
            Event::Start(Tag::Item) => {
                let content = collect_list_item(parser);
                if !content.is_empty() {
                    items.push(content);
                }
            }
            Event::End(TagEnd::List(_)) => break,
            _ => {}
        }
    }

    items
}

fn collect_list_item<'a, I>(parser: &mut std::iter::Peekable<I>) -> InlineContent
where
    I: Iterator<Item = Event<'a>>,
{
    let mut spans = Vec::new();

    while let Some(event) = parser.next() {
        match event {
            Event::Start(Tag::Paragraph) => {
                let content = collect_inline_content(parser, TagEnd::Paragraph);
                spans.extend(content.spans);
            }
            Event::Text(value) => push_text_span(&mut spans, value.as_ref()),
            Event::Code(value) => spans.push(InlineSpan::Code(value.to_string())),
            Event::SoftBreak | Event::HardBreak => spans.push(InlineSpan::LineBreak),
            Event::Start(Tag::Strong) => {
                let text = collect_text_until(parser, TagEnd::Strong);
                if !text.is_empty() {
                    spans.push(InlineSpan::Strong(text));
                }
            }
            Event::Start(Tag::Emphasis) => {
                let text = collect_text_until(parser, TagEnd::Emphasis);
                if !text.is_empty() {
                    spans.push(InlineSpan::Emphasis(text));
                }
            }
            Event::Start(Tag::Link {
                link_type,
                dest_url,
                ..
            }) => {
                let link = collect_link_span(parser, link_type, dest_url.to_string());
                if let Some(link) = link {
                    spans.push(link);
                }
            }
            Event::End(TagEnd::Item) => break,
            _ => {}
        }
    }

    InlineContent { spans }
}

fn collect_blockquote_lines<'a, I>(parser: &mut std::iter::Peekable<I>) -> Vec<InlineContent>
where
    I: Iterator<Item = Event<'a>>,
{
    let mut lines = Vec::new();

    while let Some(event) = parser.next() {
        match event {
            Event::Start(Tag::Paragraph) => {
                let content = collect_inline_content(parser, TagEnd::Paragraph);
                if !content.is_empty() {
                    lines.push(content);
                }
            }
            Event::End(TagEnd::BlockQuote(_)) => break,
            _ => {}
        }
    }

    lines
}

fn collect_link_span<'a, I>(
    parser: &mut I,
    _link_type: LinkType,
    destination: String,
) -> Option<InlineSpan>
where
    I: Iterator<Item = Event<'a>>,
{
    let text = collect_text_until(parser, TagEnd::Link);
    if text.is_empty() {
        None
    } else {
        Some(InlineSpan::Link { text, destination })
    }
}

fn push_text_span(spans: &mut Vec<InlineSpan>, text: &str) {
    if text.is_empty() {
        return;
    }

    spans.push(InlineSpan::Text(text.to_owned()));
}

impl InlineContent {
    pub fn is_empty(&self) -> bool {
        self.spans.iter().all(|span| match span {
            InlineSpan::Text(text)
            | InlineSpan::Strong(text)
            | InlineSpan::Emphasis(text)
            | InlineSpan::Code(text) => text.is_empty(),
            InlineSpan::Link { text, .. } => text.is_empty(),
            InlineSpan::LineBreak => false,
        })
    }
}
