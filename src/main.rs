mod app;
mod categories;
mod config;
mod feed;
mod paper_faves;
mod read_state;
mod topic_filter;
mod ui;

use app::App;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::{io, time::Duration};
use topic_filter::TopicFilter;

fn print_help()
{
    println!(
        "{name} {version}
arXiv paper browser

USAGE:
    {name} [OPTIONS]

OPTIONS:
    -h, --help                  Print this help and exit
    -V, --version               Print version and exit
    -F, --filter <file>         JSON topic-filter file; scores and filters all
                                fetched papers, hiding those with score <= 0
                                and sorting the rest by relevance (highest first)
        --new-filter <file>     Write a demo filter JSON to <file> and exit
",
        name = env!("CARGO_PKG_NAME"),
        version = env!("CARGO_PKG_VERSION"),
    );
}

const DEMO_FILTER: &str = r#"{
  "_meta": {
    "name": "my-arxiv-filter",
    "version": "1.0",
    "description": "Customize this filter to highlight papers relevant to your research.",
    "scoring": "Each paper accumulates a score from matching category weights, keyword rules, and author rules. Negative weights from anti_keywords reduce the score. Papers with score <= 0 are hidden; the rest are sorted highest-first.",
    "last_updated": "2026-01-01"
  },

  "categories": {
    "primary": [
      {
        "code": "cs.LG",
        "weight": 3,
        "rationale": "Machine Learning — primary research area."
      },
      {
        "code": "stat.ML",
        "weight": 2,
        "rationale": "Statistics / ML crossover."
      }
    ],
    "secondary": [
      {
        "code": "cs.AI",
        "weight": 1,
        "rationale": "Broader AI; lower hit rate for this filter."
      },
      {
        "code": "cs.CV",
        "weight": 1,
        "rationale": "Computer Vision when it touches core ML."
      }
    ],
    "tertiary": [
      {
        "code": "cs.NE",
        "weight": 0,
        "rationale": "Neural & Evolutionary Computing — score via keywords only."
      }
    ]
  },

  "keyword_rules": {
    "core_topics": [
      {
        "match": "any_of",
        "terms": [
          "diffusion model",
          "score matching",
          "denoising diffusion",
          "flow matching"
        ],
        "weight": 8,
        "rationale": "Generative modelling via diffusion / flow — read immediately."
      },
      {
        "match": "any_of",
        "terms": [
          "transformer",
          "attention mechanism",
          "self-attention",
          "vision transformer"
        ],
        "weight": 5,
        "rationale": "Transformer architecture papers — read abstract."
      }
    ],
    "methods": [
      {
        "match": "any_of",
        "terms": [
          "reinforcement learning",
          "reward model",
          "RLHF",
          "policy gradient"
        ],
        "weight": 4,
        "rationale": "RL and alignment methods."
      }
    ]
  },

  "author_rules": {
    "tier_1": [
      {
        "name": "Yann LeCun",
        "weight": 8,
        "rationale": "Read everything."
      },
      {
        "name": "Yoshua Bengio",
        "weight": 8,
        "rationale": "Read everything."
      }
    ],
    "tier_2": [
      {
        "name": "Ilya Sutskever",
        "weight": 5,
        "rationale": "Follow closely."
      }
    ]
  },

  "exclusions": {
    "anti_keywords": [
      {
        "match": "any_of",
        "terms": [
          "drug discovery",
          "protein folding",
          "genomics",
          "medical imaging"
        ],
        "weight": -8,
        "rationale": "Bio/medical ML — not relevant to this filter."
      },
      {
        "match": "any_of",
        "terms": [
          "quantum computing",
          "quantum circuit",
          "quantum algorithm"
        ],
        "weight": -5,
        "rationale": "Quantum computing papers — out of scope."
      }
    ]
  },

  "scoring_thresholds": {
    "read_closely":  { "min_score": 10, "action": "Read full paper." },
    "read_abstract": { "min_score": 5,  "action": "Read abstract, decide." },
    "glance_title":  { "min_score": 1,  "action": "Glance at title." },
    "skip":          { "max_score": 0,  "action": "Hidden by rxy." }
  }
}
"#;

fn main() -> Result<(), Box<dyn std::error::Error>>
{
    let mut args = std::env::args().peekable();
    // skip argv[0]
    args.next();

    let mut filter_path: Option<String> = None;
    let mut new_filter_path: Option<String> = None;
    let mut show_version = false;
    let mut show_help = false;
    let mut unknown: Vec<String> = Vec::new();

    while let Some(arg) = args.next()
    {
        match arg.as_str()
        {
            "--version" | "-V" => show_version = true,
            "--help" | "-h" => show_help = true,
            "--filter" | "-F" =>
            {
                match args.next()
                {
                    Some(path) => filter_path = Some(path),
                    None =>
                    {
                        eprintln!("error: --filter requires a file argument");
                        eprintln!("Try '{} --help' for usage.", env!("CARGO_PKG_NAME"));
                        std::process::exit(1);
                    }
                }
            }
            "--new-filter" =>
            {
                match args.next()
                {
                    Some(path) => new_filter_path = Some(path),
                    None =>
                    {
                        eprintln!("error: --new-filter requires a file argument");
                        eprintln!("Try '{} --help' for usage.", env!("CARGO_PKG_NAME"));
                        std::process::exit(1);
                    }
                }
            }
            other => unknown.push(other.to_string()),
        }
    }

    if show_help
    {
        print_help();
        return Ok(());
    }

    if show_version
    {
        println!("{} {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
        return Ok(());
    }

    if let Some(ref path) = new_filter_path
    {
        if std::path::Path::new(path).exists()
        {
            eprintln!("error: '{}' already exists — refusing to overwrite.", path);
            std::process::exit(1);
        }
        match std::fs::write(path, DEMO_FILTER)
        {
            Ok(()) => println!("Demo filter written to '{}'.\nEdit it, then run: {} --filter {}", path, env!("CARGO_PKG_NAME"), path),
            Err(e) => { eprintln!("error: could not write '{}': {}", path, e); std::process::exit(1); }
        }
        return Ok(());
    }

    if !unknown.is_empty()
    {
        for flag in &unknown
        {
            eprintln!("error: unknown option '{}'", flag);
        }
        eprintln!("Try '{} --help' for usage.", env!("CARGO_PKG_NAME"));
        std::process::exit(1);
    }

    let topic_filter = match filter_path
    {
        Some(ref path) => match TopicFilter::load(path)
        {
            Ok(f) => Some(f),
            Err(e) =>
            {
                eprintln!("Filter error: {}", e);
                return Ok(());
            }
        },
        None => None,
    };

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new(topic_filter);
    // Kick off the initial load in the background — UI is immediately responsive.
    app.load_feed();

    let result = run_app(&mut terminal, &mut app);

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(e) = result
    {
        eprintln!("Error: {}", e);
    }
    Ok(())
}

fn run_app<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App,
) -> io::Result<()>
{
    loop
    {
        app.tick();

        terminal.draw(|f| ui::render(f, app))?;

        if event::poll(Duration::from_millis(100))?
        {
            if let Event::Key(key) = event::read()?
            {
                if key.kind == KeyEventKind::Press
                {
                    if app.on_key(key.code)
                    {
                        break;
                    }
                }
            }
        }
    }
    Ok(())
}
