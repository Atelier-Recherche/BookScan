use image::{GrayImage, Luma, RgbImage};
use imageproc::contours::{find_contours, BorderType};
use imageproc::edges::canny;
use imageproc::filter::gaussian_blur_f32;

#[derive(Debug, Clone, Copy)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

#[derive(Debug, Clone, Copy)]
pub struct Quad {
    pub tl: Point,
    pub tr: Point,
    pub br: Point,
    pub bl: Point,
}

pub fn default_full_frame_quad(w: u32, h: u32) -> Quad {
    let margin = 0.05f32;
    let mx = w as f32 * margin;
    let my = h as f32 * margin;
    Quad {
        tl: Point { x: mx, y: my },
        tr: Point {
            x: w as f32 - mx,
            y: my,
        },
        br: Point {
            x: w as f32 - mx,
            y: h as f32 - my,
        },
        bl: Point {
            x: mx,
            y: h as f32 - my,
        },
    }
}

fn order_corners(pts: [(f32, f32); 4]) -> Quad {
    let mut indexed: Vec<(usize, f32, f32)> = pts
        .iter()
        .enumerate()
        .map(|(i, &(x, y))| (i, x, y))
        .collect();
    // sum for tl/br, diff for tr/bl
    indexed.sort_by(|a, b| (a.1 + a.2).partial_cmp(&(b.1 + b.2)).unwrap());
    let tl = indexed[0];
    let br = indexed[3];
    let mut rest = vec![indexed[1], indexed[2]];
    rest.sort_by(|a, b| (a.1 - a.2).partial_cmp(&(b.1 - b.2)).unwrap());
    let tr = rest[0];
    let bl = rest[1];
    Quad {
        tl: Point { x: tl.1, y: tl.2 },
        tr: Point { x: tr.1, y: tr.2 },
        br: Point { x: br.1, y: br.2 },
        bl: Point { x: bl.1, y: bl.2 },
    }
}

fn to_gray(rgb: &RgbImage) -> GrayImage {
    GrayImage::from_fn(rgb.width(), rgb.height(), |x, y| {
        let p = rgb.get_pixel(x, y);
        let y8 = (0.299 * p[0] as f32 + 0.587 * p[1] as f32 + 0.114 * p[2] as f32) as u8;
        Luma([y8])
    })
}

fn approx_quad(points: &[(f32, f32)]) -> Option<[(f32, f32); 4]> {
    if points.len() < 4 {
        return None;
    }
    // Take convex-ish extremes via PCA-free bounding: sample farthest points iteratively
    let hull = points.to_vec();
    // Simplify: if already ~4, use; else pick 4 extremes
    if hull.len() == 4 {
        return Some([hull[0], hull[1], hull[2], hull[3]]);
    }
    let mut xs: Vec<f32> = hull.iter().map(|p| p.0).collect();
    let mut ys: Vec<f32> = hull.iter().map(|p| p.1).collect();
    xs.sort_by(|a, b| a.partial_cmp(b).unwrap());
    ys.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let min_x = xs[0];
    let max_x = *xs.last().unwrap();
    let min_y = ys[0];
    let max_y = *ys.last().unwrap();
    // nearest to each corner of bbox
    let corners = [
        (min_x, min_y),
        (max_x, min_y),
        (max_x, max_y),
        (min_x, max_y),
    ];
    let mut out = [(0f32, 0f32); 4];
    for (i, c) in corners.iter().enumerate() {
        out[i] = hull
            .iter()
            .copied()
            .min_by(|a, b| {
                let da = (a.0 - c.0).hypot(a.1 - c.1);
                let db = (b.0 - c.0).hypot(b.1 - c.1);
                da.partial_cmp(&db).unwrap()
            })
            .unwrap();
    }
    Some(out)
}

pub fn find_document_quad(rgb: &RgbImage) -> Option<Quad> {
    let max_side = 960u32;
    let (w, h) = (rgb.width(), rgb.height());
    let scale = if w.max(h) > max_side {
        max_side as f32 / w.max(h) as f32
    } else {
        1.0
    };
    let work = if (scale - 1.0).abs() > f32::EPSILON {
        image::imageops::resize(
            rgb,
            (w as f32 * scale) as u32,
            (h as f32 * scale) as u32,
            image::imageops::FilterType::Triangle,
        )
    } else {
        rgb.clone()
    };

    let gray = to_gray(&work);
    let blurred = gaussian_blur_f32(&gray, 1.2);
    let edges = canny(&blurred, 50.0, 100.0);
    let contours = find_contours::<u32>(&edges);

    let img_area = (work.width() * work.height()) as f32;
    let mut best: Option<(f32, [(f32, f32); 4])> = None;

    for c in contours {
        if c.border_type != BorderType::Outer {
            continue;
        }
        if c.points.len() < 4 {
            continue;
        }
        let pts: Vec<(f32, f32)> = c
            .points
            .iter()
            .map(|p| (p.x as f32, p.y as f32))
            .collect();
        let Some(quad_pts) = approx_quad(&pts) else {
            continue;
        };
        let area = polygon_area(&quad_pts);
        if area < img_area * 0.12 {
            continue;
        }
        if best.as_ref().map(|(a, _)| area > *a).unwrap_or(true) {
            best = Some((area, quad_pts));
        }
    }

    let (_, pts) = best?;
    let mut ordered = order_corners(pts);
    if scale != 1.0 {
        let inv = 1.0 / scale;
        ordered.tl.x *= inv;
        ordered.tl.y *= inv;
        ordered.tr.x *= inv;
        ordered.tr.y *= inv;
        ordered.br.x *= inv;
        ordered.br.y *= inv;
        ordered.bl.x *= inv;
        ordered.bl.y *= inv;
    }
    Some(ordered)
}

fn polygon_area(pts: &[(f32, f32); 4]) -> f32 {
    let mut a = 0.0;
    for i in 0..4 {
        let (x1, y1) = pts[i];
        let (x2, y2) = pts[(i + 1) % 4];
        a += x1 * y2 - x2 * y1;
    }
    a.abs() * 0.5
}
