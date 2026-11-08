use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use printpdf::{ColorBits, ColorSpace, Image, ImageTransform, ImageXObject, Mm, PdfDocument, Px};

pub fn export_pages_to_pdf(pages: &[PathBuf], output: &Path) -> Result<()> {
    if pages.is_empty() {
        bail!("no pages to export");
    }

    let first = image::open(&pages[0]).context("open first page")?.to_rgb8();
    let (w, h) = first.dimensions();
    let width_mm = Mm((w as f32) * 0.264583);
    let height_mm = Mm((h as f32) * 0.264583);

    let (doc, page1, layer1) = PdfDocument::new("bookscan", width_mm, height_mm, "Layer 1");
    add_rgb_page(&doc, page1, layer1, &first, width_mm, height_mm)?;

    for page_path in pages.iter().skip(1) {
        let rgb = image::open(page_path)
            .with_context(|| format!("open {}", page_path.display()))?
            .to_rgb8();
        let (pw, ph) = rgb.dimensions();
        let wmm = Mm((pw as f32) * 0.264583);
        let hmm = Mm((ph as f32) * 0.264583);
        let (p, l) = doc.add_page(wmm, hmm, "Layer 1");
        add_rgb_page(&doc, p, l, &rgb, wmm, hmm)?;
    }

    let file = std::fs::File::create(output).context("create pdf")?;
    let mut writer = std::io::BufWriter::new(file);
    doc.save(&mut writer).context("save pdf")?;
    Ok(())
}

fn add_rgb_page(
    doc: &printpdf::PdfDocumentReference,
    page: printpdf::PdfPageIndex,
    layer: printpdf::PdfLayerIndex,
    rgb: &image::RgbImage,
    width_mm: Mm,
    height_mm: Mm,
) -> Result<()> {
    let (w, h) = rgb.dimensions();
    let image = Image::from(ImageXObject {
        width: Px(w as usize),
        height: Px(h as usize),
        color_space: ColorSpace::Rgb,
        bits_per_component: ColorBits::Bit8,
        interpolate: true,
        image_data: rgb.as_raw().clone(),
        image_filter: None,
        clipping_bbox: None,
        smask: None,
    });
    let layer = doc.get_page(page).get_layer(layer);
    image.add_to_layer(
        layer,
        ImageTransform {
            translate_x: Some(Mm(0.0)),
            translate_y: Some(Mm(0.0)),
            scale_x: Some(width_mm.0 / w as f32),
            scale_y: Some(height_mm.0 / h as f32),
            ..ImageTransform::default()
        },
    );
    Ok(())
}
