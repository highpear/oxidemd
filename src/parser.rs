use pulldown_cmark::{CodeBlockKind, Event, HeadingLevel, Options, Parser, Tag, TagEnd};

pub struct MarkdownDocument {
    pub blocks: Vec<Block>,
}

pub enum Block {
    Heading {
        level: HeadingLevel,
        text: String,
    },
    Paragraph(String),
    UnorderedList(Vec<String>),
    OrderedList {
        start: u64,
        items: Vec<String>,
    },
    BlockQuote(Vec<String>),
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
                let text = collect_text_until(&mut parser, TagEnd::Heading(level));
                blocks.push(Block::Heading { level, text });
            }
            Event::Start(Tag::Paragraph) => {
                let text = collect_text_until(&mut parser, TagEnd::Paragraph);
                if !text.is_empty() {
                    blocks.push(Block::Paragraph(text));
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

fn collect_list_items<'a, I>(parser: &mut std::iter::Peekable<I>) -> Vec<String>
where
    I: Iterator<Item = Event<'a>>,
{
    let mut items = Vec::new();

    while let Some(event) = parser.next() {
        match event {
            Event::Start(Tag::Item) => {
                let text = collect_text_until(parser, TagEnd::Item);
                if !text.is_empty() {
                    items.push(text);
                }
            }
            Event::End(TagEnd::List(_)) => break,
            _ => {}
        }
    }

    items
}

fn collect_blockquote_lines<'a, I>(parser: &mut std::iter::Peekable<I>) -> Vec<String>
where
    I: Iterator<Item = Event<'a>>,
{
    let mut lines = Vec::new();

    while let Some(event) = parser.next() {
        match event {
            Event::Start(Tag::Paragraph) => {
                let text = collect_text_until(parser, TagEnd::Paragraph);
                if !text.is_empty() {
                    lines.push(text);
                }
            }
            Event::End(TagEnd::BlockQuote(_)) => break,
            Event::Text(value) | Event::Code(value) => {
                let text = value.trim();
                if !text.is_empty() {
                    lines.push(text.to_owned());
                }
            }
            Event::SoftBreak | Event::HardBreak => {}
            _ => {}
        }
    }

    lines
}
