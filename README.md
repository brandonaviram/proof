# proof

Generate branded delivery proof PDFs from a folder of finals.

Scans a directory of images and videos, extracts metadata, generates thumbnails, and produces a professional PDF with cover page, contact sheet, and manifest table — all typeset in Apercu Pro via Typst.

## Install

```sh
cargo install --path .
```

Requires [Typst](https://typst.app):

```sh
brew install typst
```

Optional: `ffmpeg` and `ffprobe` for video thumbnails and metadata.

## Usage

```sh
# Full PDF with TUI dashboard (default)
proof ./finals --client "Armani" --title "SS26 Campaign"

# Plain text mode
proof ./finals --client "Aviram" --no-tui

# Custom columns and output path
proof ./finals --client "Vogue" --columns 6 -o vogue-delivery.pdf

# Auto-rotate thumbnails using EXIF orientation
proof ./finals --client "Armani" --auto-orient

# Manifest only — TSV to stdout
proof ./finals --manifest-only
```

## Options

| Flag | Description | Default |
|------|-------------|---------|
| `--client` | Client name on cover page | `Delivery` |
| `--title` | Document title | none |
| `--date` | Delivery date | today |
| `--columns` | Contact sheet columns (3-8) | `4` |
| `-o, --output` | Output PDF path | `{client}-delivery-{date}.pdf` |
| `--auto-orient` | Rotate thumbnails using EXIF orientation | off |
| `--manifest-only` | TSV manifest to stdout | |
| `--no-tui` | Plain text instead of TUI | |

## PDF Output

- **Cover page** — client, title, date, file count, total size
- **Contact sheet** — thumbnail grid (configurable columns)
- **Manifest table** — filename, type, resolution, format, size
- **Summary** — totals with image/video breakdown

## Supported Formats

**Images:** JPG, PNG, TIFF, WebP
**Video:** MP4, MOV, MXF (requires ffmpeg)
