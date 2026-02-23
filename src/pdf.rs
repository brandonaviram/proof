use std::path::Path;

use anyhow::{Context, Result};
use serde::Serialize;

use crate::scan::{Asset, AssetKind};

pub struct PdfConfig {
    pub client: String,
    pub title: Option<String>,
    pub date: String,
    pub columns: u8,
}

pub fn render(assets: &[Asset], config: &PdfConfig, output: &Path) -> Result<()> {
    check_typst()?;

    let data = build_data(assets, config);
    let json = serde_json::to_string_pretty(&data)?;

    let build_dir = tempfile::tempdir()?;
    std::fs::write(
        build_dir.path().join("template.typ"),
        include_str!("../templates/delivery-proof.typ"),
    )?;
    std::fs::write(build_dir.path().join("data.json"), &json)?;

    let thumbs_dir = build_dir.path().join("thumbs");
    std::fs::create_dir_all(&thumbs_dir)?;
    for asset in assets {
        if let Some(ref thumb) = asset.thumbnail_path {
            if let Some(name) = thumb.file_name() {
                std::fs::copy(thumb, thumbs_dir.join(name))?;
            }
        }
    }

    // Bundle Apercu Pro fonts into build dir
    let fonts_dir = build_dir.path().join("fonts");
    std::fs::create_dir_all(&fonts_dir)?;
    std::fs::write(fonts_dir.join("Apercu Pro Regular.ttf"), include_bytes!("../fonts/Apercu Pro Regular.ttf"))?;
    std::fs::write(fonts_dir.join("Apercu Pro Light.ttf"), include_bytes!("../fonts/Apercu Pro Light.ttf"))?;
    std::fs::write(fonts_dir.join("Apercu Pro Medium.ttf"), include_bytes!("../fonts/Apercu Pro Medium.ttf"))?;
    std::fs::write(fonts_dir.join("Apercu Pro Bold.ttf"), include_bytes!("../fonts/Apercu Pro Bold.ttf"))?;

    let abs_output = if output.is_absolute() {
        output.to_path_buf()
    } else {
        std::env::current_dir()?.join(output)
    };

    let status = std::process::Command::new("typst")
        .arg("compile")
        .arg("--font-path").arg("fonts")
        .arg("template.typ")
        .arg(&abs_output)
        .current_dir(build_dir.path())
        .status()
        .context("failed to run typst")?;

    anyhow::ensure!(
        status.success(),
        "typst compile failed (exit code: {:?})",
        status.code()
    );

    Ok(())
}

fn check_typst() -> Result<()> {
    let status = std::process::Command::new("typst")
        .arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status();

    match status {
        Ok(s) if s.success() => Ok(()),
        _ => anyhow::bail!("typst not found — install with: brew install typst"),
    }
}

#[derive(Serialize)]
struct TemplateData {
    client: String,
    title: Option<String>,
    date: String,
    columns: u8,
    summary: Summary,
    assets: Vec<AssetEntry>,
}

#[derive(Serialize)]
struct Summary {
    total_files: usize,
    total_size: String,
    image_count: usize,
    video_count: usize,
}

#[derive(Serialize)]
struct AssetEntry {
    filename: String,
    kind: String,
    resolution: String,
    format: String,
    human_size: String,
    thumbnail: Option<String>,
    color_space: Option<String>,
    duration: Option<String>,
}

fn build_data(assets: &[Asset], config: &PdfConfig) -> TemplateData {
    let total_size: u64 = assets.iter().map(|a| a.file_size).sum();
    let image_count = assets.iter().filter(|a| a.kind == AssetKind::Image).count();
    let video_count = assets.iter().filter(|a| a.kind == AssetKind::Video).count();

    let entries = assets
        .iter()
        .map(|a| {
            let thumbnail = a
                .thumbnail_path
                .as_ref()
                .and_then(|p| p.file_name())
                .map(|name| format!("thumbs/{}", name.to_string_lossy()));

            let duration = a.duration.map(|d| {
                let mins = (d / 60.0).floor() as u64;
                let secs = (d % 60.0).floor() as u64;
                format!("{}:{:02}", mins, secs)
            });

            AssetEntry {
                filename: a.filename.clone(),
                kind: a.kind.to_string(),
                resolution: a.resolution(),
                format: a.format.clone(),
                human_size: a.human_size(),
                thumbnail,
                color_space: a.color_space.clone(),
                duration,
            }
        })
        .collect();

    TemplateData {
        client: config.client.clone(),
        title: config.title.clone(),
        date: config.date.clone(),
        columns: config.columns,
        summary: Summary {
            total_files: assets.len(),
            total_size: humansize::format_size(total_size, humansize::BINARY),
            image_count,
            video_count,
        },
        assets: entries,
    }
}
