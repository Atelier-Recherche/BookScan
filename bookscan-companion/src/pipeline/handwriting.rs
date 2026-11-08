use image::{Rgb, RgbImage};

pub fn remove_handwriting(img: &RgbImage) -> RgbImage {
    let (w, h) = (img.width(), img.height());
    let mut mask = vec![0u8; (w * h) as usize];

    for y in 0..h {
        for x in 0..w {
            let p = img.get_pixel(x, y);
            let (h_deg, s, v) = rgb_to_hsv(p[0], p[1], p[2]);
            let blue = (90.0..=140.0).contains(&h_deg) && s > 0.25 && v > 0.15;
            let red = ((h_deg <= 12.0) || h_deg >= 165.0) && s > 0.35 && v > 0.15;
            let dark = v < 0.35 && s < 0.35;
            if blue || red || dark {
                // Prefer thin strokes: check local variance-ish by neighbor contrast
                let mut neighbors_bright = 0;
                for dy in -1i32..=1 {
                    for dx in -1i32..=1 {
                        if dx == 0 && dy == 0 {
                            continue;
                        }
                        let xx = (x as i32 + dx).clamp(0, w as i32 - 1) as u32;
                        let yy = (y as i32 + dy).clamp(0, h as i32 - 1) as u32;
                        let q = img.get_pixel(xx, yy);
                        let g = 0.299 * q[0] as f32 + 0.587 * q[1] as f32 + 0.114 * q[2] as f32;
                        if g > 180.0 {
                            neighbors_bright += 1;
                        }
                    }
                }
                if neighbors_bright >= 3 {
                    mask[(y * w + x) as usize] = 255;
                }
            }
        }
    }

    // Reuse inpaint path via temporary finger-style: apply mask by blending to neighbor
    // Fast path: paint white-ish then slight blur by calling remove_fingers style local fill
    let mut stamped = img.clone();
    for y in 0..h {
        for x in 0..w {
            if mask[(y * w + x) as usize] > 0 {
                stamped.put_pixel(x, y, Rgb([250, 250, 250]));
            }
        }
    }
    // Soften by one pass average on masked pixels
    let mut out = stamped.clone();
    for y in 1..h - 1 {
        for x in 1..w - 1 {
            if mask[(y * w + x) as usize] == 0 {
                continue;
            }
            let mut acc = [0u32; 3];
            for dy in -1i32..=1 {
                for dx in -1i32..=1 {
                    let p = stamped.get_pixel((x as i32 + dx) as u32, (y as i32 + dy) as u32);
                    acc[0] += p[0] as u32;
                    acc[1] += p[1] as u32;
                    acc[2] += p[2] as u32;
                }
            }
            out.put_pixel(x, y, Rgb([(acc[0] / 9) as u8, (acc[1] / 9) as u8, (acc[2] / 9) as u8]));
        }
    }
    out
}

fn rgb_to_hsv(r: u8, g: u8, b: u8) -> (f32, f32, f32) {
    let rf = r as f32 / 255.0;
    let gf = g as f32 / 255.0;
    let bf = b as f32 / 255.0;
    let max = rf.max(gf).max(bf);
    let min = rf.min(gf).min(bf);
    let d = max - min;
    let v = max;
    let s = if max < 1e-6 { 0.0 } else { d / max };
    let h = if d < 1e-6 {
        0.0
    } else if (max - rf).abs() < 1e-6 {
        60.0 * (((gf - bf) / d) % 6.0)
    } else if (max - gf).abs() < 1e-6 {
        60.0 * (((bf - rf) / d) + 2.0)
    } else {
        60.0 * (((rf - gf) / d) + 4.0)
    };
    let h = if h < 0.0 { h + 360.0 } else { h };
    (h, s, v)
}
