use self::Command::*;
use self::State::*;
use std::str;

pub fn parse(s: &str) -> Parser {
    Parser {
        input: s.chars(),
        state: State::ExpectingMoveTo,
        error_at: None,
    }
}

pub struct Parser<'a> {
    input: Cursor<'a>,
    state: State,
    error_at: Option<usize>,
}

impl<'a> Parser<'a> {
    pub fn error_at(&self) -> Option<usize> {
        self.error_at
    }
}

/// https://www.w3.org/TR/SVG/paths.html#PathData
#[derive(Debug)]
pub enum Command {
    MoveTo(Coordinates, Pair),
    LineTo(Coordinates, Pair),
    HorizontalLineTo(Coordinates, f64),
    VeticalLineTo(Coordinates, f64),
    CurveTo {
        coordinates: Coordinates,
        control_start: Pair,
        control_end: Pair,
        end: Pair
    },
    SmothCurveTo {
        coordinates: Coordinates,
        control_end: Pair,
        end: Pair
    },
    QuadraticBezierCurveTo {
        coordinates: Coordinates,
        control: Pair,
        end: Pair
    },
    SmothQuadraticBezierCurveTo {
        coordinates: Coordinates,
        end: Pair
    },
    EllipticalArc {
        coordinates: Coordinates,
        /// Non-negative
        radius: Pair,
        x_axis_rotation: f64,
        large_arc: bool,
        sweep: bool,
        end: Pair
    },
    ClosePath,
}

#[derive(Debug)]
pub enum Coordinates {
    Relative,
    Absolute,
}

#[derive(Debug)]
pub struct Pair {
    pub x: f64,
    pub y: f64,
}

enum State {
    ExpectingMoveTo,
    AfterClosePath,
    InLineTo,  // Or implicit LineTo after MoveTo
    InHorizontalLineTo,
    InVeticalLineTo,
    InCurveTo,
    InSmothCurveTo,
    InQuadraticBezierCurveTo,
    InSmothQuadraticBezierCurveTo,
    InEllipticalArc,
}

/// https://www.w3.org/TR/SVG/paths.html#PathDataBNF
impl<'a> Iterator for Parser<'a> {
    type Item = Command;

    fn next(&mut self) -> Option<Self::Item> {
        if self.error_at.is_some() {
            return None
        }
        unimplemented!()
    }
}
