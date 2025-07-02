use anyhow::Result;
use pulldown_cmark::{Event, Parser, Tag, TagEnd, Options};
use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
};
use regex::Regex;

#[derive(Debug, Clone)]
pub enum MarkdownElement {
    Heading { level: u8, text: String },
    Paragraph { text: String },
    CodeBlock { language: Option<String>, code: String },
    InlineCode { text: String },
    Link { text: String, url: String },
    Bold { text: String },
    Italic { text: String },
    List { items: Vec<String>, ordered: bool },
    BlockQuote { text: String },
    Rule,
    Text { text: String },
    Table { headers: Vec<String>, rows: Vec<Vec<String>>, alignments: Vec<TableAlignment> },
}

#[derive(Debug, Clone)]
pub enum TableAlignment {
    Left,
    Center,
    Right,
    None,
}

pub struct MarkdownRenderer {
    code_block_regex: Regex,
}

impl Default for MarkdownRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl MarkdownRenderer {
    pub fn new() -> Self {
        Self {
            code_block_regex: Regex::new(r"```(\w+)?\n((?s:.)*?)```").unwrap(),
        }
    }

    pub fn parse_markdown(&self, markdown: &str) -> Result<Vec<MarkdownElement>> {
        // Use pulldown-cmark with table support enabled
        let mut options = Options::empty();
        options.insert(Options::ENABLE_TABLES);
        options.insert(Options::ENABLE_STRIKETHROUGH);
        
        let parser = Parser::new_ext(markdown, options);
        let mut elements = Vec::new();
        let mut current_text = String::new();
        let mut in_heading = None;
        let mut in_paragraph = false;
        let mut in_code_block = false;
        let mut code_lang = None;
        let mut in_bold = false;
        let mut in_italic = false;
        let mut in_link = false;
        let mut link_url = String::new();
        let mut in_blockquote = false;
        let mut list_items = Vec::new();
        let mut in_list = false;
        let mut is_ordered_list = false;
        
        // Table handling
        let mut in_table = false;
        let mut table_headers = Vec::new();
        let mut table_rows = Vec::new();
        let mut current_row = Vec::new();
        let mut table_alignments = Vec::new();

        for event in parser {
            match event {
                Event::Start(tag) => match tag {
                    Tag::Heading { level, .. } => {
                        if in_paragraph {
                            elements.push(MarkdownElement::Paragraph {
                                text: current_text.trim().to_string(),
                            });
                            current_text.clear();
                            in_paragraph = false;
                        }
                        in_heading = Some(level as u8);
                    }
                    Tag::Paragraph => {
                        if !in_list && !in_blockquote {
                            // Check if this paragraph contains a table marker
                            if !current_text.contains("__TABLE__") {
                                in_paragraph = true;
                            }
                        }
                    }
                    Tag::CodeBlock(kind) => {
                        in_code_block = true;
                        code_lang = match kind {
                            pulldown_cmark::CodeBlockKind::Fenced(lang) => {
                                if lang.is_empty() { None } else { Some(lang.to_string()) }
                            }
                            _ => None,
                        };
                    }
                    Tag::Strong => in_bold = true,
                    Tag::Emphasis => in_italic = true,
                    Tag::Link { dest_url, .. } => {
                        in_link = true;
                        link_url = dest_url.to_string();
                    }
                    Tag::BlockQuote(_) => in_blockquote = true,
                    Tag::List(start) => {
                        in_list = true;
                        is_ordered_list = start.is_some();
                        list_items.clear();
                    }
                    Tag::Item => {
                        // Start of list item
                    }
                    Tag::Table(alignments) => {
                        in_table = true;
                        table_alignments = alignments.iter().map(|a| match a {
                            pulldown_cmark::Alignment::Left => TableAlignment::Left,
                            pulldown_cmark::Alignment::Center => TableAlignment::Center,
                            pulldown_cmark::Alignment::Right => TableAlignment::Right,
                            pulldown_cmark::Alignment::None => TableAlignment::Left,
                        }).collect();
                        table_headers.clear();
                        table_rows.clear();
                    }
                    Tag::TableHead => {
                        // Start of table header
                    }
                    Tag::TableRow => {
                        current_row.clear();
                    }
                    Tag::TableCell => {
                        // Start of table cell
                    }
                    _ => {}
                },
                Event::End(tag_end) => match tag_end {
                    TagEnd::Heading(_level) => {
                        if let Some(h_level) = in_heading {
                            elements.push(MarkdownElement::Heading {
                                level: h_level,
                                text: current_text.trim().to_string(),
                            });
                            current_text.clear();
                            in_heading = None;
                        }
                    }
                    TagEnd::Paragraph => {
                        if in_paragraph {
                            elements.push(MarkdownElement::Paragraph {
                                text: current_text.trim().to_string(),
                            });
                            current_text.clear();
                            in_paragraph = false;
                        } else if in_list && !current_text.trim().is_empty() {
                            list_items.push(current_text.trim().to_string());
                            current_text.clear();
                        } else if in_blockquote {
                            elements.push(MarkdownElement::BlockQuote {
                                text: current_text.trim().to_string(),
                            });
                            current_text.clear();
                        }
                    }
                    TagEnd::CodeBlock => {
                        elements.push(MarkdownElement::CodeBlock {
                            language: code_lang.clone(),
                            code: current_text.trim_end().to_string(),
                        });
                        current_text.clear();
                        in_code_block = false;
                        code_lang = None;
                    }
                    TagEnd::Strong => in_bold = false,
                    TagEnd::Emphasis => in_italic = false,
                    TagEnd::Link => {
                        elements.push(MarkdownElement::Link {
                            text: current_text.clone(),
                            url: link_url.clone(),
                        });
                        current_text.clear();
                        in_link = false;
                        link_url.clear();
                    }
                    TagEnd::BlockQuote(_) => in_blockquote = false,
                    TagEnd::List(_) => {
                        if !list_items.is_empty() {
                            elements.push(MarkdownElement::List {
                                items: list_items.clone(),
                                ordered: is_ordered_list,
                            });
                            list_items.clear();
                        }
                        in_list = false;
                    }
                    TagEnd::Item => {
                        if !current_text.trim().is_empty() {
                            list_items.push(current_text.trim().to_string());
                            current_text.clear();
                        }
                    }
                    TagEnd::Table => {
                        if in_table {
                            elements.push(MarkdownElement::Table {
                                headers: table_headers.clone(),
                                rows: table_rows.clone(),
                                alignments: table_alignments.clone(),
                            });
                            in_table = false;
                        }
                    }
                    TagEnd::TableHead => {
                        // End of table header
                    }
                    TagEnd::TableRow => {
                        if in_table {
                            if table_headers.is_empty() {
                                // This is the header row
                                table_headers = current_row.clone();
                            } else {
                                // This is a data row
                                table_rows.push(current_row.clone());
                            }
                        }
                    }
                    TagEnd::TableCell => {
                        if in_table {
                            current_row.push(current_text.trim().to_string());
                            current_text.clear();
                        }
                    }
                    _ => {}
                },
                Event::Text(text) => {
                    current_text.push_str(&text);
                }
                Event::Code(code) => {
                    if !in_code_block {
                        elements.push(MarkdownElement::InlineCode {
                            text: code.to_string(),
                        });
                    } else {
                        current_text.push_str(&code);
                    }
                }
                Event::Rule => {
                    elements.push(MarkdownElement::Rule);
                }
                _ => {}
            }
        }

        // Handle any remaining text
        if !current_text.trim().is_empty() {
            if in_paragraph {
                elements.push(MarkdownElement::Paragraph {
                    text: current_text.trim().to_string(),
                });
            } else if !in_list {
                elements.push(MarkdownElement::Text {
                    text: current_text.trim().to_string(),
                });
            }
        }

        Ok(elements)
    }

    fn parse_tables_manually(&self, markdown: &str) -> String {
        let lines: Vec<&str> = markdown.lines().collect();
        let mut result = Vec::new();
        let mut i = 0;

        while i < lines.len() {
            let line = lines[i].trim();
            
            // Check if this line looks like a table header
            if line.contains('|') && i + 1 < lines.len() {
                let next_line = lines[i + 1].trim();
                // Check if next line is a separator (contains | and -)
                if next_line.contains('|') && next_line.contains('-') {
                    // Found a table!
                    let (table_element, consumed_lines) = self.parse_single_table(&lines[i..]);
                    
                    if let Some(table) = table_element {
                        // Add the table to our elements
                        let table_text = self.render_table_as_text(&table);
                        result.push(table_text);
                    }
                    
                    i += consumed_lines;
                } else {
                    result.push(lines[i].to_string());
                    i += 1;
                }
            } else {
                result.push(lines[i].to_string());
                i += 1;
            }
        }

        result.join("\n")
    }

    fn parse_single_table(&self, lines: &[&str]) -> (Option<MarkdownElement>, usize) {
        if lines.len() < 2 {
            return (None, 0);
        }

        let header_line = lines[0].trim();
        let separator_line = lines[1].trim();

        // Parse headers
        let headers: Vec<String> = header_line
            .split('|')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        if headers.is_empty() {
            return (None, 0);
        }

        // Parse alignment from separator
        let alignments: Vec<TableAlignment> = separator_line
            .split('|')
            .filter(|s| !s.trim().is_empty())
            .map(|s| {
                let trimmed = s.trim();
                if trimmed.starts_with(':') && trimmed.ends_with(':') {
                    TableAlignment::Center
                } else if trimmed.ends_with(':') {
                    TableAlignment::Right
                } else {
                    TableAlignment::Left
                }
            })
            .collect();

        // Parse rows
        let mut rows = Vec::new();
        let mut consumed = 2; // header + separator

        for &line in &lines[2..] {
            let trimmed = line.trim();
            
            // Stop if we hit an empty line or a line without |
            if trimmed.is_empty() || !trimmed.contains('|') {
                break;
            }

            let row: Vec<String> = trimmed
                .split('|')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();

            if !row.is_empty() {
                rows.push(row);
                consumed += 1;
            } else {
                break;
            }
        }

        let table = MarkdownElement::Table {
            headers,
            rows,
            alignments,
        };

        (Some(table), consumed)
    }

    fn render_table_as_text(&self, table: &MarkdownElement) -> String {
        if let MarkdownElement::Table { headers, rows, alignments: _alignments } = table {
            let mut result = Vec::new();
            
            // Calculate column widths
            let mut col_widths = Vec::new();
            for (i, header) in headers.iter().enumerate() {
                let mut max_width = header.len();
                for row in rows {
                    if let Some(cell) = row.get(i) {
                        max_width = max_width.max(cell.len());
                    }
                }
                col_widths.push(max_width + 2); // Add padding
            }

            // Top border
            let mut top_line = "┌".to_string();
            for (i, &width) in col_widths.iter().enumerate() {
                top_line.push_str(&"─".repeat(width));
                if i < col_widths.len() - 1 {
                    top_line.push_str("┬");
                }
            }
            top_line.push_str("┐");
            result.push(top_line);

            // Header row
            let mut header_line = "│".to_string();
            for (i, header) in headers.iter().enumerate() {
                let width = col_widths[i];
                header_line.push_str(&format!(" {:<width$}", header, width = width - 1));
                header_line.push_str("│");
            }
            result.push(header_line);

            // Separator
            let mut sep_line = "├".to_string();
            for (i, &width) in col_widths.iter().enumerate() {
                sep_line.push_str(&"─".repeat(width));
                if i < col_widths.len() - 1 {
                    sep_line.push_str("┼");
                }
            }
            sep_line.push_str("┤");
            result.push(sep_line);

            // Data rows
            for row in rows {
                let mut row_line = "│".to_string();
                for (i, _) in headers.iter().enumerate() {
                    let width = col_widths[i];
                    let cell_content = row.get(i).cloned().unwrap_or_default();
                    row_line.push_str(&format!(" {:<width$}", cell_content, width = width - 1));
                    row_line.push_str("│");
                }
                result.push(row_line);
            }

            // Bottom border
            let mut bottom_line = "└".to_string();
            for (i, &width) in col_widths.iter().enumerate() {
                bottom_line.push_str(&"─".repeat(width));
                if i < col_widths.len() - 1 {
                    bottom_line.push_str("┴");
                }
            }
            bottom_line.push_str("┘");
            result.push(bottom_line);

            result.join("\n")
        } else {
            String::new()
        }
    }

    pub fn render_to_text(&self, elements: &[MarkdownElement]) -> Text<'static> {
        let mut lines = Vec::new();

        for element in elements {
            match element {
                MarkdownElement::Heading { level, text } => {
                    // Add spacing before headings (except for the first element)
                    if !lines.is_empty() {
                        lines.push(Line::from(""));
                    }

                    let style = match level {
                        1 => Style::default()
                            .fg(Color::Red)
                            .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
                        2 => Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD),
                        3 => Style::default()
                            .fg(Color::Green)
                            .add_modifier(Modifier::BOLD),
                        4 => Style::default()
                            .fg(Color::Blue)
                            .add_modifier(Modifier::BOLD),
                        5 => Style::default()
                            .fg(Color::Magenta)
                            .add_modifier(Modifier::BOLD),
                        _ => Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                    };

                    let prefix = "#".repeat(*level as usize);
                    lines.push(Line::from(vec![
                        Span::styled(format!("{} ", prefix), Style::default().fg(Color::DarkGray)),
                        Span::styled(text.clone(), style),
                    ]));
                    lines.push(Line::from(""));
                }
                MarkdownElement::Paragraph { text } => {
                    lines.extend(self.wrap_text_with_inline_formatting(text, 80));
                    lines.push(Line::from(""));
                }
                MarkdownElement::CodeBlock { language, code } => {
                    // Add spacing before code blocks
                    if !lines.is_empty() {
                        lines.push(Line::from(""));
                    }

                    // Language label if present
                    if let Some(lang) = language {
                        lines.push(Line::from(vec![
                            Span::styled("```".to_string(), Style::default().fg(Color::DarkGray)),
                            Span::styled(lang.clone(), Style::default().fg(Color::Yellow)),
                        ]));
                    } else {
                        lines.push(Line::from(Span::styled("```".to_string(), Style::default().fg(Color::DarkGray))));
                    }

                    // Code content
                    for line in code.lines() {
                        lines.push(Line::from(Span::styled(
                            format!("  {}", line),
                            Style::default().fg(Color::Green).bg(Color::Black),
                        )));
                    }

                    lines.push(Line::from(Span::styled("```".to_string(), Style::default().fg(Color::DarkGray))));
                    lines.push(Line::from(""));
                }
                MarkdownElement::InlineCode { text } => {
                    lines.push(Line::from(Span::styled(
                        format!("`{}`", text),
                        Style::default().fg(Color::Green).bg(Color::Black),
                    )));
                }
                MarkdownElement::Link { text, url: _url } => {
                    lines.push(Line::from(Span::styled(
                        format!("[{}]", text),
                        Style::default().fg(Color::Blue).add_modifier(Modifier::UNDERLINED),
                    )));
                }
                MarkdownElement::List { items, ordered } => {
                    for (i, item) in items.iter().enumerate() {
                        let prefix = if *ordered {
                            format!("{}. ", i + 1)
                        } else {
                            "• ".to_string()
                        };

                        lines.push(Line::from(vec![
                            Span::styled(prefix, Style::default().fg(Color::Yellow)),
                            Span::raw(item.clone()),
                        ]));
                    }
                    lines.push(Line::from(""));
                }
                MarkdownElement::BlockQuote { text } => {
                    for line in text.lines() {
                        lines.push(Line::from(vec![
                            Span::styled("▎ ".to_string(), Style::default().fg(Color::Blue)),
                            Span::styled(line.to_string(), Style::default().fg(Color::Gray).add_modifier(Modifier::ITALIC)),
                        ]));
                    }
                    lines.push(Line::from(""));
                }
                MarkdownElement::Rule => {
                    lines.push(Line::from(Span::styled(
                        "─".repeat(60),
                        Style::default().fg(Color::DarkGray),
                    )));
                    lines.push(Line::from(""));
                }
                MarkdownElement::Text { text } => {
                    lines.extend(self.wrap_text_with_inline_formatting(text, 80));
                }
                MarkdownElement::Table { headers, rows, alignments: _alignments } => {
                    // Add spacing before table
                    if !lines.is_empty() {
                        lines.push(Line::from(""));
                    }

                    // Calculate column widths
                    let mut col_widths = Vec::new();
                    for (i, header) in headers.iter().enumerate() {
                        let mut max_width = header.len();
                        for row in rows {
                            if let Some(cell) = row.get(i) {
                                max_width = max_width.max(cell.len());
                            }
                        }
                        col_widths.push(max_width + 2); // Add padding
                    }

                    // Render table top border
                    let mut top_spans = vec![Span::styled("┌".to_string(), Style::default().fg(Color::Cyan))];
                    for (i, _) in headers.iter().enumerate() {
                        let width = col_widths.get(i).unwrap_or(&10);
                        top_spans.push(Span::styled("─".repeat(*width), Style::default().fg(Color::Cyan)));
                        if i < headers.len() - 1 {
                            top_spans.push(Span::styled("┬".to_string(), Style::default().fg(Color::Cyan)));
                        }
                    }
                    top_spans.push(Span::styled("┐".to_string(), Style::default().fg(Color::Cyan)));
                    lines.push(Line::from(top_spans));

                    // Render table header
                    let mut header_spans = vec![Span::styled("│".to_string(), Style::default().fg(Color::Cyan))];
                    for (i, header) in headers.iter().enumerate() {
                        let width = col_widths.get(i).unwrap_or(&10);
                        let padded_header = format!(" {:<width$}", header, width = width - 1);
                        header_spans.push(Span::styled(padded_header, Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)));
                        header_spans.push(Span::styled("│".to_string(), Style::default().fg(Color::Cyan)));
                    }
                    lines.push(Line::from(header_spans));

                    // Render table separator
                    let mut separator_spans = vec![Span::styled("├".to_string(), Style::default().fg(Color::Cyan))];
                    for (i, _) in headers.iter().enumerate() {
                        let width = col_widths.get(i).unwrap_or(&10);
                        separator_spans.push(Span::styled("─".repeat(*width), Style::default().fg(Color::Cyan)));
                        if i < headers.len() - 1 {
                            separator_spans.push(Span::styled("┼".to_string(), Style::default().fg(Color::Cyan)));
                        }
                    }
                    separator_spans.push(Span::styled("┤".to_string(), Style::default().fg(Color::Cyan)));
                    lines.push(Line::from(separator_spans));

                    // Render table rows
                    for row in rows {
                        let mut row_spans = vec![Span::styled("│".to_string(), Style::default().fg(Color::Cyan))];
                        for (i, _) in headers.iter().enumerate() {
                            let width = col_widths.get(i).unwrap_or(&10);
                            let cell_content = row.get(i).cloned().unwrap_or_default();
                            let padded_cell = format!(" {:<width$}", cell_content, width = width - 1);
                            row_spans.push(Span::styled(padded_cell, Style::default().fg(Color::White)));
                            row_spans.push(Span::styled("│".to_string(), Style::default().fg(Color::Cyan)));
                        }
                        lines.push(Line::from(row_spans));
                    }

                    // Render table bottom border
                    let mut bottom_spans = vec![Span::styled("└".to_string(), Style::default().fg(Color::Cyan))];
                    for (i, _) in headers.iter().enumerate() {
                        let width = col_widths.get(i).unwrap_or(&10);
                        bottom_spans.push(Span::styled("─".repeat(*width), Style::default().fg(Color::Cyan)));
                        if i < headers.len() - 1 {
                            bottom_spans.push(Span::styled("┴".to_string(), Style::default().fg(Color::Cyan)));
                        }
                    }
                    bottom_spans.push(Span::styled("┘".to_string(), Style::default().fg(Color::Cyan)));
                    lines.push(Line::from(bottom_spans));
                    lines.push(Line::from(""));
                }
                _ => {}
            }
        }

        Text::from(lines)
    }

    fn wrap_text_with_inline_formatting(&self, text: &str, width: usize) -> Vec<Line<'static>> {
        let mut lines = Vec::new();
        let mut current_line = Vec::new();
        let mut current_length = 0;

        // Simple word wrapping with inline markdown support
        for word in text.split_whitespace() {
            let word_len = word.len();
            
            if current_length + word_len + 1 > width && !current_line.is_empty() {
                lines.push(Line::from(current_line.clone()));
                current_line.clear();
                current_length = 0;
            }

            if !current_line.is_empty() {
                current_line.push(Span::raw(" ".to_string()));
                current_length += 1;
            }

            // Check for inline formatting
            if word.starts_with("**") && word.ends_with("**") && word.len() > 4 {
                // Bold text
                let content = &word[2..word.len()-2];
                current_line.push(Span::styled(
                    content.to_string(),
                    Style::default().add_modifier(Modifier::BOLD),
                ));
            } else if word.starts_with('*') && word.ends_with('*') && word.len() > 2 {
                // Italic text
                let content = &word[1..word.len()-1];
                current_line.push(Span::styled(
                    content.to_string(),
                    Style::default().add_modifier(Modifier::ITALIC),
                ));
            } else if word.starts_with('`') && word.ends_with('`') && word.len() > 2 {
                // Inline code
                let content = &word[1..word.len()-1];
                current_line.push(Span::styled(
                    content.to_string(),
                    Style::default().fg(Color::Green).bg(Color::Black),
                ));
            } else {
                current_line.push(Span::raw(word.to_string()));
            }

            current_length += word_len;
        }

        if !current_line.is_empty() {
            lines.push(Line::from(current_line));
        }

        if lines.is_empty() {
            lines.push(Line::from("".to_string()));
        }

        lines
    }
}
