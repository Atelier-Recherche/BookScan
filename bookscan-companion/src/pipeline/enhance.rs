use image::{Rgb, RgbImage};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EnhanceMode {
    Original,
    Color,
    Gray,
    Bw,
    Sharpen,
}

impl EnhanceMode {
    pub fn label(self) -> &'static str {
        match self {
            Self::Original => "original",
            Self::Color => "color",
            Self::Gray => "gray",
            Self::Bw => "bw",
            Self::Sharpen => "sharpen",
        }
    }

    pub fn all() -> [EnhanceMode; 5] {
        [
            Self::Original,
            Self::Color,
            Self::Gray,
            Self::Bw,
            Self::Sharpen,
        ]
    }
}

pub fn enhance(img: &RgbImage, mode: EnhanceMode) -> RgbImage {
    match mode {
        EnhanceMode::Original => img.clone(),
        EnhanceMode::Color => sharpen(&clahe_lite(img), 0.4),
        EnhanceMode::Gray => {
            let (w, h) = (img.width(), img.height());
            RgbImage::from_fn(w, h, |x, y| {
                let p = img.get_pixel(x, y);
                let g = luma(p);
                Rgb([g, g, g])
            })
        }
        EnhanceMode::Bw => {
            let (w, h) = (img.width(), img.height());
            RgbImage::from_fn(w, h, |x, y| {
                let p = img.get_pixel(x, y);
                let g = luma(p);
                // local mean approx via fixed threshold for speed
                let v = if g > 160 { 255 } else { 0 };
                Rgb([v, v, v])
            })
        }
        EnhanceMode::Sharpen => sharpen(img, 1.0),
    }
}

fn luma(p: &Rgb<u8>) -> u8 {
    (0.299 * p[0] as f32 + 0.587 * p[1] as f32 + 0.114 * p[2] as f32) as u8
}

fn clahe_lite(img: &RgbImage) -> RgbImage {
    // Lightweight contrast stretch on luminance
    let (w, h) = (img.width(), img.height());
    let mut min_l = 255u8;
    let mut max_l = 0u8;
    for p in img.pixels() {
        let g = luma(p);
        min_l = min_l.min(g);
        max_l = max_l.max(g);
    }
    let range = (max_l - min_l).max(1) as f32;
    RgbImage::from_fn(w, h, |x, y| {
        let p = img.get_pixel(x, y);
        let g = luma(p) as f32;
        let ng = ((g - min_l as f32) / range * 255.0).clamp(0.0, 255.0);
        let scale = if g < 1.0 { 1.0 } else { ng / g };
        Rgb([
            (p[0] as f32 * scale).clamp(0.0, 255.0) as u8,
            (p[1] as f32 * scale).clamp(0.0, 255.0) as u8,
            (p[2] as f32 * scale).clamp(0.0, 255.0) as u8,
        ])
    })
}

fn sharpen(img: &RgbImage, amount: f32) -> RgbImage {
    let (w, h) = (img.width(), img.height());
    let blurred = box_blur(img, 1);
    RgbImage::from_fn(w, h, |x, y| {
        let a = img.get_pixel(x, y);
        let b = blurred.get_pixel(x, y);
        Rgb([
            ((a[0] as f32) * (1.0 + amount) - (b[0] as f32) * amount).clamp(0.0, 255.0) as u8,
            ((a[1] as f32) * (1.0 + amount) - (b[1] as f32) * amount).clamp(0.0, 255.0) as u8,
            ((a[2] as f32) * (1.0 + amount) - (b[2] as f32) * amount).clamp(0.0, 255.0) as u8,
        ])
    })
}

fn box_blur(img: &RgbImage, radius: i32) -> RgbImage {
    let (w, h) = (img.width() as i32, img.height() as i32);
    RgbImage::from_fn(img.width(), img.height(), |x, y| {
        let mut acc = [0u32; 3];
        let mut n = 0u32;
        for dy in -radius..=radius {
            for dx in -radius..=radius {
                let xx = (x as i32 + dx).clamp(0, w - 1) as u32;
                let yy = (y as i32 + dy).clamp(0, h - 1) as u32;
                let p = img.get_pixel(xx, yy);
                acc[0] += p[0] as u32;
                acc[1] += p[1] as u32;
                acc[2] += p[2] as u32;
                n += 1;
            }
        }
        Rgb([(acc[0] / n) as u8, (acc[1] / n) as u8, (acc[2] / n) as u8])
    })
}
