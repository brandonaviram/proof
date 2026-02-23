# proof

Your folder already knows what's in it. proof just makes it official.

Point it at a directory of finals. Get a branded PDF with a cover page, contact sheet, and manifest table. Apercu Pro typography. Under 20 seconds.

Built for creative agencies that treat the last mile with the same discipline as the first.

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
proof ./finals --client "Armani" --title "SS26 Campaign"
```

That's it. TUI dashboard shows progress. PDF lands in the current directory.

```sh
# Custom columns and output path
proof ./finals --client "Vogue" --columns 6 -o vogue-delivery.pdf

# Auto-rotate thumbnails using EXIF orientation
proof ./finals --client "Armani" --auto-orient

# Plain text mode (no TUI)
proof ./finals --client "Aviram" --no-tui

# Manifest only. TSV to stdout.
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
| `--auto-orient` | Rotate thumbnails per EXIF | off |
| `--manifest-only` | TSV manifest to stdout | |
| `--no-tui` | Plain text instead of TUI | |

## What You Get

- **Cover page.** Client, title, date, file count, total size.
- **Contact sheet.** Thumbnail grid. Configurable columns.
- **Manifest table.** Filename, type, resolution, format, size.
- **Summary.** Totals with image/video breakdown.

All typeset in Apercu Pro. All derived from the files themselves.

## Supported Formats

**Images:** JPG, PNG, TIFF, WebP
**Video:** MP4, MOV, MXF (requires ffmpeg)

## Built With

[Rust](https://www.rust-lang.org/) / [Typst](https://typst.app) / [ratatui](https://ratatui.rs) / [rayon](https://github.com/rayon-rs/rayon) / [Claude Code](https://claude.ai/claude-code)
