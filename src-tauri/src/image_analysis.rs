use image::{DynamicImage, GenericImageView};

use crate::{config::AppConfig, error::AppResult};

#[derive(Debug, Clone)]
pub struct AnalysisConfig {
    pub blur_threshold: f64,
    pub exposure_tolerance: f64,
    pub cull_line: u8,
    pub auto_group: bool,
}

impl Default for AnalysisConfig {
    fn default() -> Self {
        Self {
            blur_threshold: 100.0,
            exposure_tolerance: 0.018,
            cull_line: 40,
            auto_group: true,
        }
    }
}

impl From<&AppConfig> for AnalysisConfig {
    fn from(config: &AppConfig) -> Self {
        Self {
            blur_threshold: config.blur_threshold.max(0.0),
            exposure_tolerance: config.exposure_tolerance.clamp(0.0, 1.0),
            cull_line: config.cull_line,
            auto_group: config.auto_group,
        }
    }
}

#[derive(Debug, Clone)]
pub struct BasicAnalysis {
    pub width: u32,
    pub height: u32,
    pub quality_score: f64,
    pub is_auto_culled: bool,
    pub is_blurred: bool,
    pub blur_score: f64,
    pub is_overexposed: bool,
    pub is_underexposed: bool,
    pub exposure_score: f64,
    pub perceptual_hash: u64,
}

pub fn analyze_image_with_config(
    path: &std::path::Path,
    config: &AnalysisConfig,
) -> AppResult<BasicAnalysis> {
    let image = image::open(path)?;
    let (width, height) = image.dimensions();
    let blur_variance = blur_variance(&image);
    let blur_score = (blur_variance / 1000.0).clamp(0.0, 1.0);
    let exposure = exposure_metrics(&image);
    let perceptual_hash = average_hash(&image);
    let is_blurred = blur_variance < config.blur_threshold;
    let is_overexposed = exposure.overexposed_ratio > config.exposure_tolerance;
    let is_underexposed = exposure.underexposed_ratio > config.exposure_tolerance;

    let blur_component = blur_score * 55.0;
    let exposure_component =
        (1.0 - (exposure.average_brightness - 0.5).abs() * 2.0).clamp(0.0, 1.0) * 35.0;
    let size_component = if width >= 1200 && height >= 800 {
        10.0
    } else {
        4.0
    };
    let quality_score = (blur_component + exposure_component + size_component).clamp(0.0, 100.0);
    let is_auto_culled =
        is_blurred || is_overexposed || is_underexposed || quality_score < config.cull_line as f64;

    Ok(BasicAnalysis {
        width,
        height,
        quality_score,
        is_auto_culled,
        is_blurred,
        blur_score,
        is_overexposed,
        is_underexposed,
        exposure_score: exposure.average_brightness,
        perceptual_hash,
    })
}

fn average_hash(image: &DynamicImage) -> u64 {
    let gray = image.thumbnail_exact(8, 8).to_luma8();
    let pixels = gray.as_raw();
    if pixels.is_empty() {
        return 0;
    }

    let average = pixels.iter().map(|pixel| *pixel as u64).sum::<u64>() / pixels.len() as u64;
    pixels
        .iter()
        .enumerate()
        .fold(0u64, |hash, (index, pixel)| {
            if *pixel as u64 >= average {
                hash | (1u64 << index)
            } else {
                hash
            }
        })
}

fn blur_variance(image: &DynamicImage) -> f64 {
    let gray = image.thumbnail(512, 512).to_luma8();
    let (width, height) = gray.dimensions();
    if width < 3 || height < 3 {
        return 0.0;
    }

    let pixels = gray.as_raw();
    let width_usize = width as usize;
    let mut sum_sq = 0.0;
    let mut count = 0usize;

    for y in 1..height - 1 {
        for x in 1..width - 1 {
            let index = y as usize * width_usize + x as usize;
            let top = pixels[index - width_usize] as f64;
            let left = pixels[index - 1] as f64;
            let center = pixels[index] as f64;
            let right = pixels[index + 1] as f64;
            let bottom = pixels[index + width_usize] as f64;
            let laplacian = top + left + right + bottom - 4.0 * center;
            sum_sq += laplacian * laplacian;
            count += 1;
        }
    }

    if count == 0 {
        return 0.0;
    }

    let variance = sum_sq / count as f64;
    variance
}

#[derive(Debug, Clone)]
struct ExposureMetrics {
    average_brightness: f64,
    overexposed_ratio: f64,
    underexposed_ratio: f64,
}

fn exposure_metrics(image: &DynamicImage) -> ExposureMetrics {
    let rgb = image.thumbnail(512, 512).to_rgb8();
    let pixels = rgb.as_raw();
    if pixels.is_empty() {
        return ExposureMetrics {
            average_brightness: 0.5,
            overexposed_ratio: 0.0,
            underexposed_ratio: 0.0,
        };
    }

    let mut total_brightness = 0.0;
    let mut count = 0usize;
    let mut overexposed = 0usize;
    let mut underexposed = 0usize;

    for chunk in pixels.chunks_exact(3) {
        let brightness =
            0.299 * chunk[0] as f64 + 0.587 * chunk[1] as f64 + 0.114 * chunk[2] as f64;
        total_brightness += brightness;
        count += 1;

        if chunk.iter().all(|value| *value >= 250) {
            overexposed += 1;
        }
        if chunk.iter().all(|value| *value <= 5) {
            underexposed += 1;
        }
    }

    if count == 0 {
        return ExposureMetrics {
            average_brightness: 0.5,
            overexposed_ratio: 0.0,
            underexposed_ratio: 0.0,
        };
    }

    ExposureMetrics {
        average_brightness: (total_brightness / count as f64 / 255.0).clamp(0.0, 1.0),
        overexposed_ratio: overexposed as f64 / count as f64,
        underexposed_ratio: underexposed as f64 / count as f64,
    }
}
