use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use image::GenericImageView;
use rayon::prelude::*;
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub enum AssetKind {
    Image,
    Video,
}

impl std::fmt::Display for AssetKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AssetKind::Image => write!(f, "Image"),
            AssetKind::Video => write!(f, "Video"),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Asset {
    pub filename: String,
    pub kind: AssetKind,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub file_size: u64,
    pub format: String,
    pub color_space: Option<String>,
    pub duration: Option<f64>,
    pub codec: Option<String>,
    #[serde(skip)]
    pub thumbnail_path: Option<PathBuf>,
}

impl Asset {
    pub fn resolution(&self) -> String {
        match (self.width, self.height) {
            (Some(w), Some(h)) => format!("{}x{}", w, h),
            _ => String::from("—"),
        }
    }

    pub fn human_size(&self) -> String {
        humansize::format_size(self.file_size, humansize::BINARY)
    }
}

fn classify(ext: &str) -> Option<AssetKind> {
    match ext.to_ascii_lowercase().as_str() {
        "jpg" | "jpeg" | "png" | "tiff" | "tif" | "webp" => Some(AssetKind::Image),
        "mp4" | "mov" | "mxf" => Some(AssetKind::Video),
        _ => None,
    }
}

pub fn discover(dir: &Path) -> Result<Vec<(PathBuf, AssetKind)>> {
    anyhow::ensure!(dir.is_dir(), "'{}' is not a directory", dir.display());

    let mut assets: Vec<(PathBuf, AssetKind)> = walkdir::WalkDir::new(dir)
        .into_iter()
        .filter_entry(|e| {
            !e.file_name()
                .to_str()
                .map_or(false, |s| s.starts_with('.'))
        })
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter_map(|e| {
            let path = e.into_path();
            let ext = path.extension()?.to_str()?;
            let kind = classify(ext)?;
            Some((path, kind))
        })
        .collect();

    assets.sort_by(|a, b| {
        natord::compare(
            a.0.file_name().unwrap_or_default().to_str().unwrap_or(""),
            b.0.file_name().unwrap_or_default().to_str().unwrap_or(""),
        )
    });

    anyhow::ensure!(
        !assets.is_empty(),
        "No supported assets found in '{}'",
        dir.display()
    );

    Ok(assets)
}

pub fn process_all(
    assets: &[(PathBuf, AssetKind)],
    thumb_dir: &Path,
    gen_thumbnails: bool,
    auto_orient: bool,
) -> (Vec<Asset>, Vec<String>) {
    eprintln!("Processing {} assets...", assets.len());

    let results: Vec<Result<Asset>> = assets
        .par_iter()
        .enumerate()
        .map(|(i, (path, kind))| {
            process_one(path, *kind, thumb_dir, i, gen_thumbnails, auto_orient)
        })
        .collect();

    let mut processed = Vec::new();
    let mut errors = Vec::new();

    for result in results {
        match result {
            Ok(asset) => processed.push(asset),
            Err(e) => errors.push(format!("{:#}", e)),
        }
    }

    processed.sort_by(|a, b| natord::compare(&a.filename, &b.filename));

    (processed, errors)
}

pub fn process_one(
    path: &Path,
    kind: AssetKind,
    thumb_dir: &Path,
    index: usize,
    gen_thumbnails: bool,
    auto_orient: bool,
) -> Result<Asset> {
    let filename = path
        .file_name()
        .context("no filename")?
        .to_string_lossy()
        .to_string();

    let file_size = std::fs::metadata(path)
        .with_context(|| format!("cannot stat '{}'", path.display()))?
        .len();

    let format = path
        .extension()
        .unwrap_or_default()
        .to_str()
        .unwrap_or("")
        .to_uppercase();

    let mut asset = Asset {
        filename,
        kind,
        width: None,
        height: None,
        file_size,
        format,
        color_space: None,
        duration: None,
        codec: None,
        thumbnail_path: None,
    };

    match kind {
        AssetKind::Image => process_image(&mut asset, path, thumb_dir, index, gen_thumbnails, auto_orient)?,
        AssetKind::Video => process_video(&mut asset, path, thumb_dir, index, gen_thumbnails),
    }

    Ok(asset)
}

fn process_image(
    asset: &mut Asset,
    path: &Path,
    thumb_dir: &Path,
    index: usize,
    gen_thumbnails: bool,
    auto_orient: bool,
) -> Result<()> {
    if gen_thumbnails {
        let img = image::open(path)
            .with_context(|| format!("cannot decode '{}'", path.display()))?;
        let img = if auto_orient {
            apply_orientation(img, read_exif_orientation(path))
        } else {
            img
        };
        let (w, h) = img.dimensions();
        asset.width = Some(w);
        asset.height = Some(h);

        let thumb = img.thumbnail(300, 300);
        let thumb_path = thumb_dir.join(format!("{:04}.jpg", index));
        thumb.save(&thumb_path)
            .with_context(|| format!("cannot save thumbnail for '{}'", path.display()))?;
        asset.thumbnail_path = Some(thumb_path);
    } else {
        let (w, h) = image::image_dimensions(path)
            .with_context(|| format!("cannot read dimensions of '{}'", path.display()))?;
        asset.width = Some(w);
        asset.height = Some(h);
    }

    read_exif(asset, path);
    Ok(())
}

fn read_exif_orientation(path: &Path) -> u32 {
    let Ok(file) = std::fs::File::open(path) else { return 1 };
    let mut reader = std::io::BufReader::new(file);
    let Ok(exif_data) = exif::Reader::new().read_from_container(&mut reader) else { return 1 };

    exif_data
        .get_field(exif::Tag::Orientation, exif::In::PRIMARY)
        .and_then(|f| f.value.get_uint(0))
        .unwrap_or(1)
}

fn apply_orientation(img: image::DynamicImage, orientation: u32) -> image::DynamicImage {
    match orientation {
        2 => img.fliph(),
        3 => img.rotate180(),
        4 => img.flipv(),
        5 => img.rotate90().fliph(),
        6 => img.rotate90(),
        7 => img.rotate270().fliph(),
        8 => img.rotate270(),
        _ => img,
    }
}

fn read_exif(asset: &mut Asset, path: &Path) {
    let Ok(file) = std::fs::File::open(path) else { return };
    let mut reader = std::io::BufReader::new(file);
    let Ok(exif_data) = exif::Reader::new().read_from_container(&mut reader) else { return };

    if let Some(field) = exif_data.get_field(exif::Tag::ColorSpace, exif::In::PRIMARY) {
        asset.color_space = Some(field.display_value().to_string());
    }
}

fn process_video(
    asset: &mut Asset,
    path: &Path,
    thumb_dir: &Path,
    index: usize,
    gen_thumbnails: bool,
) {
    if let Ok(output) = std::process::Command::new("ffprobe")
        .args(["-v", "quiet", "-print_format", "json", "-show_streams", "-show_format"])
        .arg(path)
        .output()
    {
        if let Ok(json) = serde_json::from_slice::<serde_json::Value>(&output.stdout) {
            if let Some(streams) = json["streams"].as_array() {
                for stream in streams {
                    if stream["codec_type"].as_str() == Some("video") {
                        asset.width = stream["width"].as_u64().map(|v| v as u32);
                        asset.height = stream["height"].as_u64().map(|v| v as u32);
                        asset.codec = stream["codec_name"].as_str().map(String::from);
                        break;
                    }
                }
            }
            if let Some(duration) = json["format"]["duration"].as_str() {
                asset.duration = duration.parse::<f64>().ok();
            }
        }
    }

    if gen_thumbnails {
        let thumb_path = thumb_dir.join(format!("{:04}.jpg", index));
        let status = std::process::Command::new("ffmpeg")
            .args(["-y", "-ss", "1", "-i"])
            .arg(path)
            .args(["-frames:v", "1", "-vf", "scale=300:-1"])
            .arg(&thumb_path)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status();

        if status.map_or(false, |s| s.success()) {
            asset.thumbnail_path = Some(thumb_path);
        }
    }
}
