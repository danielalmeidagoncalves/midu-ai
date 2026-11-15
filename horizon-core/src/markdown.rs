//! A small, careful markdown parser.
//!
//! The parser produces an AST. The UI turns that AST into typed nodes and text, never markup.
//! Raw HTML is therefore plain text, and links are filtered to http(s) and mailto before they
//! ever reach an `href`.

use chrono::NaiveDate;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Inline {
    Text(String),
    Bold(Vec<Inline>),
    Italic(Vec<Inline>),
    Code(String),
    Link {
        label: Vec<Inline>,
        /// `None` means the scheme was not safe; the label prints, the link does not.
        href: Option<String>,
    },
    Person(String),
    DayRef {
        day: String,
        label: Option<String>,
        num: u32,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ListItem {
    pub content: Vec<Inline>,
    /// `Some(done)` for `- [ ]` / `- [x]`; `None` for an ordinary list item.
    pub task: Option<bool>,
    /// The zero-based source line, so a reading-view tick can rewrite the right line.
    pub line: Option<usize>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Block {
    Paragraph(Vec<Inline>),
    Heading {
        level: u8,
        content: Vec<Inline>,
    },
    List {
        ordered: bool,
        items: Vec<ListItem>,
    },
    Quote(Vec<Inline>),
    Table {
        head: Vec<Vec<Inline>>,
        rows: Vec<Vec<Vec<Inline>>>,
    },
    Rule,
}

pub fn parse(src: &str) -> Vec<Block> {
    let normalized = src
        .replace("\r\n", "\n")
        .replace('\r', "\n")
        .replace('\t', "  ");
    let lines: Vec<&str> = normalized.split('\n').collect();
    let mut blocks = Vec::new();
    let mut i = 0;

    while i < lines.len() {
        let line = lines[i];
        if line.trim().is_empty() {
            i += 1;
            continue;
        }

        if is_rule(line) {
            blocks.push(Block::Rule);
            i += 1;
            continue;
        }

        if let Some((level, rest)) = heading(line) {
            blocks.push(Block::Heading {
                level,
                content: parse_inline(rest.trim_end_matches('#').trim_end()),
            });
            i += 1;
            continue;
        }

        if is_table_start(&lines, i) {
            let head = split_row(lines[i]);
            i += 2;
            let mut rows = Vec::new();
            while i < lines.len() && !lines[i].trim().is_empty() && lines[i].contains('|') {
                let mut row = split_row(lines[i]);
                while row.len() < head.len() {
                    row.push(String::new());
                }
                row.truncate(head.len());
                rows.push(row.into_iter().map(|c| parse_inline(&c)).collect());
                i += 1;
            }
            blocks.push(Block::Table {
                head: head.into_iter().map(|c| parse_inline(&c)).collect(),
                rows,
            });
            continue;
        }

        if is_task(line) {
            let mut items = Vec::new();
            while i < lines.len() && is_task(lines[i]) {
                let done = task_done(lines[i]);
                let body = strip_task(lines[i]);
                items.push(ListItem {
                    content: parse_inline(body),
                    task: Some(done),
                    line: Some(i),
                });
                i += 1;
            }
            blocks.push(Block::List {
                ordered: false,
                items,
            });
            continue;
        }

        if is_ul(line) {
            let mut items = Vec::new();
            while i < lines.len() && is_ul(lines[i]) && !is_task(lines[i]) {
                items.push(ListItem {
                    content: parse_inline(strip_ul(lines[i])),
                    task: None,
                    line: None,
                });
                i += 1;
            }
            blocks.push(Block::List {
                ordered: false,
                items,
            });
            continue;
        }

        if is_ol(line) {
            let mut items = Vec::new();
            while i < lines.len() && is_ol(lines[i]) {
                items.push(ListItem {
                    content: parse_inline(strip_ol(lines[i])),
                    task: None,
                    line: None,
                });
                i += 1;
            }
            blocks.push(Block::List {
                ordered: true,
                items,
            });
            continue;
        }

        if is_quote(line) {
            let mut buf = Vec::new();
            while i < lines.len() && is_quote(lines[i]) {
                buf.push(strip_quote(lines[i]));
                i += 1;
            }
            blocks.push(Block::Quote(parse_inline(&buf.join(" "))));
            continue;
        }

        let mut para = Vec::new();
        while i < lines.len()
            && !lines[i].trim().is_empty()
            && !is_head(lines[i])
            && !is_ul(lines[i])
            && !is_task(lines[i])
            && !is_ol(lines[i])
            && !is_quote(lines[i])
            && !is_rule(lines[i])
            && !is_table_start(&lines, i)
        {
            para.push(lines[i]);
            i += 1;
        }
        if !para.is_empty() {
            blocks.push(Block::Paragraph(parse_inline(&para.join(" "))));
        } else {
            // A line that only looked like a block but was not consumed: print it as text.
            blocks.push(Block::Paragraph(parse_inline(line)));
            i += 1;
        }
    }

    blocks
}

fn is_rule(line: &str) -> bool {
    let t = line.trim();
    if t.len() < 3 {
        return false;
    }
    let mut chars = t.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !matches!(first, '-' | '*' | '_') {
        return false;
    }
    t.chars().all(|c| c == first || c == ' ')
}

fn heading(line: &str) -> Option<(u8, &str)> {
    let trimmed = line.trim_start_matches(' ');
    if trimmed.len() - trimmed.trim_start_matches('#').len() > 6 {
        return None;
    }
    let hashes = trimmed.len() - trimmed.trim_start_matches('#').len();
    if hashes == 0 {
        return None;
    }
    let rest = &trimmed[hashes..];
    if !rest.starts_with(' ') {
        return None;
    }
    Some((hashes as u8, rest.trim_start()))
}

fn is_head(line: &str) -> bool {
    heading(line).is_some()
}

fn is_task(line: &str) -> bool {
    let t = line.trim_start();
    let Some(rest) = t
        .strip_prefix("- ")
        .or_else(|| t.strip_prefix("* "))
        .or_else(|| t.strip_prefix("+ "))
    else {
        return false;
    };
    let rest = rest.trim_start();
    (rest.starts_with("[ ]") || rest.starts_with("[x]") || rest.starts_with("[X]"))
        && rest
            .get(3..)
            .is_none_or(|r| r.is_empty() || r.starts_with(char::is_whitespace))
}

fn task_done(line: &str) -> bool {
    let t = line.trim_start();
    let rest = t
        .strip_prefix("- ")
        .or_else(|| t.strip_prefix("* "))
        .or_else(|| t.strip_prefix("+ "))
        .unwrap_or(t)
        .trim_start();
    rest.starts_with("[x]") || rest.starts_with("[X]")
}

fn strip_task(line: &str) -> &str {
    let t = line.trim_start();
    let rest = t
        .strip_prefix("- ")
        .or_else(|| t.strip_prefix("* "))
        .or_else(|| t.strip_prefix("+ "))
        .unwrap_or(t)
        .trim_start();
    rest.get(3..).unwrap_or("").trim_start()
}

fn is_ul(line: &str) -> bool {
    let t = line.trim_start();
    (t.starts_with("- ") || t.starts_with("* ") || t.starts_with("+ ")) && !is_task(line)
}

fn strip_ul(line: &str) -> &str {
    line.trim_start()
        .strip_prefix("- ")
        .or_else(|| line.trim_start().strip_prefix("* "))
        .or_else(|| line.trim_start().strip_prefix("+ "))
        .unwrap_or(line)
        .trim_start()
}

fn is_ol(line: &str) -> bool {
    let t = line.trim_start();
    let digits = t.len() - t.trim_start_matches(|c: char| c.is_ascii_digit()).len();
    if digits == 0 || digits > 3 {
        return false;
    }
    let rest = &t[digits..];
    (rest.starts_with(". ") || rest.starts_with(") ")) && !rest[2..].trim().is_empty()
}

fn strip_ol(line: &str) -> &str {
    let t = line.trim_start();
    let digits = t.len() - t.trim_start_matches(|c: char| c.is_ascii_digit()).len();
    t[digits..]
        .strip_prefix(". ")
        .or_else(|| t[digits..].strip_prefix(") "))
        .unwrap_or(t)
        .trim_start()
}

fn is_quote(line: &str) -> bool {
    line.trim_start().starts_with('>')
}

fn strip_quote(line: &str) -> &str {
    line.trim_start()
        .strip_prefix('>')
        .unwrap_or(line)
        .strip_prefix(' ')
        .unwrap_or_else(|| line.trim_start().strip_prefix('>').unwrap_or(line))
}

fn is_table_start(lines: &[&str], i: usize) -> bool {
    i + 1 < lines.len() && lines[i].contains('|') && is_table_rule(lines[i + 1])
}

fn is_table_rule(line: &str) -> bool {
    let t = line.trim();
    let t = t.strip_prefix('|').unwrap_or(t);
    let t = t.strip_suffix('|').unwrap_or(t);
    let cells: Vec<&str> = t.split('|').collect();
    if cells.is_empty() {
        return false;
    }
    cells.iter().all(|cell| {
        let c = cell.trim();
        let c = c.strip_prefix(':').unwrap_or(c);
        let c = c.strip_suffix(':').unwrap_or(c);
        c.len() >= 2 && c.chars().all(|ch| ch == '-')
    })
}

fn split_row(line: &str) -> Vec<String> {
    let t = line.trim();
    let t = t.strip_prefix('|').unwrap_or(t);
    let t = t.strip_suffix('|').unwrap_or(t);
    t.split('|').map(|c| c.trim().to_string()).collect()
}

fn safe_href(url: &str) -> Option<String> {
    let url = url.trim();
    let lower = url.to_lowercase();
    if lower.starts_with("http://") || lower.starts_with("https://") || lower.starts_with("mailto:")
    {
        Some(url.to_string())
    } else {
        None
    }
}

fn find_from(s: &str, needle: &str, start: usize) -> Option<usize> {
    s.get(start..)?.find(needle).map(|p| p + start)
}

fn day_num(day: &str) -> Option<u32> {
    NaiveDate::parse_from_str(day, "%Y-%m-%d")
        .ok()
        .map(|d| chrono::Datelike::day(&d))
}

pub fn parse_inline(text: &str) -> Vec<Inline> {
    let mut out = Vec::new();
    let mut buf = String::new();
    let mut i = 0;
    let bytes = text.as_bytes();

    fn flush(out: &mut Vec<Inline>, buf: &mut String) {
        if !buf.is_empty() {
            out.push(Inline::Text(std::mem::take(buf)));
        }
    }

    while i < text.len() {
        if !text.is_char_boundary(i) {
            i += 1;
            continue;
        }
        let rest = &text[i..];
        let c = rest.chars().next().unwrap_or('\0');

        if c == '\\' {
            if let Some(next) = rest.chars().nth(1) {
                if "*_`[]#>-|!".contains(next) {
                    buf.push(next);
                    i += 1 + next.len_utf8();
                    continue;
                }
            }
        }

        if c == '`' {
            if let Some(end) = find_from(text, "`", i + 1) {
                if end > i + 1 {
                    flush(&mut out, &mut buf);
                    out.push(Inline::Code(text[i + 1..end].to_string()));
                    i = end + 1;
                    continue;
                }
            }
        }

        if c == '*' || c == '_' {
            let doubled = rest.starts_with(if c == '*' { "**" } else { "__" });
            if doubled {
                let marker = if c == '*' { "**" } else { "__" };
                if let Some(end) = find_from(text, marker, i + 2) {
                    if end > i + 2 {
                        flush(&mut out, &mut buf);
                        out.push(Inline::Bold(parse_inline(&text[i + 2..end])));
                        i = end + 2;
                        continue;
                    }
                }
            } else if rest
                .chars()
                .nth(1)
                .is_some_and(|n| !n.is_whitespace() && n != c)
            {
                let marker = if c == '*' { "*" } else { "_" };
                if let Some(end) = find_from(text, marker, i + 1) {
                    if end > i + 1 {
                        flush(&mut out, &mut buf);
                        out.push(Inline::Italic(parse_inline(&text[i + 1..end])));
                        i = end + 1;
                        continue;
                    }
                }
            }
        }

        if c == '[' {
            if let Some(close) = find_from(text, "]", i + 1) {
                if text[close + 1..].starts_with('(') {
                    if let Some(close2) = find_from(text, ")", close + 2) {
                        let label = &text[i + 1..close];
                        let url = &text[close + 2..close2];
                        flush(&mut out, &mut buf);
                        out.push(Inline::Link {
                            label: parse_inline(label),
                            href: safe_href(url),
                        });
                        i = close2 + 1;
                        continue;
                    }
                }
            }
        }

        if c == '@' {
            let before_ok = i == 0
                || text[..i]
                    .chars()
                    .next_back()
                    .is_some_and(|p| p.is_whitespace() || p == '(');
            if before_ok {
                let after = &text[i + 1..];
                let len = after
                    .chars()
                    .take_while(|ch| ch.is_ascii_alphanumeric() || *ch == '_' || *ch == '-')
                    .count();
                if len >= 2
                    && after
                        .chars()
                        .next()
                        .is_some_and(|ch| ch.is_ascii_alphabetic())
                {
                    let name: String = after.chars().take(len).collect();
                    flush(&mut out, &mut buf);
                    let name_len = name.len();
                    out.push(Inline::Person(name));
                    i += 1 + name_len;
                    continue;
                }
            }
        }

        if c == '^' && rest.starts_with("^[") {
            if let Some(close) = find_from(text, "]", i + 2) {
                let inside = &text[i + 2..close];
                let mut parts = inside.splitn(2, '^');
                let day = parts.next().unwrap_or("");
                let label = parts.next().filter(|l| !l.is_empty());
                if let Some(num) = day_num(day) {
                    flush(&mut out, &mut buf);
                    out.push(Inline::DayRef {
                        day: day.to_string(),
                        label: label.map(str::to_string),
                        num,
                    });
                    i = close + 1;
                    continue;
                }
            }
        }

        // Ordinary character. Advance by its UTF-8 width.
        buf.push(c);
        i += c.len_utf8();
        let _ = bytes;
    }

    flush(&mut out, &mut buf);
    out
}

/// Rewrite one `- [ ]` line in the source. Returns `None` when the line is not a task.
pub fn toggle_task_line(src: &str, line_no: usize) -> Option<String> {
    let normalized = src.replace("\r\n", "\n").replace('\r', "\n");
    let mut lines: Vec<String> = normalized.split('\n').map(str::to_string).collect();
    let line = lines.get(line_no)?.clone();
    let t = line.trim_start();
    let (prefix, rest) = if let Some(r) = t.strip_prefix("- ") {
        ("- ", r)
    } else if let Some(r) = t.strip_prefix("* ") {
        ("* ", r)
    } else {
        let r = t.strip_prefix("+ ")?;
        ("+ ", r)
    };
    let leading = &line[..line.len() - t.len()];
    let rest_trim = rest.trim_start();
    let space = &rest[..rest.len() - rest_trim.len()];
    let (marker, body) = if let Some(b) = rest_trim.strip_prefix("[ ]") {
        ("[x]", b)
    } else if let Some(b) = rest_trim.strip_prefix("[x]") {
        ("[ ]", b)
    } else {
        let b = rest_trim.strip_prefix("[X]")?;
        ("[ ]", b)
    };
    lines[line_no] = format!("{}{}{}{}{}", leading, prefix, space, marker, body);
    Some(lines.join("\n"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn headings_paragraphs_and_rules() {
        let blocks = parse("# Title\n\nSome **bold** text.\n\n---\n");
        assert_eq!(blocks.len(), 3);
        assert!(matches!(&blocks[0], Block::Heading { level: 1, .. }));
        assert!(matches!(&blocks[1], Block::Paragraph(_)));
        assert_eq!(blocks[2], Block::Rule);
    }

    #[test]
    fn lists_and_tasks_keep_their_line_numbers() {
        let blocks = parse("- one\n- two\n\n- [ ] book\n- [x] photos\n\n1. first\n2. second");
        assert_eq!(blocks.len(), 3);
        match &blocks[0] {
            Block::List { ordered, items } => {
                assert!(!ordered);
                assert_eq!(items.len(), 2);
                assert!(items[0].task.is_none());
            }
            _ => panic!("expected list"),
        }
        match &blocks[1] {
            Block::List { items, .. } => {
                assert_eq!(items[0].task, Some(false));
                assert_eq!(items[0].line, Some(3));
                assert_eq!(items[1].task, Some(true));
                assert_eq!(items[1].line, Some(4));
            }
            _ => panic!("expected task list"),
        }
        match &blocks[2] {
            Block::List { ordered, items } => {
                assert!(ordered);
                assert_eq!(items.len(), 2);
            }
            _ => panic!("expected ordered list"),
        }
    }

    #[test]
    fn quotes_tables_and_inline_marks() {
        let blocks = parse("> a quote\n\n| what | when |\n| --- | --- |\n| flights | 14 Nov |");
        assert_eq!(blocks.len(), 2);
        assert!(matches!(&blocks[0], Block::Quote(_)));
        match &blocks[1] {
            Block::Table { head, rows } => {
                assert_eq!(head.len(), 2);
                assert_eq!(rows.len(), 1);
            }
            _ => panic!("expected table"),
        }
    }

    #[test]
    fn inline_bold_italic_code_link_person_and_dayref() {
        let inline = parse_inline(
            "**bold** *italic* `code` [label](https://example.com) @Sipho ^[2026-11-15^drive]",
        );
        assert!(matches!(inline[0], Inline::Bold(_)));
        assert!(matches!(inline[1], Inline::Text(_)));
        assert!(matches!(inline[2], Inline::Italic(_)));
        assert!(matches!(inline[4], Inline::Code(_)));
        assert!(matches!(&inline[6], Inline::Link { href: Some(_), .. }));
        assert!(inline
            .iter()
            .any(|i| matches!(i, Inline::Person(p) if p == "Sipho")));
        assert!(inline
            .iter()
            .any(|i| matches!(i, Inline::DayRef { num: 15, label: Some(l), .. } if l == "drive")));
    }

    #[test]
    fn unsafe_links_keep_the_label_but_drop_the_href() {
        let inline = parse_inline("[click](javascript:alert(1))");
        assert!(matches!(&inline[0], Inline::Link { href: None, .. }));
        let inline = parse_inline("[mail](mailto:someone@example.com)");
        assert!(matches!(&inline[0], Inline::Link { href: Some(_), .. }));
    }

    #[test]
    fn raw_html_is_text_never_markup() {
        let blocks = parse("<script>alert('x')</script>");
        match &blocks[0] {
            Block::Paragraph(inline) => {
                assert!(inline.iter().all(|i| matches!(i, Inline::Text(_))));
                if let Inline::Text(t) = &inline[0] {
                    assert_eq!(t, "<script>alert('x')</script>");
                }
            }
            _ => panic!("expected paragraph"),
        }
    }

    #[test]
    fn task_toggle_rewrites_only_the_task_marker() {
        let src = "- [ ] book\n- [x] photos\nplain";
        assert_eq!(
            toggle_task_line(src, 0).unwrap(),
            "- [x] book\n- [x] photos\nplain"
        );
        assert_eq!(
            toggle_task_line(src, 1).unwrap(),
            "- [ ] book\n- [ ] photos\nplain"
        );
        assert_eq!(toggle_task_line(src, 2), None);
        assert_eq!(toggle_task_line(src, 9), None);
        // Leading indentation and the rest of the line survive.
        assert_eq!(
            toggle_task_line("  - [ ] keep this", 0).unwrap(),
            "  - [x] keep this"
        );
    }

    #[test]
    fn escaped_markers_are_text() {
        let inline = parse_inline(r"\*not italic\*");
        assert!(matches!(&inline[0], Inline::Text(t) if t == "*not italic*"));
    }

    #[test]
    fn empty_markers_do_not_loop() {
        let inline = parse_inline("**** ** `` [](x)");
        assert!(!inline.is_empty());
    }
}
