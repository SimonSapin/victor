use cairo_ffi::*;
use cairo::*;
use errors::{CairoError, GlibError};
use std::marker::PhantomData;
use std::ops::Range;
use std::os::raw::*;
use poppler_ffi::*;

/// A PDF document parsed by Poppler.
pub struct PdfDocument<'data> {
    ptr: *mut PopplerDocument,
    phantom: PhantomData<&'data [u8]>,
}

impl<'data> PdfDocument<'data> {
    /// Parse the given bytes as PDF.
    pub fn from_bytes(bytes: &'data [u8]) -> Result<Self, GlibError> {
        let mut error = 0 as *mut GError;
        let ptr = unsafe {
            poppler_document_new_from_data(
                // Although this function takes *mut c_char rather than *const c_char,
                // that pointer is only passed to Poppler’s `MemStream` abstraction
                // which appears to only provide read access.
                bytes.as_ptr() as *const c_char as *mut c_char,
                bytes.len() as c_int,
                0 as *const c_char,
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
    /// The width and height of this page, in PostScript points.
    ///
    /// One PostScript point is ¹⁄₁₂ inch,
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

    /// Render (rasterize) this page to the given pixel buffer, with the given options.
    pub fn render(&self, surface: &mut ImageSurface, options: RenderOptions) -> Result<(), CairoError> {
        let RenderOptions { dpi_x, dpi_y, antialias, for_printing } = options;
        // PDF’s default unit is the PostScript point, wich is 1/72 inches.
        let scale_x = dpi_x / 72.;
        let scale_y = dpi_y / 72.;
        let context = surface.context()?;
        unsafe {
            cairo_scale(context.ptr, scale_x, scale_y);
            cairo_set_antialias(context.ptr, antialias.to_cairo());
            if for_printing {
                poppler_page_render_for_printing(self.ptr, context.ptr)
            } else {
                poppler_page_render(self.ptr, context.ptr)
            }
            cairo_surface_flush(surface.ptr);
        }
        context.check_status()?;
        Ok(())
    }
}

impl<'data> Drop for Page<'data> {
    fn drop(&mut self) {
        unsafe {
            g_object_unref(self.ptr as *mut c_void)
        }
    }
}

/// Parameters for rendering (rasterization)
///
/// This type implements the `Default` trait.
/// To only change some fields from the default, it can be constructed as:
///
/// ```rust
/// let options = RenderOptions {
///     for_printing: true,
///     ..RenderOptions::default
/// };
/// ```
#[derive(Copy, Clone, Debug)]
pub struct RenderOptions {
    /// The number of pixels per inch in the horizontal direction,
    /// where one inch is 72 PostScript points.
    ///
    /// Since Victor generates PDF such as a `1pt` CSS length is one PostScript point,
    /// a DPI of 96 will make a `1px` CSS length is one rendered pixel.
    /// A DPI of 192 is similar to a “retina” double-density display.
    pub dpi_x: f64,

    /// The number of pixels per inch in the vertical direction.
    /// Typically this is the same as `dpi_x`.
    pub dpi_y: f64,

    /// The antialiasing mode to use for rasterizing text and vector graphics.
    pub antialias: Antialias,

    /// Whether to use `poppler_page_render_for_printing` instead of `poppler_page_render`.
    /// What that does excactly doesn’t seem well-documented.
    pub for_printing: bool,
}

impl Default for RenderOptions {
    fn default() -> Self {
        RenderOptions {
            // Default to CSS '1px' == 1 raster pixel,
            // assuming CSS '1pt' == 1 PostScript point.
            dpi_x: 96.,
            dpi_y: 96.,
            antialias: Antialias::Default,
            for_printing: false,
        }
    }
}
