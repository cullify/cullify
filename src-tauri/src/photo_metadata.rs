use std::{
    fs,
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

use chrono::{DateTime, NaiveDateTime, Utc};
use exif::{In, Reader, Tag};

#[derive(Debug, Clone, Default)]
pub struct PhotoMetadata {
    pub captured_at: Option<String>,
    pub camera: Option<String>,
    pub lens: Option<String>,
    pub focal: Option<String>,
    pub aperture: Option<String>,
    pub shutter: Option<String>,
    pub iso: Option<String>,
}

pub fn read_photo_metadata(path: &Path) -> PhotoMetadata {
    let mut metadata = read_exif_metadata(path).unwrap_or_default();
    if metadata.captured_at.is_none() {
        metadata.captured_at = file_modified_at(path);
    }
    metadata
}

fn read_exif_metadata(path: &Path) -> Option<PhotoMetadata> {
    let file = fs::File::open(path).ok()?;
    let mut reader = std::io::BufReader::new(file);
    let exif = Reader::new().read_from_container(&mut reader).ok()?;

    let make = field_text(&exif, Tag::Make);
    let model = field_text(&exif, Tag::Model);
    let camera = match (make.as_deref(), model.as_deref()) {
        (Some(make), Some(model)) if !model.contains(make) => Some(format!("{make} {model}")),
        (_, Some(model)) => Some(model.to_string()),
        (Some(make), _) => Some(make.to_string()),
        _ => None,
    };

    Some(PhotoMetadata {
        captured_at: field_text(&exif, Tag::DateTimeOriginal)
            .or_else(|| field_text(&exif, Tag::DateTime))
            .and_then(|value| parse_exif_datetime(&value)),
        camera,
        lens: field_text(&exif, Tag::LensModel),
        focal: field_text(&exif, Tag::FocalLength),
        aperture: field_text(&exif, Tag::FNumber).map(|value| format!("f/{value}")),
        shutter: field_text(&exif, Tag::ExposureTime).map(|value| format!("{value} s")),
        iso: field_text(&exif, Tag::PhotographicSensitivity)
            .or_else(|| field_text(&exif, Tag::ISOSpeed)),
    })
}

fn field_text(exif: &exif::Exif, tag: Tag) -> Option<String> {
    exif.get_field(tag, In::PRIMARY)
        .map(|field| field.display_value().with_unit(exif).to_string())
        .map(clean_exif_value)
        .filter(|value| !value.is_empty())
}

fn clean_exif_value(value: String) -> String {
    value
        .replace('\0', "")
        .trim()
        .trim_matches('"')
        .trim()
        .to_string()
}

fn parse_exif_datetime(value: &str) -> Option<String> {
    NaiveDateTime::parse_from_str(value.trim(), "%Y:%m:%d %H:%M:%S")
        .ok()
        .map(|datetime| datetime.and_utc().to_rfc3339())
}

fn file_modified_at(path: &Path) -> Option<String> {
    let modified = fs::metadata(path).ok()?.modified().ok()?;
    system_time_to_rfc3339(modified)
}

fn system_time_to_rfc3339(time: SystemTime) -> Option<String> {
    let duration = time.duration_since(UNIX_EPOCH).ok()?;
    let datetime =
        DateTime::<Utc>::from_timestamp(duration.as_secs() as i64, duration.subsec_nanos())?;
    Some(datetime.to_rfc3339())
}
