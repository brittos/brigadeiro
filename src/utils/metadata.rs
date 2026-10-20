use auto_palette::{ImageData, Palette};
use cosmic::iced::Color;
use image::DynamicImage;
use lofty::file::{AudioFile, EXTENSIONS, TaggedFileExt};
use lofty::probe::Probe;
use lofty::tag::Accessor;
use std::cmp::Reverse;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Default)]
pub struct AudioMetadata {
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub album_artist: Option<String>,
    pub date: Option<String>,
    pub genre: Option<String>,
    pub duration: f64,
    pub track_number: Option<u32>,
    pub disc_number: Option<u32>,
    pub file_type: String,
    pub cover: Option<DynamicImage>,
    pub accent_color: Option<Color>,
    pub secondary_color: Option<Color>,
    pub tertiary_color: Option<Color>,
    pub picture_data: Option<Vec<u8>>,
}

pub fn extract_metadata<P: AsRef<Path>>(path: P) -> Option<AudioMetadata> {
    let path = path.as_ref();
    let (mut metadata, _tagged_file, tag) = extract_metadata_inner(path)?;

    // Extract picture
    let pictures = tag.pictures();
    let cover = if let Some(pic) = pictures.first() {
        image::load_from_memory(pic.data()).ok()
    } else {
        None
    };

    // Process cover for background: Blur and Darken
    let cover = cover.map(|img| {
        let (processed, colors) = process_image_for_theme(&img);
        (processed, colors)
    });

    let (processed_cover, extracted_colors) = if let Some((img, colors)) = cover {
        (Some(img), colors)
    } else {
        (None, (None, None, None))
    };

    metadata.cover = processed_cover;
    metadata.accent_color = Some(
        extracted_colors
            .0
            .unwrap_or(Color::from_rgb8(0x3D, 0x67, 0x97)),
    );
    metadata.secondary_color = Some(
        extracted_colors
            .1
            .unwrap_or(Color::from_rgb8(0x1B, 0x2E, 0x44)),
    );
    metadata.tertiary_color = Some(
        extracted_colors
            .2
            .unwrap_or(Color::from_rgb8(0x56, 0x8A, 0xC2)),
    );

    Some(metadata)
}

/// Helper for basic metadata extraction shared between fast and full versions
fn extract_metadata_inner<P: AsRef<Path>>(
    path: P,
) -> Option<(AudioMetadata, lofty::file::TaggedFile, lofty::tag::Tag)> {
    let path = path.as_ref();

    // Probe the file
    let tagged_file = Probe::open(path).ok()?.read().ok()?;
    let tag = tagged_file
        .primary_tag()
        .or(tagged_file.first_tag())?
        .clone();

    let title = tag.title().map(|s| s.to_string());
    let artist = tag.artist().map(|s| s.to_string());
    let album = tag.album().map(|s| s.to_string());
    let album_artist = tag
        .get_string(lofty::tag::ItemKey::AlbumArtist)
        .map(std::string::ToString::to_string);
    let date = tag.date().map(|d| d.year.to_string());
    let genre = tag.genre().map(|s| s.to_string());
    let track_number = tag.track();
    let disc_number = tag.disk();
    let duration = tagged_file.properties().duration().as_secs_f64();
    let file_type = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_string();

    Some((
        AudioMetadata {
            title,
            artist,
            album,
            album_artist,
            date,
            genre,
            duration,
            track_number,
            disc_number,
            file_type,
            ..Default::default()
        },
        tagged_file,
        tag,
    ))
}

/// Fast metadata extraction (skips expensive image processing)
pub fn extract_metadata_fast<P: AsRef<Path>>(path: P) -> Option<AudioMetadata> {
    let (mut metadata, _, tag) = extract_metadata_inner(path)?;

    // Extract raw picture data for DB persistence (skip expensive processing)
    if let Some(pic) = tag.pictures().first() {
        metadata.picture_data = Some(pic.data().to_vec());
    }

    Some(metadata)
}

type ThemeColors = (Option<Color>, Option<Color>, Option<Color>);

/// Helper to process an image for the application theme (blur + color extraction)
pub fn process_image_for_theme(img: &DynamicImage) -> (DynamicImage, ThemeColors) {
    // 1. Resize first (used for both background blur AND color extraction)
    // Downscaling to 150x150 is enough for dominant colors and blur
    let resized = img.resize(150, 150, image::imageops::FilterType::Triangle); // resize image to 150x150

    // 2. Extract colors from RESIZED image (orders of magnitude faster)
    let colors = extract_colors(&resized).unwrap_or((None, None, None));

    // 3. Apply strong blur for background
    let blurred = resized.blur(20.0);

    (blurred, colors)
}

fn extract_colors(image: &DynamicImage) -> Option<(Option<Color>, Option<Color>, Option<Color>)> {
    let img = image.to_rgba8();
    let width = img.width();
    let height = img.height();
    let data = img.as_raw();

    // Create ImageData from raw bytes
    let image_data = ImageData::new(width, height, data).ok()?;

    let palette: Palette<f32> = Palette::extract(&image_data).ok()?;

    // palette.swatches() returns a slice or ref, convert to Vec to sort
    let mut swatches: Vec<_> = palette.swatches().iter().collect();
    // Sort by population descending
    swatches.sort_by_key(|s| Reverse(s.population()));

    let primary = swatches.first().and_then(|s| convert_color(s.color()));
    let secondary = swatches.get(1).and_then(|s| convert_color(s.color()));
    let tertiary = swatches.get(2).and_then(|s| convert_color(s.color()));

    Some((primary, secondary, tertiary))
}

fn convert_color(color: &auto_palette::color::Color<f32>) -> Option<Color> {
    if let Ok(hex) = std::panic::catch_unwind(|| color.to_hex_string()) {
        let hex = hex.trim_start_matches('#');
        if hex.len() == 6 {
            let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(0);
            let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(0);
            let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(0);
            return Some(Color::from_rgb8(r, g, b));
        }
    }
    None
}

/// Recursively scan a directory for audio files
pub fn scan_directory<P: AsRef<Path>>(path: P) -> Vec<PathBuf> {
    let mut files = Vec::new();
    if let Ok(entries) = std::fs::read_dir(path) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                files.extend(scan_directory(path));
            } else if is_audio_file(&path) {
                files.push(path);
            }
        }
    }
    files
}

/// Check if a file is a supported audio file
pub fn is_audio_file(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| EXTENSIONS.contains(&e.to_lowercase().as_str()))
}
