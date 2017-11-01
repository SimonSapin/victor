#!/bin/env python2

import cairo
import os.path

this = os.path.dirname(__file__)

def millimeters_to_poscript_points(mm):
    inches = mm / 25.4
    return inches * 72

cairo.PDFSurface(os.path.join(this, "A4_one_empty_page.pdf"),
                 millimeters_to_poscript_points(210),
                 millimeters_to_poscript_points(297))
