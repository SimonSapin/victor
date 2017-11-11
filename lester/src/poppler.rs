use cairo::*;
use cairo_ffi::{CAIRO_FORMAT_ARGB32, CAIRO_FORMAT_RGB24};
use convert::TryInto;
use errors::{CairoError, GlibError};
use std::ffi::CStr;
use std::marker::PhantomData;
use std::ops::Range;
use std::os::raw::*;
use std::ptr;
use std::str::Utf8Error;
use poppler_ffi::*;

/// A PDF document parsed by Poppler.
pub struct PdfDocument<'data> {
    ptr: *mut PopplerDocument,
    phantom: PhantomData<&'data [u8]>,
}

impl<'data> PdfDocument<'data> {
    /// Parse the given bytes as PDF.
    pub fn from_bytes(mut bytes: &'data [u8]) -> Result<Self, GlibError> {
        // Work around https://bugs.freedesktop.org/show_bug.cgi?id=103552
        if bytes.is_empty() {
            bytes = b"";
        }

        let mut error = ptr::null_mut();
        let ptr = unsafe {
            poppler_document_new_from_data(
                // Although this function takes *mut c_char rather than *const c_char,
                // that pointer is only passed to Poppler’s `MemStream` abstraction
                // which appears to only provide read access.
                bytes.as_ptr() as *const c_char as *mut c_char,
                bytes.len().try_into().unwrap(),
                ptr::null(),
                &mut error
            )
        };
        if ptr.is_null() {
            Err(GlibError { ptr: error })
        } else {
            Ok(PdfDocument { ptr, phantom: PhantomData })
        }
    }

    /// Make an iterator of the pages in this document.
    ///
    /// The page count can be obtained with `.pages().len()`,
    /// and an arbitrary page with `.pages().nth(index)`.
    pub fn pages<'doc>(&'doc self) -> PagesIter<'doc, 'data> {
        let page_count = unsafe {
            poppler_document_get_n_pages(self.ptr)
        };
        PagesIter {
            doc: self,
            range: 0..page_count
        }
    }

    fn get_page(&self, index: c_int) -> Page<'data> {
        let ptr = unsafe {
            poppler_document_get_page(self.ptr, index)
        };
        assert!(!ptr.is_null());
        Page { ptr, phantom: PhantomData }
    }

    /// Return the `Producer` entry of the document’s *information dictionary*.
    pub fn producer(&self) -> Option<GlibString> {
        unsafe {
            GlibString::from_nullable_ptr(poppler_document_get_producer(self.ptr))
        }
    }
}

impl<'data> Drop for PdfDocument<'data> {
    fn drop(&mut self) {
        unsafe {
            g_object_unref(self.ptr as *mut c_void)
        }
    }
}

/// Double-ended exact-size iterator for the pages in a given `PdfDocument`.
pub struct PagesIter<'doc, 'data: 'doc> {
    doc: &'doc PdfDocument<'data>,
    range: Range<c_int>,
}

impl<'doc, 'data> Iterator for PagesIter<'doc, 'data> {
    type Item = Page<'data>;

    fn next(&mut self) -> Option<Self::Item> {
        self.range.next().map(|index| self.doc.get_page(index))
    }

    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        self.range.nth(n).map(|index| self.doc.get_page(index))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.range.len();
        (len, Some(len))
    }
}

impl<'doc, 'data> DoubleEndedIterator for PagesIter<'doc, 'data> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.range.next_back().map(|index| self.doc.get_page(index))
    }
}

impl<'doc, 'data> ExactSizeIterator for PagesIter<'doc, 'data> {
    fn len(&self) -> usize {
        self.range.len()
    }
}

/// A page from a `PdfDocument`.
pub struct Page<'data> {
    ptr: *mut PopplerPage,
    phantom: PhantomData<&'data [u8]>,
}

impl<'data> Page<'data> {
    /// The width and height of this page, in PostScript points as stored in PDF.
    ///
    /// One PostScript point is ¹⁄₇₂ inch,
    /// or 0.352<span style="text-decoration: overline">7</span> mm.
    /// It is the base length unit of the PDF file format.
    pub fn size_in_ps_points(&self) -> (f64, f64) {
        let mut width = 0.;
        let mut height = 0.;
        unsafe {
            poppler_page_get_size(self.ptr, &mut width, &mut height)
        }
        (width, height)
    }

    /// The width and height of this page, converted to CSS `px` units.
    ///
    /// This assumes that the CSS `pt` unit is mapped to one PostScript point, as Victor does.
    /// (This mapping also makes CSS `in` and `mm` map to physical inches and millimeters.)
    pub fn size_in_css_px(&self) -> (f64, f64) {
        let (w, h) = self.size_in_ps_points();
        (w * PX_PER_PT,
         h * PX_PER_PT)
    }

    /// Render (rasterize) this page with the default options to a new image surface.
    pub fn render(&self) -> Result<ImageSurface, CairoError> {
        self.render_with_options(RenderOptions::default())
    }

    /// Render (rasterize) this page with the given zoom/scale level to a new image surface.
    ///
    /// The parameter is the number of rendered pixels per CSS `px` unit,
    /// assuming that the CSS `pt` unit maps to PostScript points as Victor does.
    ///
    /// The default is `1.0`, which equals `96dpi`.
    /// A value of `2.0` produces a rendering similar to a “retina” double-density display.
    /// `3.125` equals `300dpi`.
    pub fn render_with_dppx(&self, dppx: f64) -> Result<ImageSurface, CairoError> {
        self.render_with_options(RenderOptions {
            dppx_x: dppx,
            dppx_y: dppx,
            ..RenderOptions::default()
        })
    }

    /// Render (rasterize) this page with the given options to a new image surface.
    pub fn render_with_options(&self, options: RenderOptions) -> Result<ImageSurface, CairoError> {
        let RenderOptions { dppx_x, dppx_y, antialias, backdrop, for_printing } = options;
        let (width, height) = self.size_in_css_px();
        let mut surface = ImageSurface::new_c_int(
            match backdrop {
                Backdrop::Transparent => CAIRO_FORMAT_ARGB32,
                Backdrop::White => CAIRO_FORMAT_RGB24,
            },
            (width * dppx_x).ceil().try_into().unwrap(),
            (height * dppx_y).ceil().try_into().unwrap(),
        )?;
        let mut context = surface.context()?;
        if let Backdrop::White = backdrop {
            context.set_source_rgb(1., 1., 1.);
            context.paint();
        }
        context.scale(dppx_x * PX_PER_PT,
                      dppx_y * PX_PER_PT);
        context.set_antialias(antialias);
        unsafe {
            if for_printing {
                poppler_page_render_for_printing(self.ptr, context.ptr)
            } else {
                poppler_page_render(self.ptr, context.ptr)
            }
        }
        context.check_status()?;
        Ok(surface)
    }

    /// Return the text on this page
    pub fn text(&self) -> GlibString {
        unsafe {
            GlibString::from_nullable_ptr(poppler_page_get_text(self.ptr))
            .expect("poppler_page_get_text returned a NULL pointer")
        }
    }
}

impl<'data> Drop for Page<'data> {
    fn drop(&mut self) {
        unsafe {
            g_object_unref(self.ptr as *mut c_void)
        }
    }
}

const PT_PER_INCH: f64 = 72.;
const PX_PER_INCH: f64 = 96.;
const PX_PER_PT: f64 = PX_PER_INCH / PT_PER_INCH;

/// Parameters for rendering (rasterization)
///
/// This type implements the `Default` trait.
/// To only change some fields from the default, it can be constructed as:
///
/// ```rust
/// # use lester::RenderOptions;
/// let options = RenderOptions {
///     for_printing: true,
///     ..RenderOptions::default()
/// };
/// ```
#[derive(Copy, Clone, Debug)]
pub struct RenderOptions {
    /// The number of rendered pixels per CSS `px` unit in the horizontal direction,
    /// assuming that the CSS `pt` unit maps to PostScript points as Victor does.
    ///
    /// The default is `1.0`, which equals `96dpi`.
    /// A value of `2.0` produces a rendering similar to a “retina” double-density display.
    /// `3.125` equals `300dpi`.
    pub dppx_x: f64,

    /// The number of rendered pixels per CSS `px` unit in the vertical direction.
    /// Typically this is the same as `dppx_x`.
    pub dppx_y: f64,

    /// The antialiasing mode to use for rasterizing text and vector graphics.
    pub antialias: Antialias,

    /// What background to render pages on
    pub backdrop: Backdrop,

    /// Whether to use `poppler_page_render_for_printing` instead of `poppler_page_render`.
    /// What that does excactly doesn’t seem well-documented.
    pub for_printing: bool,
}

impl Default for RenderOptions {
    fn default() -> Self {
        RenderOptions {
            dppx_x: 1.0,
            dppx_y: 1.0,
            antialias: Antialias::Default,
            backdrop: Backdrop::Transparent,
            for_printing: false,
        }
    }
}

/// What background to render pages on
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Backdrop {
    /// Solid white page background in a RGB24 image
    White,
    /// Transparent page background in a ARGB32 image
    Transparent,
}

/// A string allocated by `glib`
pub struct GlibString {
    ptr: *mut gchar,
}

impl GlibString {
    fn from_nullable_ptr(ptr: *mut gchar) -> Option<Self> {
        if ptr.is_null() {
            None
        } else {
            Some(GlibString { ptr })
        }
    }

    pub fn to_str(&self) -> Result<&str, Utf8Error> {
        let cstr = unsafe {
            CStr::from_ptr(self.ptr)
        };
        cstr.to_str()
    }
}

impl Drop for GlibString {
    fn drop(&mut self) {
        unsafe {
            g_free(self.ptr as *mut c_void);
        }
    }
}
