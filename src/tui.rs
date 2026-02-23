use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Gauge, List, ListItem, Paragraph};
use ratatui::Frame;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use crate::cli::Cli;
use crate::pdf;
use crate::scan;

// ── Messages from background thread ────────────────────────

enum Msg {
    AssetFound { filename: String, kind: String },
    ScanDone { total: usize },
    Processing { index: usize },
    Processed { index: usize },
    Failed { index: usize, error: String },
    Rendering,
    Done { output: String, total: usize },
    Error(String),
}

// ── TUI state ──────────────────────────────────────────────

#[derive(Clone, PartialEq)]
enum Phase {
    Scanning,
    Processing,
    Rendering,
    Complete,
    Failed,
}

#[derive(Clone)]
enum FileStatus {
    Pending,
    Processing,
    Done,
    Failed(String),
}

#[derive(Clone)]
struct FileEntry {
    filename: String,
    kind: String,
    status: FileStatus,
}

struct App {
    phase: Phase,
    files: Vec<FileEntry>,
    scroll: usize,
    tick: u64,
    total_found: usize,
    processed_count: usize,
    failed_count: usize,
    client: String,
    date: String,
    columns: u8,
    output_path: String,
    error_msg: Option<String>,
}

const SPINNER: &[char] = &['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];

impl App {
    fn new(client: &str, date: &str, columns: u8) -> Self {
        Self {
            phase: Phase::Scanning,
            files: Vec::new(),
            scroll: 0,
            tick: 0,
            total_found: 0,
            processed_count: 0,
            failed_count: 0,
            client: client.to_string(),
            date: date.to_string(),
            columns,
            output_path: String::new(),
            error_msg: None,
        }
    }

    fn spinner(&self) -> char {
        SPINNER[(self.tick as usize / 2) % SPINNER.len()]
    }
}

// ── Entry point ────────────────────────────────────────────

pub fn run(cli: Cli) -> Result<()> {
    let date = cli
        .date
        .unwrap_or_else(|| chrono::Local::now().format("%Y-%m-%d").to_string());
    let client = cli.client.unwrap_or_else(|| String::from("Delivery"));
    let columns = cli.columns;

    let output = cli.output.unwrap_or_else(|| {
        let slug = client.to_lowercase().replace(' ', "-");
        std::path::PathBuf::from(format!("{slug}-delivery-{date}.pdf"))
    });

    let auto_orient = cli.auto_orient;
    let config = pdf::PdfConfig {
        client: client.clone(),
        title: cli.title.clone(),
        date: date.clone(),
        columns,
        auto_orient,
    };

    let (tx, rx) = mpsc::channel::<Msg>();

    // Spawn background pipeline
    let input = cli.input.clone();
    let output_bg = output.clone();
    std::thread::spawn(move || {
        if let Err(e) = pipeline(tx.clone(), &input, &config, &output_bg) {
            let _ = tx.send(Msg::Error(format!("{e:#}")));
        }
    });

    // Run TUI
    let mut terminal = ratatui::init();
    let mut app = App::new(&client, &date, columns);
    let result = event_loop(&mut terminal, &mut app, &rx);
    ratatui::restore();
    result
}

fn event_loop(
    terminal: &mut ratatui::DefaultTerminal,
    app: &mut App,
    rx: &mpsc::Receiver<Msg>,
) -> Result<()> {
    let tick_rate = Duration::from_millis(80);
    let mut last_tick = Instant::now();

    loop {
        terminal.draw(|f| draw(f, app))?;

        let timeout = tick_rate.saturating_sub(last_tick.elapsed());
        if event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
                        KeyCode::Char('j') | KeyCode::Down => {
                            if app.scroll < app.files.len().saturating_sub(1) {
                                app.scroll += 1;
                            }
                        }
                        KeyCode::Char('k') | KeyCode::Up => {
                            app.scroll = app.scroll.saturating_sub(1);
                        }
                        KeyCode::Enter
                            if app.phase == Phase::Complete || app.phase == Phase::Failed =>
                        {
                            return Ok(());
                        }
                        _ => {}
                    }
                }
            }
        }

        // Drain messages from background thread
        while let Ok(msg) = rx.try_recv() {
            match msg {
                Msg::AssetFound { filename, kind } => {
                    app.files.push(FileEntry {
                        filename,
                        kind,
                        status: FileStatus::Pending,
                    });
                    app.total_found = app.files.len();
                }
                Msg::ScanDone { total } => {
                    app.total_found = total;
                    app.phase = Phase::Processing;
                }
                Msg::Processing { index } => {
                    if let Some(f) = app.files.get_mut(index) {
                        f.status = FileStatus::Processing;
                    }
                }
                Msg::Processed { index } => {
                    if let Some(f) = app.files.get_mut(index) {
                        f.status = FileStatus::Done;
                    }
                    app.processed_count += 1;
                }
                Msg::Failed { index, error } => {
                    if let Some(f) = app.files.get_mut(index) {
                        f.status = FileStatus::Failed(error);
                    }
                    app.failed_count += 1;
                    app.processed_count += 1;
                }
                Msg::Rendering => {
                    app.phase = Phase::Rendering;
                }
                Msg::Done { output, total } => {
                    app.phase = Phase::Complete;
                    app.output_path = output;
                    app.processed_count = total;
                }
                Msg::Error(e) => {
                    app.phase = Phase::Failed;
                    app.error_msg = Some(e);
                }
            }
        }

        if last_tick.elapsed() >= tick_rate {
            app.tick += 1;
            last_tick = Instant::now();
        }
    }
}

// ── Background pipeline ────────────────────────────────────

fn pipeline(
    tx: mpsc::Sender<Msg>,
    input: &std::path::Path,
    config: &pdf::PdfConfig,
    output: &std::path::Path,
) -> Result<()> {
    // 1. Scan
    let found = scan::discover(input)?;
    for (path, kind) in &found {
        let fname = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("?")
            .to_string();
        let kind_str = match kind {
            scan::AssetKind::Image => "image",
            scan::AssetKind::Video => "video",
        };
        let _ = tx.send(Msg::AssetFound {
            filename: fname,
            kind: kind_str.into(),
        });
    }
    let _ = tx.send(Msg::ScanDone {
        total: found.len(),
    });

    // 2. Process sequentially (for per-file TUI updates)
    let thumb_dir = tempfile::tempdir()?;
    let mut assets = Vec::with_capacity(found.len());

    for (i, (path, kind)) in found.iter().enumerate() {
        let _ = tx.send(Msg::Processing { index: i });

        match scan::process_one(path, *kind, thumb_dir.path(), i, true, config.auto_orient) {
            Ok(asset) => {
                let _ = tx.send(Msg::Processed { index: i });
                assets.push(asset);
            }
            Err(e) => {
                let _ = tx.send(Msg::Failed {
                    index: i,
                    error: format!("{e:#}"),
                });
            }
        }
    }

    // Sort to match natural order
    assets.sort_by(|a, b| natord::compare(&a.filename, &b.filename));

    // 3. Render PDF
    let _ = tx.send(Msg::Rendering);
    pdf::render(&assets, config, output)?;

    let out_str = output.display().to_string();
    let total = assets.len();
    let _ = tx.send(Msg::Done {
        output: out_str,
        total,
    });
    Ok(())
}

// ── Drawing ────────────────────────────────────────────────

fn draw(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // header
            Constraint::Min(6),   // file list
            Constraint::Length(3), // progress
            Constraint::Length(3), // footer
        ])
        .split(f.area());

    draw_header(f, app, chunks[0]);
    draw_files(f, app, chunks[1]);
    draw_progress(f, app, chunks[2]);
    draw_footer(f, app, chunks[3]);
}

fn draw_header(f: &mut Frame, app: &App, area: Rect) {
    let phase_str = match app.phase {
        Phase::Scanning => format!("{} Scanning...", app.spinner()),
        Phase::Processing => format!("{} Processing...", app.spinner()),
        Phase::Rendering => format!("{} Rendering PDF...", app.spinner()),
        Phase::Complete => "Done".into(),
        Phase::Failed => "Failed".into(),
    };

    let header = Paragraph::new(Line::from(vec![
        Span::styled(
            " proof ",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" "),
        Span::styled(
            &app.client,
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("  {}  cols:{}  ", app.date, app.columns),
            Style::default().fg(Color::DarkGray),
        ),
        Span::styled(
            phase_str,
            Style::default().fg(match app.phase {
                Phase::Complete => Color::Green,
                Phase::Failed => Color::Red,
                _ => Color::Yellow,
            }),
        ),
    ]))
    .block(
        Block::default()
            .borders(Borders::BOTTOM)
            .border_style(Style::default().fg(Color::DarkGray)),
    );

    f.render_widget(header, area);
}

fn draw_files(f: &mut Frame, app: &App, area: Rect) {
    let visible = (area.height as usize).saturating_sub(2);
    let start = app.scroll.min(app.files.len().saturating_sub(visible));

    let items: Vec<ListItem> = app
        .files
        .iter()
        .skip(start)
        .take(visible)
        .map(|entry| {
            let (icon, color) = match &entry.status {
                FileStatus::Done => ("\u{2713} ", Color::Green),
                FileStatus::Processing => ("\u{25CF} ", Color::Yellow),
                FileStatus::Failed(_) => ("\u{2717} ", Color::Red),
                FileStatus::Pending => ("  ", Color::DarkGray),
            };

            let mut spans = vec![
                Span::styled(icon, Style::default().fg(color)),
                Span::styled(entry.filename.as_str(), Style::default().fg(color)),
                Span::styled(
                    format!("  {}", entry.kind),
                    Style::default().fg(Color::DarkGray),
                ),
            ];

            if let FileStatus::Failed(ref err) = entry.status {
                spans.push(Span::styled(
                    format!("  {err}"),
                    Style::default().fg(Color::Red),
                ));
            }

            ListItem::new(Line::from(spans))
        })
        .collect();

    let title = format!(" Files ({}) ", app.files.len());
    let list = List::new(items).block(
        Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray)),
    );
    f.render_widget(list, area);
}

fn draw_progress(f: &mut Frame, app: &App, area: Rect) {
    let (ratio, label) = match app.phase {
        Phase::Scanning => (0.0, format!("Scanning... {} found", app.total_found)),
        Phase::Processing => {
            let r = if app.total_found > 0 {
                app.processed_count as f64 / app.total_found as f64
            } else {
                0.0
            };
            (
                r,
                format!("{}/{} processed", app.processed_count, app.total_found),
            )
        }
        Phase::Rendering => (1.0, "Rendering PDF...".into()),
        Phase::Complete => (1.0, format!("Complete: {}", app.output_path)),
        Phase::Failed => (0.0, "Failed".into()),
    };

    let gauge = Gauge::default()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray)),
        )
        .gauge_style(Style::default().fg(Color::Cyan))
        .ratio(ratio.clamp(0.0, 1.0))
        .label(label);
    f.render_widget(gauge, area);
}

fn draw_footer(f: &mut Frame, app: &App, area: Rect) {
    let text = match app.phase {
        Phase::Complete | Phase::Failed => " q/Enter: exit  j/k: scroll ",
        _ => " q: cancel  j/k: scroll ",
    };

    let mut spans = vec![Span::styled(text, Style::default().fg(Color::DarkGray))];

    if app.failed_count > 0 {
        spans.push(Span::styled(
            format!(" {} failed ", app.failed_count),
            Style::default().fg(Color::Red),
        ));
    }

    if let Some(ref err) = app.error_msg {
        spans.push(Span::styled(
            format!(" {err}"),
            Style::default().fg(Color::Red),
        ));
    }

    let footer = Paragraph::new(Line::from(spans)).block(
        Block::default()
            .borders(Borders::TOP)
            .border_style(Style::default().fg(Color::DarkGray)),
    );
    f.render_widget(footer, area);
}
