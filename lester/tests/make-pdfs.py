#!/bin/env python2

import cairo
import os.path

this = os.path.dirname(__file__)

def millimeters_to_poscript_points(mm):
    inches = mm / 25.4
    return inches * 72

def css_px_to_poscript_points(px):
    inches = px / 96.
    return inches * 72

cairo.PDFSurface(os.path.join(this, "A4_one_empty_page.pdf"),
                 millimeters_to_poscript_points(210),
                 millimeters_to_poscript_points(297))

pattern = cairo.ImageSurface.create_from_png(os.path.join(this, "pattern_4x4.png"))
out = cairo.PDFSurface(os.path.join(this, "pattern_4x4.pdf"),
                       css_px_to_poscript_points(4),
                       css_px_to_poscript_points(4))
ctx = cairo.Context(out)
ctx.set_source(cairo.SurfacePattern(pattern))
ctx.paint()
