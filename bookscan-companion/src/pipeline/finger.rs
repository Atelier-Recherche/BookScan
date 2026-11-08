use image::{Rgb, RgbImage};

pub fn remove_fingers(img: &RgbImage) -> RgbImage {
    let (w, h) = (img.width(), img.height());
    let mut mask = vec![0u8; (w * h) as usize];

    for y in 0..h {
        for x in 0..w {
            let p = img.get_pixel(x, y);
            let (yy, cr, cb) = rgb_to_ycrcb(p[0], p[1], p[2]);
            let skin = (133..=173).contains(&cr) && (77..=127).contains(&cb) && yy > 40;
            let edge_bias = x < w / 8 || x > w * 7 / 8 || y > h * 11 / 20;
            if skin && edge_bias {
                mask[(y * w + x) as usize] = 255;
            }
        }
    }

    // Dilate a bit
    let mut dil = mask.clone();
    for y in 1..h - 1 {
        for x in 1..w - 1 {
            if mask[(y * w + x) as usize] > 0 {
                for dy in -1i32..=1 {
                    for dx in -1i32..=1 {
                        let xx = (x as i32 + dx) as u32;
                        let yy = (y as i32 + dy) as u32;
                        dil[(yy * w + xx) as usize] = 255;
                    }
                }
            }
        }
    }

    inpaint(img, &dil)
}

fn rgb_to_ycrcb(r: u8, g: u8, b: u8) -> (u8, u8, u8) {
    let rf = r as f32;
    let gf = g as f32;
    let bf = b as f32;
    let y = (0.299 * rf + 0.587 * gf + 0.114 * bf).round() as u8;
    let cr = (128.0 + 0.5 * rf - 0.418688 * gf - 0.081312 * bf).round().clamp(0.0, 255.0) as u8;
    let cb = (128.0 - 0.168736 * rf - 0.331264 * gf + 0.5 * bf).round().clamp(0.0, 255.0) as u8;
    (y, cr, cb)
}

fn inpaint(img: &RgbImage, mask: &[u8]) -> RgbImage {
    let (w, h) = (img.width(), img.height());
    let mut out = img.clone();
    for y in 0..h {
        for x in 0..w {
            if mask[(y * w + x) as usize] == 0 {
                continue;
            }
            // Average from nearest non-masked neighbors in expanding radius
            let mut found = false;
            for radius in 2..40 {
                let mut acc = [0u32; 3];
                let mut n = 0u32;
                let r = radius as i32;
                for dy in -r..=r {
                    for dx in -r..=r {
                        let xx = x as i32 + dx;
                        let yy = y as i32 + dy;
                        if xx < 0 || yy < 0 || xx >= w as i32 || yy >= h as i32 {
                            continue;
                        }
                        let idx = (yy as u32 * w + xx as u32) as usize;
                        if mask[idx] > 0 {
                            continue;
                        }
                        let p = img.get_pixel(xx as u32, yy as u32);
                        acc[0] += p[0] as u32;
                        acc[1] += p[1] as u32;
                        acc[2] += p[2] as u32;
                        n += 1;
                    }
                }
                if n > 0 {
                    out.put_pixel(
                        x,
                        y,
                        Rgb([(acc[0] / n) as u8, (acc[1] / n) as u8, (acc[2] / n) as u8]),
                    );
                    found = true;
                    break;
                }
            }
            if !found {
                out.put_pixel(x, y, Rgb([245, 245, 245]));
            }
        }
    }
    out
}
