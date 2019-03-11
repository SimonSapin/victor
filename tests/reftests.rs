use std::env;
use std::fs;
use std::path::{Path, PathBuf};

fn main() {
    for_each_file_in(reftests_dir(), &mut |path| TestFile::load(path).test())
}

fn target_dir() -> PathBuf {
    let current = env::current_dir().unwrap();
    let exe = current.join(env::current_exe().unwrap());
    let deps = exe.parent().unwrap();
    let debug = deps.parent().unwrap();
    let target = debug.parent().unwrap();
    target.to_owned()
}

fn reftests_dir() -> PathBuf {
    let target = target_dir();
    let repo = target.parent().unwrap();
    repo.join("tests").join("reftests")
}

fn for_each_file_in(path: PathBuf, f: &mut impl FnMut(PathBuf)) {
    for entry in path.read_dir().unwrap() {
        let entry = entry.unwrap();
        let type_ = entry.file_type().unwrap();
        if type_.is_dir() {
            for_each_file_in(entry.path(), f)
        }
        if type_.is_file() {
            f(entry.path())
        }
    }
}

struct TestFile {
    path: PathBuf,
    doc: Option<victor::dom::Document>,
    pdf: Option<Vec<u8>>,
    pages_pixels: Option<Vec<lester::ImageSurface>>,
}

impl TestFile {
    fn load(path: PathBuf) -> Self {
        let bytes = fs::read(&path).unwrap();
        let mut doc = None;
        let mut pdf = None;
        let mut pages_pixels = None;
        match path.extension().and_then(|e| e.to_str()) {
            Some("html") => doc = Some(victor::dom::Document::parse_html(&bytes)),
            Some("pdf") => pdf = Some(bytes),
            Some("png") => {
                pages_pixels = Some(vec![lester::ImageSurface::read_from_png(&*bytes).unwrap()])
            }
            ext => panic!("Unsupported file extension: {:?}", ext),
        }
        Self {
            path,
            doc,
            pdf,
            pages_pixels,
        }
    }

    fn pdf_bytes(&mut self) -> &[u8] {
        let doc = self.doc.as_ref().unwrap();
        self.pdf.get_or_insert_with(|| doc.to_pdf_bytes())
    }

    fn pages_pixels(&mut self) -> &mut [lester::ImageSurface] {
        if self.pages_pixels.is_none() {
            let pages = lester::PdfDocument::from_bytes(self.pdf_bytes())
                .unwrap()
                .pages()
                .map(|page| page.render().unwrap())
                .collect();
            self.pages_pixels = Some(pages)
        }
        self.pages_pixels.as_mut().unwrap()
    }

    fn expect_single_page(&mut self) -> lester::Argb32Pixels {
        let pages = self.pages_pixels();
        assert_eq!(pages.len(), 1);
        pages[0].pixels()
    }

    fn write_png(&mut self, path: &Path) {
        self.pages_pixels()[0].write_to_png_file(path).unwrap()
    }

    fn test(&mut self) {
        let base = self.path.parent().unwrap();
        let references = self.doc.as_ref().map_or(Vec::new(), |doc| {
            doc.html_link_elements()
                .filter_map(|(rel, href)| match rel {
                    "match" => Some((true, resolve_href(base, href))),
                    "mismatch" => Some((false, resolve_href(base, href))),
                    _ => None,
                })
                .collect()
        });
        let page = self.expect_single_page();
        for (expect_equal, reference_path) in references {
            let mut reference = Self::load(reference_path);
            let reference_page = reference.expect_single_page();
            if (page == reference_page) != expect_equal {
                let test_png = target_dir().join("test.png");
                let reference_png = target_dir().join("reference.png");
                self.write_png(&test_png);
                reference.write_png(&reference_png);
                std::fs::write(target_dir().join("test.pdf"), self.pdf_bytes()).unwrap();
                panic!(
                    "Failed {} {} ↔ {}\n{}\n{}",
                    if expect_equal { "match" } else { "mismatch" },
                    show(&self.path),
                    show(&reference.path),
                    show(&test_png),
                    show(&reference_png),
                )
            }
        }
    }
}

fn show(path: &Path) -> std::path::Display {
    path.strip_prefix(env::current_dir().unwrap())
        .unwrap_or(path)
        .display()
}

fn resolve_href(base: &Path, href: &str) -> PathBuf {
    assert!(!href.starts_with('/'));
    assert!(!href.is_empty());
    let mut resolved = PathBuf::from(base);
    resolved.extend(href.split('/'));
    resolved
}
