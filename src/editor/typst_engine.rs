//! In-process Typst compiler. Targets typst 0.13.

use anyhow::{anyhow, Result};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;
use typst::diag::{FileError, FileResult};
use typst::foundations::{Bytes, Datetime};
use typst::layout::{Frame, FrameItem, PagedDocument, Point};
use typst::model::Destination;
use typst::syntax::{FileId, Source, VirtualPath};
use typst::text::{Font, FontBook};
use typst::utils::LazyHash;
use typst::{Library, LibraryExt, World};

/// URL prefix the theme's `line-note` function emits and the engine
/// recognises as an inter-note navigation request.
pub const VELLUM_LINK_SCHEME: &str = "vellum://";

/// Successful render result: the texture plus any `vellum://` link
/// rectangles discovered in the compiled page frame.
#[derive(Clone)]
pub struct RenderedPage {
    pub texture: egui::TextureHandle,
    pub links: Vec<LinkRect>,
}

/// A clickable region inside a rendered segment, positioned in egui
/// logical points relative to the rendered image's top-left.
#[derive(Clone, Debug)]
pub struct LinkRect {
    pub rect: egui::Rect,
    /// Note name to navigate to — the part following `vellum://`.
    pub target: String,
}

const MAIN_PATH: &str = "/__vellum_snippet__.typ";

/// Pixels-per-typst-pt used when rasterising snippets. The cache is keyed
/// on snippet text alone, so this must stay constant. Values > 1 give
/// oversampling for sharper text on non-HiDPI displays.
pub const PIXEL_PER_PT: f32 = 2.0;

pub struct TypstEngine {
    library: LazyHash<Library>,
    book: LazyHash<FontBook>,
    fonts: Vec<Font>,
    root: PathBuf,
    main: Mutex<Source>,
    cache: Mutex<HashMap<FileId, FileEntry>>,
}

#[derive(Default)]
struct FileEntry {
    source: Option<Source>,
    bytes: Option<Bytes>,
}

impl TypstEngine {
    pub fn new(root: PathBuf) -> Result<Self> {
        let mut db = fontdb::Database::new();
        db.load_system_fonts();

        let mut fonts = Vec::new();
        let mut bundled = 0usize;
        for data in typst_assets::fonts() {
            let bytes = Bytes::new(data.to_vec());
            let mut i = 0;
            while let Some(font) = Font::new(bytes.clone(), i) {
                fonts.push(font);
                bundled += 1;
                i += 1;
            }
        }
        for face in db.faces() {
            if let fontdb::Source::File(path) = &face.source {
                if let Ok(data) = std::fs::read(path) {
                    let bytes = Bytes::new(data);
                    if let Some(font) = Font::new(bytes, face.index) {
                        fonts.push(font);
                    }
                }
            }
        }
        log::info!(
            "typst engine: {} fonts ({} bundled + {} system), root {}",
            fonts.len(),
            bundled,
            fonts.len() - bundled,
            root.display()
        );

        let book = FontBook::from_fonts(fonts.iter());
        let main_id = FileId::new(None, VirtualPath::new(MAIN_PATH));
        let main = Source::new(main_id, String::new());

        Ok(Self {
            library: LazyHash::new(Library::default()),
            book: LazyHash::new(book),
            fonts,
            root,
            main: Mutex::new(main),
            cache: Mutex::new(HashMap::new()),
        })
    }

    fn main_id() -> FileId {
        FileId::new(None, VirtualPath::new(MAIN_PATH))
    }

    /// Compile a complete typst source string and return the rendered
    /// texture plus any inter-note link rectangles. The caller is
    /// responsible for wrapping the snippet body in the theme template
    /// (`preamble::wrap_for_render` does this).
    pub fn render(
        &self,
        ctx: &egui::Context,
        source: &str,
    ) -> Result<RenderedPage> {
        *self.main.lock().unwrap() = Source::new(Self::main_id(), source.to_string());
        self.cache.lock().unwrap().clear();
        comemo::evict(0);

        let warned = typst::compile::<PagedDocument>(self);
        let document = match warned.output {
            Ok(d) => d,
            Err(errs) => {
                let msg = errs
                    .iter()
                    .map(|e| e.message.to_string())
                    .collect::<Vec<_>>()
                    .join("; ");
                return Err(anyhow!(msg));
            }
        };

        let page = document
            .pages
            .first()
            .ok_or_else(|| anyhow!("compiled document has no pages"))?;
        let pixmap = typst_render::render(page, PIXEL_PER_PT);

        let size = [pixmap.width() as usize, pixmap.height() as usize];
        let color_image =
            egui::ColorImage::from_rgba_unmultiplied(size, pixmap.data());
        let texture = ctx.load_texture(
            "vellum-snippet",
            color_image,
            egui::TextureOptions::LINEAR,
        );

        let mut links = Vec::new();
        collect_links(&page.frame, Point::zero(), &mut links);
        Ok(RenderedPage { texture, links })
    }
}

/// Walk the compiled page frame for `FrameItem::Link` entries whose
/// destination is a `vellum://...` URL, recording each rectangle in
/// egui logical points (1 typst pt ↔ 1 egui pt).
///
/// `Group` items are descended into with their transform's translation
/// component folded into the accumulated origin. We ignore rotation
/// and scale — `line-note` is plain inline text, so anything that
/// reaches us via groups is just nested layout, not a rotated callout.
fn collect_links(frame: &Frame, origin: Point, out: &mut Vec<LinkRect>) {
    for (point, item) in frame.items() {
        let here = Point::new(origin.x + point.x, origin.y + point.y);
        match item {
            FrameItem::Link(Destination::Url(url), size) => {
                let url_str: &str = url;
                if let Some(target) = url_str.strip_prefix(VELLUM_LINK_SCHEME) {
                    out.push(LinkRect {
                        rect: egui::Rect::from_min_size(
                            egui::pos2(here.x.to_pt() as f32, here.y.to_pt() as f32),
                            egui::vec2(
                                size.x.to_pt() as f32,
                                size.y.to_pt() as f32,
                            ),
                        ),
                        target: target.to_string(),
                    });
                }
            }
            FrameItem::Group(group) => {
                let child_origin = Point::new(
                    here.x + group.transform.tx,
                    here.y + group.transform.ty,
                );
                collect_links(&group.frame, child_origin, out);
            }
            _ => {}
        }
    }
}

impl World for TypstEngine {
    fn library(&self) -> &LazyHash<Library> {
        &self.library
    }

    fn book(&self) -> &LazyHash<FontBook> {
        &self.book
    }

    fn main(&self) -> FileId {
        Self::main_id()
    }

    fn source(&self, id: FileId) -> FileResult<Source> {
        if id == Self::main_id() {
            return Ok(self.main.lock().unwrap().clone());
        }
        let mut cache = self.cache.lock().unwrap();
        if let Some(entry) = cache.get(&id) {
            if let Some(source) = &entry.source {
                return Ok(source.clone());
            }
        }
        let path = id
            .vpath()
            .resolve(&self.root)
            .ok_or_else(|| FileError::NotFound(id.vpath().as_rooted_path().into()))?;
        let text = std::fs::read_to_string(&path)
            .map_err(|_| FileError::NotFound(path.clone()))?;
        let source = Source::new(id, text);
        cache.entry(id).or_default().source = Some(source.clone());
        Ok(source)
    }

    fn file(&self, id: FileId) -> FileResult<Bytes> {
        let mut cache = self.cache.lock().unwrap();
        if let Some(entry) = cache.get(&id) {
            if let Some(bytes) = &entry.bytes {
                return Ok(bytes.clone());
            }
        }
        let path = id
            .vpath()
            .resolve(&self.root)
            .ok_or_else(|| FileError::NotFound(id.vpath().as_rooted_path().into()))?;
        let data = std::fs::read(&path).map_err(|_| FileError::NotFound(path))?;
        let bytes = Bytes::new(data);
        cache.entry(id).or_default().bytes = Some(bytes.clone());
        Ok(bytes)
    }

    fn font(&self, index: usize) -> Option<Font> {
        self.fonts.get(index).cloned()
    }

    fn today(&self, _offset: Option<i64>) -> Option<Datetime> {
        Datetime::from_ymd(2024, 1, 1)
    }
}
