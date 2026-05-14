//! In-process Typst compiler. Targets typst 0.13.

use anyhow::{anyhow, Result};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;
use typst::diag::{FileError, FileResult};
use typst::foundations::{Bytes, Datetime};
use typst::layout::PagedDocument;
use typst::syntax::{FileId, Source, VirtualPath};
use typst::text::{Font, FontBook};
use typst::utils::LazyHash;
use typst::{Library, LibraryExt, World};

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
        for data in typst_assets::fonts() {
            let bytes = Bytes::new(data.to_vec());
            let mut i = 0;
            while let Some(font) = Font::new(bytes.clone(), i) {
                fonts.push(font);
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

    /// Compile a complete typst source string and return a texture. The
    /// caller is responsible for wrapping the snippet body in the theme
    /// template (`mixed::wrap_source` does this).
    pub fn render(
        &self,
        ctx: &egui::Context,
        source: &str,
    ) -> Result<egui::TextureHandle> {
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
        let handle = ctx.load_texture(
            "vellum-snippet",
            color_image,
            egui::TextureOptions::LINEAR,
        );
        Ok(handle)
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
