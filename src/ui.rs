use crate::app::{App, Panel, PapersTab};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap},
    Frame,
};

pub fn render(f: &mut Frame, app: &App)
{
    let area = f.area();

    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(40), Constraint::Min(0)])
        .split(area);

    let right_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(main_chunks[1]);

    draw_categories(f, app, main_chunks[0]);
    draw_papers(f, app, right_chunks[0]);
    draw_abstract(f, app, right_chunks[1]);
    draw_status(f, app, area);

    if app.fav_picker
    {
        draw_fav_picker(f, app, area);
    }
    else if app.show_help
    {
        draw_help(f, area);
    }
}

fn key_hint(pairs: &[(&str, &str)]) -> Paragraph<'static>
{
    let mut spans: Vec<Span> = Vec::new();
    for (i, (key, label)) in pairs.iter().enumerate()
    {
        if i > 0
        {
            spans.push(Span::raw("  "));
        }
        spans.push(Span::styled(
            key.to_string(),
            Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
        ));
        spans.push(Span::styled(
            format!(" {}", label),
            Style::default(),
        ));
    }
    Paragraph::new(Line::from(spans))
}

fn border_style(focused: bool) -> Style
{
    if focused
    {
        Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
    }
    else
    {
        Style::default()
    }
}

fn draw_categories(f: &mut Frame, app: &App, area: Rect)
{
    let focused = app.focus == Panel::Categories;
    let vis = app.visible_categories();

    let mode_label = if app.show_all_categories
    {
        " [all] (a=faves, +=add)"
    }
    else if app.config.favorites.is_empty()
    {
        " [all] (+=add fave)"
    }
    else
    {
        " [★ only] (a=all, +=add, -=del)"
    };
    let title = format!(" Categories{} ", mode_label);

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(border_style(focused));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(1)])
        .split(inner);

    if vis.is_empty()
    {
        let p = Paragraph::new("No favorites.\nPress 'a' to show all,\nor '+' to add one.");
        f.render_widget(p, chunks[0]);
    }
    else
    {
        let items: Vec<ListItem> = vis
            .iter()
            .map(|(id, desc)|
            {
                let star = if app.is_favorite(id) { "[★]" } else { "[ ]" };
                let style = if app.is_favorite(id)
                {
                    Style::default().fg(Color::Yellow)
                }
                else
                {
                    Style::default()
                };
                ListItem::new(Line::from(vec![
                    Span::styled(format!("{} {}", star, id), style),
                    Span::styled(format!(" {}", desc), Style::default()),
                ]))
            })
            .collect();

        let list = List::new(items)
            .highlight_style(
                Style::default()
                    .bg(Color::Blue)
                    .fg(Color::Rgb(255, 255, 255))
                    .add_modifier(Modifier::BOLD)
                    .remove_modifier(Modifier::DIM),
            )
            .highlight_symbol("▶ ");

        f.render_stateful_widget(list, chunks[0], &mut app.cat_state.clone());
    }

    let hint = key_hint(&[("f", "toggle"), ("+", "add"), ("-", "remove"), ("a", "all/faves")]);
    f.render_widget(hint, chunks[1]);
}

fn draw_papers(f: &mut Frame, app: &App, area: Rect)
{
    let focused = app.focus == Panel::Papers;

    match app.papers_tab
    {
        PapersTab::Feed => draw_papers_feed(f, app, area, focused),
        PapersTab::Saved => draw_papers_saved(f, app, area, focused),
    }
}

fn draw_papers_feed(f: &mut Frame, app: &App, area: Rect, focused: bool)
{
    let vis_indices = app.visible_paper_indices();
    let total = app.papers.len();
    let hidden = total - vis_indices.len();

    let title = if hidden > 0 && app.hide_read
    {
        format!(" Feed [{} read hidden — h=show] ", hidden)
    }
    else if !app.hide_read && app.read_state.count() > 0
    {
        " Feed [showing all — h=hide read] ".to_string()
    }
    else
    {
        " Feed ".to_string()
    };

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(border_style(focused));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(1)])
        .split(inner);

    if app.loading
    {
        f.render_widget(Paragraph::new("Loading…"), chunks[0]);
    }
    else if vis_indices.is_empty()
    {
        let msg = if total > 0 && app.hide_read
        {
            format!(
                "All {} papers read.\nPress 'h' to show them or 'r' for a fresh feed.",
                total
            )
        }
        else
        {
            "No papers. Press 'r' to load favorites,\nor select a category and press Enter."
                .to_string()
        };
        f.render_widget(Paragraph::new(msg), chunks[0]);
    }
    else
    {
        let items: Vec<ListItem> = vis_indices
            .iter()
            .map(|&idx|
            {
                let p = &app.papers[idx];
                let is_read = app.read_state.is_read(&p.link);
                let is_saved = app.paper_faves.is_saved(&p.link);
                let date = if p.pub_date.is_empty()
                {
                    String::new()
                }
                else
                {
                    format!(" ({})", p.pub_date)
                };
                let title: String = p.title.chars().take(56).collect();
                let title = if p.title.chars().count() > 56
                {
                    format!("{}…", title)
                }
                else
                {
                    title
                };
                let (title_style, date_style) = if is_read
                {
                    (
                        Style::default().add_modifier(Modifier::DIM),
                        Style::default().add_modifier(Modifier::DIM),
                    )
                }
                else
                {
                    (
                        Style::default().add_modifier(Modifier::BOLD),
                        Style::default(),
                    )
                };
                let star = if is_saved
                {
                    Span::styled("★ ", Style::default().fg(Color::Yellow))
                }
                else
                {
                    Span::raw("  ")
                };
                ListItem::new(Line::from(vec![
                    star,
                    Span::styled(title, title_style),
                    Span::styled(date, date_style),
                ]))
            })
            .collect();

        let list = List::new(items)
            .highlight_style(
                Style::default()
                    .bg(Color::Blue)
                    .fg(Color::Rgb(255, 255, 255))
                    .add_modifier(Modifier::BOLD)
                    .remove_modifier(Modifier::DIM),
            )
            .highlight_symbol("▶ ");

        f.render_stateful_widget(list, chunks[0], &mut app.paper_state.clone());
    }

    let hint = key_hint(&[
        ("s", "★ save"),
        ("S", "→saved"),
        ("x", "read/unread"),
        ("h", "hide/show"),
        ("o", "open"),
        ("p", "pdf"),
    ]);
    f.render_widget(hint, chunks[1]);
}

fn draw_papers_saved(f: &mut Frame, app: &App, area: Rect, focused: bool)
{
    let saved = app.paper_faves.papers();
    let title = format!(" ★ Saved [{}] ", saved.len());

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(border_style(focused));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(1)])
        .split(inner);

    if saved.is_empty()
    {
        f.render_widget(
            Paragraph::new("No saved papers yet.\nPress 's' on any paper to save it."),
            chunks[0],
        );
    }
    else
    {
        let items: Vec<ListItem> = saved
            .iter()
            .map(|p|
            {
                let is_read = app.read_state.is_read(&p.link);
                let date = if p.pub_date.is_empty()
                {
                    String::new()
                }
                else
                {
                    format!(" ({})", p.pub_date)
                };
                let title: String = p.title.chars().take(58).collect();
                let title = if p.title.chars().count() > 58
                {
                    format!("{}…", title)
                }
                else
                {
                    title
                };
                let (title_style, date_style) = if is_read
                {
                    (
                        Style::default().add_modifier(Modifier::DIM),
                        Style::default().add_modifier(Modifier::DIM),
                    )
                }
                else
                {
                    (
                        Style::default().add_modifier(Modifier::BOLD),
                        Style::default(),
                    )
                };
                ListItem::new(Line::from(vec![
                    Span::styled(title, title_style),
                    Span::styled(date, date_style),
                ]))
            })
            .collect();

        let list = List::new(items)
            .highlight_style(
                Style::default()
                    .bg(Color::Blue)
                    .fg(Color::Rgb(255, 255, 255))
                    .add_modifier(Modifier::BOLD)
                    .remove_modifier(Modifier::DIM),
            )
            .highlight_symbol("▶ ");

        f.render_stateful_widget(list, chunks[0], &mut app.saved_paper_state.clone());
    }

    let hint = key_hint(&[
        ("s", "unsave"),
        ("S", "→feed"),
        ("x", "read/unread"),
        ("o", "open"),
        ("p", "pdf"),
    ]);
    f.render_widget(hint, chunks[1]);
}

fn draw_abstract(f: &mut Frame, app: &App, area: Rect)
{
    let focused = app.focus == Panel::Abstract;
    let block = Block::default()
        .title(" Abstract ")
        .borders(Borders::ALL)
        .border_style(border_style(focused));

    let inner = block.inner(area);

    match app.selected_paper()
    {
        None =>
        {
            let p = Paragraph::new("Select a paper to view its abstract.").block(block);
            f.render_widget(p, area);
        }
        Some(paper) =>
        {
            f.render_widget(block, area);

            let mut lines: Vec<Line> = Vec::new();

            lines.push(Line::from(Span::styled(
                paper.title.clone(),
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
            )));
            lines.push(Line::from(""));

            if !paper.authors.is_empty()
            {
                lines.push(Line::from(vec![
                    Span::styled("Authors: ", Style::default().fg(Color::Yellow)),
                    Span::raw(paper.authors.join(", ")),
                ]));
                lines.push(Line::from(""));
            }

            if !paper.abstract_text.is_empty()
            {
                lines.push(Line::from(Span::styled(
                    "Abstract:",
                    Style::default().fg(Color::Yellow),
                )));
                lines.push(Line::from(paper.abstract_text.clone()));
                lines.push(Line::from(""));
            }

            lines.push(Line::from(vec![
                Span::styled(
                    "[o] ",
                    Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
                ),
                Span::styled("Abstract: ", Style::default()),
                Span::styled(paper.link.clone(), Style::default().fg(Color::Blue)),
            ]));

            if !paper.pdf_link.is_empty()
            {
                lines.push(Line::from(vec![
                    Span::styled(
                        "[p] ",
                        Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
                    ),
                    Span::styled("PDF:      ", Style::default()),
                    Span::styled(paper.pdf_link.clone(), Style::default().fg(Color::Blue)),
                ]));
            }

            let p = Paragraph::new(Text::from(lines))
                .wrap(Wrap { trim: false })
                .scroll((app.abstract_scroll, 0));
            f.render_widget(p, inner);
        }
    }
}

fn draw_status(f: &mut Frame, app: &App, area: Rect)
{
    let status_area = Rect
    {
        x: area.x,
        y: area.y + area.height.saturating_sub(1),
        width: area.width,
        height: 1,
    };
    let p = Paragraph::new(format!(" {} ", app.status_msg))
        .style(Style::default().bg(Color::DarkGray).fg(Color::White));
    f.render_widget(p, status_area);
}

fn draw_fav_picker(f: &mut Frame, app: &App, area: Rect)
{
    let items_data = app.fav_picker_items();

    let width = 44u16;
    let height = (items_data.len() as u16 + 4).min(area.height.saturating_sub(4)).max(5);
    let x = area.width.saturating_sub(width) / 2;
    let y = area.height.saturating_sub(height) / 2;
    let popup_area = Rect { x, y, width: width.min(area.width), height };

    f.render_widget(Clear, popup_area);

    if items_data.is_empty()
    {
        let p = Paragraph::new("All categories are already favorites.")
            .block(
                Block::default()
                    .title(" Add Favorite ")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Yellow)),
            )
            .style(Style::default().bg(Color::Black));
        f.render_widget(p, popup_area);
        return;
    }

    let items: Vec<ListItem> = items_data
        .iter()
        .map(|(id, desc)|
        {
            ListItem::new(Line::from(vec![
                Span::styled(id.to_string(), Style::default().fg(Color::White)),
                Span::styled(format!(" {}", desc), Style::default()),
            ]))
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .title(" Add Favorite  ↑/↓ navigate · Enter add · Esc close ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Yellow)),
        )
        .style(Style::default().bg(Color::Black))
        .highlight_style(
            Style::default()
                .bg(Color::Yellow)
                .fg(Color::Black)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▶ ");

    f.render_stateful_widget(list, popup_area, &mut app.fav_picker_state.clone());
}

fn draw_help(f: &mut Frame, area: Rect)
{
    let help_lines = vec![
        Line::from(Span::styled(
            " Keyboard Shortcuts ",
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(" Tab          Cycle panel focus"),
        Line::from(" ↑/↓  j/k     Navigate list"),
        Line::from(" Enter/Space  Load category / view abstract"),
        Line::from(" s            Save / unsave paper (persists across sessions)"),
        Line::from(" S            Switch Papers panel: Feed ↔ Saved"),
        Line::from(" x            Toggle read/unread on current paper"),
        Line::from(" h            Toggle hide/show read papers (Feed only)"),
        Line::from(" f            Toggle favorite (Categories panel)"),
        Line::from(" +            Add a new favorite (picker popup)"),
        Line::from(" -            Remove highlighted favorite"),
        Line::from(" a            Toggle all/favorites-only (Categories)"),
        Line::from(" r            Refresh / load favorites feed"),
        Line::from(" o            Open abstract page in browser"),
        Line::from(" p            Open PDF in browser"),
        Line::from(" q / Esc      Quit"),
        Line::from(" ?            Toggle this help"),
        Line::from(""),
        Line::from(Span::styled(
            " Press any key to close ",
            Style::default(),
        )),
    ];

    let width = 56u16;
    let height = (help_lines.len() as u16) + 2;
    let x = area.width.saturating_sub(width) / 2;
    let y = area.height.saturating_sub(height) / 2;

    let popup_area = Rect
    {
        x,
        y,
        width: width.min(area.width),
        height: height.min(area.height),
    };

    f.render_widget(Clear, popup_area);
    let p = Paragraph::new(Text::from(help_lines))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan)),
        )
        .style(Style::default().bg(Color::Black));
    f.render_widget(p, popup_area);
}
