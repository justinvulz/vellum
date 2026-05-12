use anyhow::{anyhow, Context, Result};
use pdfium_render::prelude::*;
use std::path::{Path, PathBuf};
use std::process::Command;

pub struct Page {
    pub texture: egui::TextureHandle,
    pub size: [usize; 2],
}

#[derive(Default)]
pub struct PreviewState {
    pub source_path: Option<PathBuf>,
    pub pdf_path: Option<PathBuf>,
    pub pages: Vec<Page>,
    pub error: Option<String>,
    pub fallback_text: String,
    pub target_width: u32,
}

impl PreviewState {
    pub fn new() -> Self {
        Self {
            target_width: 2400,
            ..Default::default()
        }
    }

    /// Compile `source_path` to PDF, then rasterize via pdfium straight into egui textures.
    pub fn compile(&mut self, ctx: &egui::Context, vault_root: &Path, source_path: &Path) {
        self.source_path = Some(source_path.to_path_buf());

        let out_dir = std::env::temp_dir().join("vellum-preview");
        if let Err(e) = std::fs::create_dir_all(&out_dir) {
            self.fail(format!("temp dir: {e}"), source_path);
            return;
        }

        let pdf_path = out_dir.join(format!("preview-{}.pdf", std::process::id()));
        if let Err(e) = run_typst(vault_root, source_path, &pdf_path) {
            self.fail(format!("typst: {e}"), source_path);
            return;
        }
        self.pdf_path = Some(pdf_path.clone());

        match render_pdf(ctx, &pdf_path, self.target_width) {
            Ok(pages) if !pages.is_empty() => {
                self.pages = pages;
                self.error = None;
                self.fallback_text.clear();
            }
            Ok(_) => self.fail("pdf has no pages".into(), source_path),
            Err(e) => self.fail(format!("pdfium: {e:#}"), source_path),
        }
    }

    fn fail(&mut self, msg: String, source_path: &Path) {
        self.pages.clear();
        self.error = Some(msg);
        self.fallback_text = std::fs::read_to_string(source_path).unwrap_or_default();
    }
}

fn run_typst(root: &Path, source: &Path, pdf_out: &Path) -> Result<()> {
    let output = Command::new("typst")
        .arg("compile")
        .arg("--root")
        .arg(root)
        .arg(source)
        .arg(pdf_out)
        .output()
        .context("spawning typst (is the `typst` CLI installed?)")?;
    if !output.status.success() {
        return Err(anyhow!(String::from_utf8_lossy(&output.stderr)
            .trim()
            .to_string()));
    }
    Ok(())
}

fn bind_pdfium() -> Result<Pdfium> {
    let mut last_err: Option<anyhow::Error> = None;
    if let Ok(dir) = std::env::var("PDFIUM_DYNAMIC_LIB_PATH") {
        let lib = Pdfium::pdfium_platform_library_name_at_path(&dir);
        match Pdfium::bind_to_library(&lib) {
            Ok(b) => return Ok(Pdfium::new(b)),
            Err(e) => last_err = Some(anyhow!("PDFIUM_DYNAMIC_LIB_PATH={dir}: {e}")),
        }
    }
    match Pdfium::bind_to_system_library() {
        Ok(b) => Ok(Pdfium::new(b)),
        Err(e) => Err(last_err
            .map(|prev| anyhow!("{prev}; system lookup also failed: {e}"))
            .unwrap_or_else(|| anyhow!("system lookup failed: {e}"))),
    }
}

fn render_pdf(ctx: &egui::Context, pdf: &Path, target_width: u32) -> Result<Vec<Page>> {
    let pdfium = bind_pdfium()?;

    let pdf_str = pdf
        .to_str()
        .ok_or_else(|| anyhow!("non-utf8 pdf path"))?;
    let document = pdfium
        .load_pdf_from_file(pdf_str, None)
        .context("loading pdf")?;

    let render_config = PdfRenderConfig::new().set_target_width(target_width as i32);

    let mut pages = Vec::new();
    for (i, page) in document.pages().iter().enumerate() {
        let bitmap = page
            .render_with_config(&render_config)
            .context("rasterizing page")?;
        let rgba = bitmap.as_image().into_rgba8();
        let size = [rgba.width() as usize, rgba.height() as usize];
        let pixels = rgba.into_raw();
        let color_image = egui::ColorImage::from_rgba_unmultiplied(size, &pixels);
        let handle = ctx.load_texture(
            format!("vellum-preview-{i}"),
            color_image,
            egui::TextureOptions::LINEAR,
        );
        pages.push(Page {
            texture: handle,
            size,
        });
    }
    Ok(pages)
}

pub fn open_externally(pdf: &Path) -> Result<()> {
    Command::new("xdg-open")
        .arg(pdf)
        .spawn()
        .with_context(|| format!("xdg-open {}", pdf.display()))?;
    Ok(())
}
