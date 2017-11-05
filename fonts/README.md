# Fonts in Victor

Victor does not plan to support loading fonts installed on the system.
This is both to avoid the associated complexity across platforms,
and to keep the rendering of a given document reproducible when made in different environments.

Instead, it is intended that users provide fonts via a Rust API or CSS `@font-face` rules.
Still, in order to be able to render *some* text out of the box,
Victor will come with one fallback font built-in.
Although Latin-centrism is unfortunate,
this font doesn’t need wide Unicode coverage
since in “normal use” it is intended to be replaced.
On the other hand a small font limits the bloat in library binary size.


## Bitstream Vera

[Vera] is a font family published in 2003.
It was the default in GNOME before version 3.0.
It is the ancestor of [DejaVu]
and has much more limited Unicode coverage (not much basic Latin),
and thus smaller file sizes.
It is distributed under a custom license (see the `ttf-bitstream-vera-1.10/COPYRIGHT.TXT` file).
Files obtained from from [ftp.gnome.org].

[Vera]: https://www.gnome.org/fonts/
[DejaVu]: https://dejavu-fonts.github.io/
[ftp.gnome.org]: http://ftp.gnome.org/pub/GNOME/sources/ttf-bitstream-vera/1.10/


## Ahem

[Ahem] is a testing font where most glyphs are plain squares.
It is distributed under CC0 (see the `Ahem-2017.01.31/COPYING` file).
Files obtained from [www.w3.org] on 2017-11-05 with “Last modified 2017-01-31”.
(Ahem has had revisions over time but doesn’t seem to have anything like a version number.)

[Ahem]: http://web-platform-tests.org/writing-tests/ahem.html
[www.w3.org]: https://www.w3.org/Style/CSS/Test/Fonts/Ahem/
