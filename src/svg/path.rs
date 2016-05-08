use self::Command::*;
use self::Origin::*;
use self::State::*;
use std::str;
use svg::geometry::Pair;

#[path = "simple_path.rs"]
mod simple_path;
pub use self::simple_path::{SimpleCommand, Simplify, simplify};

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
        state: State::ExpectingMove,
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

    pub fn simplify(self) -> Simplify<Self> {
        simplify(self)
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
    Move {
        origin: Origin,
        to: Pair,
    },
    Line {
        origin: Origin,
        to: Pair,
    },
    HorizontalLine {
        origin: Origin,
        to: f64,
    },
    VerticalLine {
        origin: Origin,
        to: f64,
    },
    Curve {
        origin: Origin,
        control_1: Pair,
        control_2: Pair,
        to: Pair,
    },
    SmothCurve {
        origin: Origin,
        control_2: Pair,
        to: Pair,
    },
    QuadraticBezierCurve {
        origin: Origin,
        control: Pair,
        to: Pair,
    },
    SmothQuadraticBezierCurve {
        origin: Origin,
        to: Pair,
    },
    EllipticalArc(Origin, EllipticalArcCommand),
    ClosePath
}

#[derive(Copy, Clone, Debug)]
pub struct EllipticalArcCommand {
    /// Non-negative
    pub radius: Pair,
    pub x_axis_rotation: f64,
    pub large_arc: bool,
    pub sweep: bool,
    pub to: Pair,
}

#[derive(Copy, Clone, Debug)]
pub enum Origin {
    Relative,
    Absolute,
}


#[derive(Copy, Clone, Debug)]
enum State {
    ExpectingMove,
    AfterClosePath,
    AfterMove(Origin),
    AfterLine(Origin),
    AfterHorizontalLine(Origin),
    AfterVeticalLine(Origin),
    AfterCurve(Origin),
    AfterSmothCurve(Origin),
    AfterQuadraticBezierCurve(Origin),
    AfterSmothQuadraticBezierCurve(Origin),
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
            ExpectingMove => {
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
            AfterMove(origin) |
            AfterLine(origin) => after!(origin, parse_line_to),
            AfterHorizontalLine(origin) => after!(origin, parse_horizontal_line_to),
            AfterVeticalLine(origin) => after!(origin, parse_vertical_line_to),
            AfterCurve(origin) => after!(origin, parse_curve_to),
            AfterSmothCurve(origin) => after!(origin, parse_smooth_curve_to),
            AfterQuadraticBezierCurve(origin) => after!(origin, parse_quadratic_bezier_curve_to),
            AfterSmothQuadraticBezierCurve(o) => after!(o, parse_smooth_quadratic_bezier_curve_to),
            AfterEllipticalArc(origin) => after!(origin, parse_elliptical_arc),
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

            Some('A') => self.parse_elliptical_arc(Absolute),
            Some('a') => self.parse_elliptical_arc(Relative),

            Some(_) => Err("expected command"),
            None => Ok(None),
        }
    }

    fn parse_move_to(&mut self, origin: Origin) -> CommandResult {
        self.consume_whitespace();
        let to = try!(self.parse_coordinate_pair());
        self.state = AfterMove(origin);
        Ok(Some(Move {
            origin: origin,
            to: to
        }))
    }

    fn parse_line_to(&mut self, origin: Origin) -> CommandResult {
        self.consume_whitespace();
        let to = try!(self.parse_coordinate_pair());
        self.state = AfterLine(origin);
        Ok(Some(Line {
            origin: origin,
            to: to,
        }))
    }

    fn parse_horizontal_line_to(&mut self, origin: Origin) -> CommandResult {
        self.consume_whitespace();
        let to = try!(self.parse_number());
        self.state = AfterHorizontalLine(origin);
        Ok(Some(HorizontalLine {
            origin: origin,
            to: to,
        }))
    }

    fn parse_vertical_line_to(&mut self, origin: Origin) -> CommandResult {
        self.consume_whitespace();
        let to = try!(self.parse_number());
        self.state = AfterVeticalLine(origin);
        Ok(Some(VerticalLine {
            origin: origin,
            to: to,
        }))
    }

    fn parse_curve_to(&mut self, origin: Origin) -> CommandResult {
        self.consume_whitespace();

        let control_1 = try!(self.parse_coordinate_pair());
        self.consume_comma_whitespace();

        let control_2 = try!(self.parse_coordinate_pair());
        self.consume_comma_whitespace();

        let to = try!(self.parse_coordinate_pair());
        self.state = AfterCurve(origin);
        Ok(Some(Curve {
            origin: origin,
            control_1: control_1,
            control_2: control_2,
            to: to,
        }))
    }

    fn parse_smooth_curve_to(&mut self, origin: Origin) -> CommandResult {
        self.consume_whitespace();

        let control_2 = try!(self.parse_coordinate_pair());
        self.consume_comma_whitespace();

        let to = try!(self.parse_coordinate_pair());
        self.state = AfterSmothCurve(origin);
        Ok(Some(SmothCurve {
            origin: origin,
            control_2: control_2,
            to: to,
        }))
    }

    fn parse_quadratic_bezier_curve_to(&mut self, origin: Origin) -> CommandResult {
        self.consume_whitespace();

        let control = try!(self.parse_coordinate_pair());
        self.consume_comma_whitespace();

        let to = try!(self.parse_coordinate_pair());
        self.state = AfterQuadraticBezierCurve(origin);
        Ok(Some(QuadraticBezierCurve {
            origin: origin,
            control: control,
            to: to,
        }))
    }

    fn parse_smooth_quadratic_bezier_curve_to(&mut self, origin: Origin) -> CommandResult {
        self.consume_whitespace();
        let to = try!(self.parse_coordinate_pair());
        self.state = AfterSmothQuadraticBezierCurve(origin);
        Ok(Some(SmothQuadraticBezierCurve {
            origin: origin,
            to: to,
        }))
    }

    fn parse_elliptical_arc(&mut self, origin: Origin) -> CommandResult {
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

        let to = try!(self.parse_coordinate_pair());
        self.state = AfterEllipticalArc(origin);
        Ok(Some(EllipticalArc(origin, EllipticalArcCommand {
            radius: Pair {
                x: rx,
                y: ry,
            },
            x_axis_rotation: x_axis_rotation,
            large_arc: large_arc,
            sweep: sweep,
            to: to,
        })))
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
