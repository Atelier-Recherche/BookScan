use anyhow::{bail, Result};
use image::{Rgb, RgbImage};

use super::detect::{Point, Quad};

fn dist(a: Point, b: Point) -> f32 {
    (a.x - b.x).hypot(a.y - b.y)
}

pub fn perspective_crop(img: &RgbImage, quad: &Quad) -> Result<RgbImage> {
    let width_a = dist(quad.br, quad.bl);
    let width_b = dist(quad.tr, quad.tl);
    let height_a = dist(quad.tr, quad.br);
    let height_b = dist(quad.tl, quad.bl);
    let w = width_a.max(width_b).round().max(1.0).min(5000.0) as u32;
    let h = height_a.max(height_b).round().max(1.0).min(5000.0) as u32;
    if w == 0 || h == 0 {
        bail!("invalid crop size");
    }

    let src = [quad.tl, quad.tr, quad.br, quad.bl];
    let dst = [
        Point { x: 0.0, y: 0.0 },
        Point {
            x: (w - 1) as f32,
            y: 0.0,
        },
        Point {
            x: (w - 1) as f32,
            y: (h - 1) as f32,
        },
        Point {
            x: 0.0,
            y: (h - 1) as f32,
        },
    ];
    let m = perspective_matrix(&dst, &src); // map dest -> source for sampling

    let mut out = RgbImage::new(w, h);
    for y in 0..h {
        for x in 0..w {
            let (sx, sy) = apply_perspective(&m, x as f32, y as f32);
            out.put_pixel(x, y, sample_bilinear(img, sx, sy));
        }
    }
    Ok(out)
}

/// 3x3 row-major homography.
fn perspective_matrix(src: &[Point; 4], dst: &[Point; 4]) -> [f32; 9] {
    // Solve for H mapping src -> dst using DLT (8 equations).
    let mut a = [[0.0f32; 8]; 8];
    let mut b = [0.0f32; 8];
    for i in 0..4 {
        let (x, y) = (src[i].x, src[i].y);
        let (u, v) = (dst[i].x, dst[i].y);
        let r = i * 2;
        a[r] = [x, y, 1.0, 0.0, 0.0, 0.0, -u * x, -u * y];
        b[r] = u;
        a[r + 1] = [0.0, 0.0, 0.0, x, y, 1.0, -v * x, -v * y];
        b[r + 1] = v;
    }
    let h8 = solve8(&a, &b);
    [
        h8[0], h8[1], h8[2], h8[3], h8[4], h8[5], h8[6], h8[7], 1.0,
    ]
}

fn solve8(a: &[[f32; 8]; 8], b: &[f32; 8]) -> [f32; 8] {
    let mut m = [[0.0f32; 9]; 8];
    for i in 0..8 {
        for j in 0..8 {
            m[i][j] = a[i][j];
        }
        m[i][8] = b[i];
    }
    for col in 0..8 {
        let mut pivot = col;
        for r in col + 1..8 {
            if m[r][col].abs() > m[pivot][col].abs() {
                pivot = r;
            }
        }
        m.swap(col, pivot);
        let div = m[col][col];
        if div.abs() < 1e-8 {
            continue;
        }
        for j in col..9 {
            m[col][j] /= div;
        }
        for r in 0..8 {
            if r == col {
                continue;
            }
            let f = m[r][col];
            for j in col..9 {
                m[r][j] -= f * m[col][j];
            }
        }
    }
    let mut x = [0.0f32; 8];
    for i in 0..8 {
        x[i] = m[i][8];
    }
    x
}

fn apply_perspective(m: &[f32; 9], x: f32, y: f32) -> (f32, f32) {
    let w = m[6] * x + m[7] * y + m[8];
    let inv = if w.abs() < 1e-8 { 1.0 } else { 1.0 / w };
    (
        (m[0] * x + m[1] * y + m[2]) * inv,
        (m[3] * x + m[4] * y + m[5]) * inv,
    )
}

fn sample_bilinear(img: &RgbImage, x: f32, y: f32) -> Rgb<u8> {
    let w = img.width() as i32;
    let h = img.height() as i32;
    if x < 0.0 || y < 0.0 || x >= (w - 1) as f32 || y >= (h - 1) as f32 {
        let xi = x.round().clamp(0.0, (w - 1) as f32) as u32;
        let yi = y.round().clamp(0.0, (h - 1) as f32) as u32;
        return *img.get_pixel(xi, yi);
    }
    let x0 = x.floor() as u32;
    let y0 = y.floor() as u32;
    let x1 = x0 + 1;
    let y1 = y0 + 1;
    let fx = x - x0 as f32;
    let fy = y - y0 as f32;
    let p00 = img.get_pixel(x0, y0).0;
    let p10 = img.get_pixel(x1, y0).0;
    let p01 = img.get_pixel(x0, y1).0;
    let p11 = img.get_pixel(x1, y1).0;
    let mut out = [0u8; 3];
    for c in 0..3 {
        let v = (1.0 - fx) * (1.0 - fy) * p00[c] as f32
            + fx * (1.0 - fy) * p10[c] as f32
            + (1.0 - fx) * fy * p01[c] as f32
            + fx * fy * p11[c] as f32;
        out[c] = v.round().clamp(0.0, 255.0) as u8;
    }
    Rgb(out)
}

pub fn dewarp_book_curve(img: &RgbImage) -> RgbImage {
    let (w, h) = (img.width(), img.height());
    let cols = 32usize.min((w as usize / 8).max(8));
    let col_w = w as f32 / cols as f32;

    // Build inverted text mask via simple adaptive-ish threshold
    let mut binary = vec![0u8; (w * h) as usize];
    for y in 0..h {
        for x in 0..w {
            let p = img.get_pixel(x, y);
            let g = (0.299 * p[0] as f32 + 0.587 * p[1] as f32 + 0.114 * p[2] as f32) as u8;
            binary[(y * w + x) as usize] = if g < 140 { 255 } else { 0 };
        }
    }

    let mut shifts = vec![0.0f32; cols];
    for i in 0..cols {
        let x0 = (i as f32 * col_w) as u32;
        let x1 = (((i + 1) as f32 * col_w) as u32).min(w);
        let mut sum_y = 0.0f32;
        let mut count = 0.0f32;
        for y in 0..h {
            for x in x0..x1 {
                if binary[(y * w + x) as usize] > 0 {
                    sum_y += y as f32;
                    count += 1.0;
                }
            }
        }
        shifts[i] = if count > 50.0 {
            sum_y / count - h as f32 * 0.45
        } else {
            0.0
        };
    }

    // Smooth shifts
    let mut smooth = shifts.clone();
    for i in 0..cols {
        let mut acc = 0.0;
        let mut n = 0.0;
        for k in i.saturating_sub(2)..=(i + 2).min(cols - 1) {
            acc += shifts[k];
            n += 1.0;
        }
        smooth[i] = acc / n;
    }
    let median = {
        let mut s = smooth.clone();
        s.sort_by(|a, b| a.partial_cmp(b).unwrap());
        s[s.len() / 2]
    };
    for v in &mut smooth {
        *v -= median;
    }

    let mut out = RgbImage::new(w, h);
    for y in 0..h {
        for x in 0..w {
            let xf = x as f32;
            let col_f = (xf / col_w).clamp(0.0, (cols - 1) as f32);
            let i0 = col_f.floor() as usize;
            let i1 = (i0 + 1).min(cols - 1);
            let t = col_f - i0 as f32;
            let shift = smooth[i0] * (1.0 - t) + smooth[i1] * t;
            let sy = (y as f32 + shift).clamp(0.0, (h - 1) as f32);
            out.put_pixel(x, y, sample_bilinear(img, xf, sy));
        }
    }
    out
}
