use self::Command::*;
use self::Origin::*;
use self::State::*;
use std::str;

/// Parse the given string as SVG path data. Returns a value that both:
///
/// * Is an iterator of `Command`
/// * Has a `.error()` method that returns details of the first syntax error, if any so far.
///
/// Syntax errors stop the parser (so that the rest of the string is ignored)
/// and it is desirable to report them to users somehow
/// (such as in the console of a web browser’s developper tools)
/// but they should not prevent the rendering of the path commands before the first error.
///
/// The path may be empty (`Iterator::next` immediately returns `None`).
/// This is not an error, instead it disables rendering of the path.
pub fn parse(s: &str) -> Parser {
    Parser {
        bytes: s.as_bytes(),
        position: 0,
        state: State::ExpectingMoveTo,
        error: None,
    }
}

#[derive(Clone)]
pub struct Parser<'input> {
    bytes: &'input [u8],
    position: usize,
    state: State,
    error: Option<Error>,
}

impl<'input> Parser<'input> {
    /// Returns details of the first syntax error encountered so far.
    ///
    /// Errors should not prevent rendering the commands that are parsed.
    pub fn error(&self) -> Option<Error> {
        self.error
    }
}

#[derive(Copy, Clone, Debug)]
pub struct Error {
    /// Position of the error, in UTF-8 bytes from the start of the string.
    pub position: usize,
    pub reason: &'static str,
}

/// https://www.w3.org/TR/SVG/paths.html#PathData
#[derive(Copy, Clone, Debug)]
pub enum Command {
    MoveTo {
        origin: Origin,
        end: Pair,
    },
    LineTo {
        origin: Origin,
        end: Pair,
    },
    HorizontalLineTo {
        origin: Origin,
        end: f64,
    },
    VerticalLineTo {
        origin: Origin,
        end: f64,
    },
    CurveTo {
        origin: Origin,
        control_start: Pair,
        control_end: Pair,
        end: Pair,
    },
    SmothCurveTo {
        origin: Origin,
        control_end: Pair,
        end: Pair,
    },
    QuadraticBezierCurveTo {
        origin: Origin,
        control: Pair,
        end: Pair,
    },
    SmothQuadraticBezierCurveTo {
        origin: Origin,
        end: Pair,
    },
    EllipticalArc {
        origin: Origin,
        /// Non-negative
        radius: Pair,
        x_axis_rotation: f64,
        large_arc: bool,
        sweep: bool,
        end: Pair,
    },
    ClosePath
}

#[derive(Copy, Clone, Debug)]
pub enum Origin {
    Relative,
    Absolute,
}

#[derive(Copy, Clone, Debug)]
pub struct Pair {
    pub x: f64,
    pub y: f64,
}

#[derive(Copy, Clone, Debug)]
enum State {
    ExpectingMoveTo,
    AfterClosePath,
    AfterMoveTo(Origin),
    AfterLineTo(Origin),
    AfterHorizontalLineTo(Origin),
    AfterVeticalLineTo(Origin),
    AfterCurveTo(Origin),
    AfterSmothCurveTo(Origin),
    AfterQuadraticBezierCurveTo(Origin),
    AfterSmothQuadraticBezierCurveTo(Origin),
    AfterEllipticalArc(Origin),
}

/// https://www.w3.org/TR/SVG/paths.html#PathDataBNF
impl<'input> Iterator for Parser<'input> {
    type Item = Command;

    fn next(&mut self) -> Option<Self::Item> {
        if self.error.is_some() {
            None
        } else {
            match self.parse_next() {
                Ok(next) => next,
                Err(reason) => {
                    self.error = Some(Error {
                        position: self.position,
                        reason: reason,
                    });
                    None
                }
            }
        }
    }
}

type CommandResult = Result<Option<Command>, &'static str>;

impl<'input> Parser<'input> {
    fn parse_next(&mut self) -> CommandResult {
        macro_rules! after {
            ($origin: expr, $parse_arguments: ident) => {
                self.try(|p| {
                    p.consume_comma_whitespace();
                    p.$parse_arguments($origin)
                }).or_else(|_| {
                    self.parse_command()
                })
            }
        }
        match self.state {
            ExpectingMoveTo => {
                self.consume_whitespace();
                let origin = match self.next() {
                    Some('M') => Absolute,
                    Some('m') => Relative,
                    Some(_) => return Err("expected move-to command"),
                    None => return Ok(None),
                };
                self.parse_move_to(origin)
            }
            AfterClosePath => self.parse_command(),
            AfterMoveTo(origin) |
            AfterLineTo(origin) => after!(origin, parse_line_to),
            AfterHorizontalLineTo(origin) => after!(origin, parse_horizontal_line_to),
            AfterVeticalLineTo(origin) => after!(origin, parse_vertical_line_to),
            AfterCurveTo(origin) => after!(origin, parse_curve_to),
            AfterSmothCurveTo(origin) => after!(origin, parse_smooth_curve_to),
            AfterQuadraticBezierCurveTo(origin) => after!(origin, parse_quadratic_bezier_curve_to),
            AfterSmothQuadraticBezierCurveTo(o) => after!(o, parse_smooth_quadratic_bezier_curve_to),
            AfterEllipticalArc(origin) => after!(origin, parse_elliptic_arc),
        }
    }

    fn peek(&self) -> Option<char> {
        // Interpreting individual UTF-8 bytes as code points is only valid in the ASCII range,
        // but everything we’re parsing is in the ASCII range.
        // A non-ASCII byte or code point would be a parse error either way.
        self.bytes.get(self.position).map(|&b| b as char)
    }

    fn next(&mut self) -> Option<char> {
        self.peek().map(|c| {
            self.position += 1;
            c
        })
    }

    fn consume_whitespace(&mut self) {
        while matches!(self.peek(), Some(' ') | Some('\t') | Some('\n') | Some('\r')) {
            self.position += 1
        }
    }

    fn consume_comma_whitespace(&mut self) {
        self.consume_whitespace();
        if self.peek() == Some(',') {
            self.position += 1;
            self.consume_whitespace();
        }
    }

    /// When they return `Err`, the parse_* commands may still have increment `self.position`.
    ///
    /// When parsing alternates, using this to restore the starting position on `Err`.
    fn try<T, F>(&mut self, mut f: F) -> Result<T, &'static str>
    where F: FnMut(&mut Self) -> Result<T, &'static str> {
        let starting_position = self.position;
        let result = f(self);
        if result.is_err() {
            self.position = starting_position;
        }
        result
    }

    fn parse_command(&mut self) -> CommandResult {
        self.consume_whitespace();
        match self.next() {
            Some('Z') | Some('z') => {
                self.state = AfterClosePath;
                Ok(Some(ClosePath))
            }

            Some('M') => self.parse_move_to(Absolute),
            Some('m') => self.parse_move_to(Relative),

            Some('L') => self.parse_line_to(Absolute),
            Some('l') => self.parse_line_to(Relative),

            Some('H') => self.parse_horizontal_line_to(Absolute),
            Some('h') => self.parse_horizontal_line_to(Relative),

            Some('V') => self.parse_vertical_line_to(Absolute),
            Some('v') => self.parse_vertical_line_to(Relative),

            Some('C') => self.parse_curve_to(Absolute),
            Some('c') => self.parse_curve_to(Relative),

            Some('S') => self.parse_smooth_curve_to(Absolute),
            Some('s') => self.parse_smooth_curve_to(Relative),

            Some('Q') => self.parse_quadratic_bezier_curve_to(Absolute),
            Some('q') => self.parse_quadratic_bezier_curve_to(Relative),

            Some('T') => self.parse_smooth_quadratic_bezier_curve_to(Absolute),
            Some('t') => self.parse_smooth_quadratic_bezier_curve_to(Relative),

            Some('A') => self.parse_elliptic_arc(Absolute),
            Some('a') => self.parse_elliptic_arc(Relative),

            Some(_) => Err("expected command"),
            None => Ok(None),
        }
    }

    fn parse_move_to(&mut self, origin: Origin) -> CommandResult {
        self.consume_whitespace();
        let end = try!(self.parse_coordinate_pair());
        self.state = AfterMoveTo(origin);
        Ok(Some(MoveTo {
            origin: origin,
            end: end
        }))
    }

    fn parse_line_to(&mut self, origin: Origin) -> CommandResult {
        self.consume_whitespace();
        let end = try!(self.parse_coordinate_pair());
        self.state = AfterLineTo(origin);
        Ok(Some(LineTo {
            origin: origin,
            end: end,
        }))
    }

    fn parse_horizontal_line_to(&mut self, origin: Origin) -> CommandResult {
        self.consume_whitespace();
        let end = try!(self.parse_number());
        self.state = AfterHorizontalLineTo(origin);
        Ok(Some(HorizontalLineTo {
            origin: origin,
            end: end,
        }))
    }

    fn parse_vertical_line_to(&mut self, origin: Origin) -> CommandResult {
        self.consume_whitespace();
        let end = try!(self.parse_number());
        self.state = AfterVeticalLineTo(origin);
        Ok(Some(VerticalLineTo {
            origin: origin,
            end: end,
        }))
    }

    fn parse_curve_to(&mut self, origin: Origin) -> CommandResult {
        self.consume_whitespace();

        let control_start = try!(self.parse_coordinate_pair());
        self.consume_comma_whitespace();

        let control_end = try!(self.parse_coordinate_pair());
        self.consume_comma_whitespace();

        let end = try!(self.parse_coordinate_pair());
        self.state = AfterCurveTo(origin);
        Ok(Some(CurveTo {
            origin: origin,
            control_start: control_start,
            control_end: control_end,
            end: end,
        }))
    }

    fn parse_smooth_curve_to(&mut self, origin: Origin) -> CommandResult {
        self.consume_whitespace();

        let control_end = try!(self.parse_coordinate_pair());
        self.consume_comma_whitespace();

        let end = try!(self.parse_coordinate_pair());
        self.state = AfterSmothCurveTo(origin);
        Ok(Some(SmothCurveTo {
            origin: origin,
            control_end: control_end,
            end: end,
        }))
    }

    fn parse_quadratic_bezier_curve_to(&mut self, origin: Origin) -> CommandResult {
        self.consume_whitespace();

        let control = try!(self.parse_coordinate_pair());
        self.consume_comma_whitespace();

        let end = try!(self.parse_coordinate_pair());
        self.state = AfterQuadraticBezierCurveTo(origin);
        Ok(Some(QuadraticBezierCurveTo {
            origin: origin,
            control: control,
            end: end,
        }))
    }

    fn parse_smooth_quadratic_bezier_curve_to(&mut self, origin: Origin) -> CommandResult {
        self.consume_whitespace();
        let end = try!(self.parse_coordinate_pair());
        self.state = AfterSmothQuadraticBezierCurveTo(origin);
        Ok(Some(SmothQuadraticBezierCurveTo {
            origin: origin,
            end: end,
        }))
    }

    fn parse_elliptic_arc(&mut self, origin: Origin) -> CommandResult {
        self.consume_whitespace();

        let rx = try!(self.parse_non_negative_number());
        self.consume_comma_whitespace();

        let ry = try!(self.parse_non_negative_number());
        self.consume_comma_whitespace();

        let x_axis_rotation = try!(self.parse_number());
        self.consume_comma_whitespace();

        let large_arc = try!(self.parse_flag());
        self.consume_comma_whitespace();

        let sweep = try!(self.parse_flag());
        self.consume_comma_whitespace();

        let end = try!(self.parse_coordinate_pair());
        self.state = AfterEllipticalArc(origin);
        Ok(Some(EllipticalArc {
            origin: origin,
            radius: Pair {
                x: rx,
                y: ry,
            },
            x_axis_rotation: x_axis_rotation,
            large_arc: large_arc,
            sweep: sweep,
            end: end,
        }))
    }

    fn parse_coordinate_pair(&mut self) -> Result<Pair, &'static str> {
        let x = try!(self.parse_number());
        self.consume_comma_whitespace();

        let y = try!(self.parse_number());
        Ok(Pair {
            x: x,
            y: y,
        })
    }

    fn parse_number(&mut self) -> Result<f64, &'static str> {
        let start_position = self.position;
        self.consume_sign();
        self.parse_number_common(start_position)
    }

    fn parse_non_negative_number(&mut self) -> Result<f64, &'static str> {
        let start_position = self.position;
        self.parse_number_common(start_position)
    }

    fn consume_sign(&mut self) {
        if matches!(self.peek(), Some('+') | Some('-')) {
             self.position += 1
        }
    }

    fn parse_number_common(&mut self, start_position: usize) -> Result<f64, &'static str> {
        let mut any_significand_digit = false;
        while matches!(self.peek(), Some('0'...'9')) {
            self.position += 1;
            any_significand_digit = true;
        }
        if matches!(self.peek(), Some('.')) {
            self.position += 1;
        }
        while matches!(self.peek(), Some('0'...'9')) {
            self.position += 1;
            any_significand_digit = true;
        }
        if matches!(self.peek(), Some('e') | Some('E')) {
            self.position += 1;
            self.consume_sign();
            if !matches!(self.peek(), Some('0'...'9')) {
                return Err("expected digits after exponent sign")
            }
            self.position += 1;
            while matches!(self.peek(), Some('0'...'9')) {
                self.position += 1;
            }
        }
        if !any_significand_digit {
            return Err("expected number")
        }

        // Unwrap here because we’ve only matched ASCII bytes.
        let float_str = str::from_utf8(&self.bytes[start_position..self.position]).unwrap();

        // We’ve found the longest slice of the input
        // that matches the SVG path grammar for 'number' or 'nonnegative-number'.
        // We do that ourselves to know where to stop
        // (str::parse wants its entire input to be a number)
        // and to rejects some inputs like "inf" or "NaN" that are accepted by str::parse
        // but not the grammar.
        //
        // Once we have that, we still use str::parse rather than implement it ourselves
        // because its algortithm is surprisingly complicated,
        // to deal correctly with various corner cases.
        //
        // Unwrap here because any input matched by the grammar should be accepted by str::parse.
        // (It returns Ok(INFINITY) or Ok(NEG_INFINITY) on overflow.)
        Ok(float_str.parse().unwrap())
    }

    fn parse_flag(&mut self) -> Result<bool, &'static str> {
        match self.next() {
            Some('1') => Ok(true),
            Some('0') => Ok(false),
            _ => Err("expected '0' or '1' flag"),
        }
    }
}
