use anyhow::Result;
use clap::Parser;

mod cli;
mod pdf;
mod scan;
mod tui;

fn main() {
    if let Err(err) = run() {
        eprintln!("error: {:#}", err);
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let cli = cli::Cli::parse();

    // TUI mode is default unless --no-tui or --manifest-only
    if !cli.no_tui && !cli.manifest_only {
        return tui::run(cli);
    }

    let date = cli
        .date
        .unwrap_or_else(|| chrono::Local::now().format("%Y-%m-%d").to_string());
    let client = cli.client.unwrap_or_else(|| String::from("Delivery"));

    eprintln!("Scanning {}...", cli.input.display());
    let found = scan::discover(&cli.input)?;

    let image_count = found
        .iter()
        .filter(|(_, k)| *k == scan::AssetKind::Image)
        .count();
    let video_count = found
        .iter()
        .filter(|(_, k)| *k == scan::AssetKind::Video)
        .count();
    eprintln!(
        "Found {} assets ({} images, {} videos)",
        found.len(),
        image_count,
        video_count
    );

    let gen_thumbnails = !cli.manifest_only;
    let thumb_dir = tempfile::tempdir()?;
    let (assets, errors) = scan::process_all(&found, thumb_dir.path(), gen_thumbnails, cli.auto_orient);

    if !errors.is_empty() {
        eprintln!("\n{} files skipped:", errors.len());
        for err in &errors {
            eprintln!("  - {}", err);
        }
    }

    if assets.is_empty() {
        anyhow::bail!("No assets could be processed");
    }

    if cli.manifest_only {
        println!("Filename\tType\tResolution\tFormat\tSize\tColor Space");
        for a in &assets {
            println!(
                "{}\t{}\t{}\t{}\t{}\t{}",
                a.filename,
                a.kind,
                a.resolution(),
                a.format,
                a.human_size(),
                a.color_space.as_deref().unwrap_or("—")
            );
        }
        return Ok(());
    }

    let output = cli.output.unwrap_or_else(|| {
        let slug = client.to_lowercase().replace(' ', "-");
        std::path::PathBuf::from(format!("{}-delivery-{}.pdf", slug, date))
    });

    let config = pdf::PdfConfig {
        client: client.clone(),
        title: cli.title,
        date,
        columns: cli.columns,
        auto_orient: cli.auto_orient,
    };

    eprintln!("Generating PDF...");
    pdf::render(&assets, &config, &output)?;
    eprintln!("Done: {} ({} assets)", output.display(), assets.len());

    Ok(())
}
