use std::fmt;
use svg::geometry::Pair;
use svg::path::{Command, Origin, Parser, Error};
use svg::path::Command::*;
use svg::path::Origin::*;

/// Where as `Command` keeps data close to how it is in the source SVG path,
/// this simplifies it by making everything absolute,
/// converting vertical and horizontal line-to to plain line-to,
/// and converting every smooth and quadratic curves to plain (cubic) curve-to.
///
/// This conversion is exact.
/// It does not include converting elliptical arcs to bezier curves,
/// which could only be an approximation.
#[derive(Copy, Clone)]
pub enum SimpleCommand {
    Move {
        to: Pair
    },
    Line {
        to: Pair
    },
    Curve {
        control_1: Pair,
        control_2: Pair,
        to: Pair,
    },
    EllipticalArc {
        /// Non-negative
        radius: Pair,
        x_axis_rotation: f64,
        large_arc: bool,
        sweep: bool,
        to: Pair,
    },
    ClosePath
}

impl fmt::Debug for SimpleCommand {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            SimpleCommand::Move { to } => write!(formatter, "Move {{ to: {:?} }}", to),
            SimpleCommand::Line { to } => write!(formatter, "Line {{ to: {:?} }}", to),
            SimpleCommand::Curve { control_1: c1, control_2: c2, to } => {
                write!(formatter, "Curve {{ c1: {:?}, c2: {:?}, to: {:?} }}", c1, c2, to)
            }
            SimpleCommand::EllipticalArc { radius, x_axis_rotation, large_arc, sweep, to } => {
                fn flag(b: bool) -> &'static str {
                    if b { "✓" } else { "✗" }
                }
                write!(formatter, "EllipticalArc {{ radius: {:?}, x_axis_rotation: {}, \
                                                    large_arc: {}, sweep: {}, to: {:?} }}",
                       radius, x_axis_rotation, flag(large_arc), flag(sweep), to)
            }
            SimpleCommand::ClosePath => write!(formatter, "ClosePath")
        }
    }
}

pub fn simplify<I: Iterator<Item=Command>>(mut iter: I) -> Simplify<I> {
    match iter.next() {
        Some(Move { origin: _, to: first_point }) => {
            Simplify {
                iter: iter,
                current_point: first_point,
                subpath_start_point: first_point,
                previous_cubic_control_2: first_point,
                previous_quadratic_control: first_point,
            }
        },
        Some(command) => panic!("path starts with {:?} instead of move-to command", command),
        None => {
            // `dummy` is not gonna be used (unless `iter.next()` starts returning `Some(_)` again)
            let dummy = Pair { x: 0., y: 0. };
            Simplify {
                iter: iter,
                current_point: dummy,
                subpath_start_point: dummy,
                previous_cubic_control_2: dummy,
                previous_quadratic_control: dummy,
            }
        }
    }
}

pub struct Simplify<I: Iterator<Item=Command>> {
    iter: I,
    current_point: Pair,
    subpath_start_point: Pair,  // The point we go back to with ClosePath
    previous_cubic_control_2: Pair,
    previous_quadratic_control: Pair,
}

impl<'input> Simplify<Parser<'input>> {
    pub fn error(&self) -> Option<Error> {
        self.iter.error
    }
}

impl<I: Iterator<Item=Command>> Simplify<I> {
    fn to_absolute(&self, point: Pair, origin: Origin) -> Pair {
        match origin {
            Absolute => point,
            Relative => self.current_point + point,
        }
    }
}

impl Pair {
    fn reflect(&self, center: &Pair) -> Pair {
        // self + 2 * (center - self) == center * 2 - self
        center * 2. - *self
    }
}

impl<I: Iterator<Item=Command>> Iterator for Simplify<I> {
    type Item = SimpleCommand;

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|next| match next {
            Move { origin, to } => {
                let to = self.to_absolute(to, origin);
                self.subpath_start_point = to;
                self.current_point = to;
                self.previous_cubic_control_2 = to;
                self.previous_quadratic_control = to;
                SimpleCommand::Move { to: to }
            }
            Line { origin, to } => {
                let to = self.to_absolute(to, origin);
                self.current_point = to;
                self.previous_cubic_control_2 = to;
                self.previous_quadratic_control = to;
                SimpleCommand::Line { to: to }
            }
            HorizontalLine { origin, to } => {
                let to = match origin {
                    Absolute => to,
                    Relative => to + self.current_point.x
                };
                let to_point = Pair { x: to, y: self.current_point.y };
                self.current_point = to_point;
                self.previous_cubic_control_2 = to_point;
                self.previous_quadratic_control = to_point;
                SimpleCommand::Line { to: to_point }
            }
            VerticalLine { origin, to } => {
                let to = match origin {
                    Absolute => to,
                    Relative => to + self.current_point.y
                };
                let to_point = Pair { x: self.current_point.x, y: to };
                self.current_point = to_point;
                self.previous_cubic_control_2 = to_point;
                self.previous_quadratic_control = to_point;
                SimpleCommand::Line { to: to_point }
            }
            Curve { origin, control_1, control_2, to } => {
                let control_1 = self.to_absolute(control_1, origin);
                let control_2 = self.to_absolute(control_2, origin);
                let to = self.to_absolute(to, origin);
                self.current_point = to;
                self.previous_cubic_control_2 = control_2;
                self.previous_quadratic_control = to;
                SimpleCommand::Curve {
                    control_1: control_1,
                    control_2: control_2,
                    to: to,
                }
            }
            SmothCurve { origin, control_2, to } => {
                // https://www.w3.org/TR/SVG/paths.html#PathDataCubicBezierCommands
                //
                // > The first control point is assumed to be the reflection
                // > of the second control point on the previous command
                // > relative to the current point.
                // > (If there is no previous command
                // > or if the previous command was not an C, c, S or s,
                // > assume the first control point is coincident with the current point.)
                let control_1 = self.previous_cubic_control_2.reflect(&self.current_point);


                let control_2 = self.to_absolute(control_2, origin);
                let to = self.to_absolute(to, origin);
                self.current_point = to;
                self.previous_cubic_control_2 = control_2;
                self.previous_quadratic_control = to;
                SimpleCommand::Curve {
                    control_1: control_1,
                    control_2: control_2,
                    to: to,
                }
            }
            QuadraticBezierCurve { origin, control, to } => {
                let control = self.to_absolute(control, origin);

                // https://fontforge.github.io/bezier.html#ttf2ps
                //
                // > Any quadratic spline can be expressed as a cubic
                // > (where the cubic term is zero).
                // > The end points of the cubic will be the same as the quadratic's.
                // >
                // >    CP0 = QP0
                // >    CP3 = QP2
                // >
                // > The two control points for the cubic are:
                // >
                // >    CP1 = QP0 + 2/3 *(QP1-QP0)
                // >    CP2 = QP2 + 2/3 *(QP1-QP2)
                let control_1 = self.current_point + (control - self.current_point) * (2./3.);
                let control_2 = to + (control - to) * (2./3.);

                let to = self.to_absolute(to, origin);
                self.current_point = to;
                self.previous_cubic_control_2 = to;
                self.previous_quadratic_control = control;
                SimpleCommand::Curve {
                    control_1: control_1,
                    control_2: control_2,
                    to: to,
                }
            }
            SmothQuadraticBezierCurve { origin, to } => {
                // https://www.w3.org/TR/SVG/paths.html#PathDataQuadraticBezierCommands
                //
                // > The control point is assumed to be the reflection
                // > of the control point on the previous command relative to the current point.
                // > (If there is no previous command
                // > or if the previous command was not a Q, q, T or t,
                // > assume the control point is coincident with the current point.)
                let control = self.previous_quadratic_control.reflect(&self.current_point);

                let control_1 = self.current_point + (control - self.current_point) * (2./3.);
                let control_2 = to + (control - to) * (2./3.);

                let to = self.to_absolute(to, origin);
                self.current_point = to;
                self.previous_cubic_control_2 = to;
                self.previous_quadratic_control = control;
                SimpleCommand::Curve {
                    control_1: control_1,
                    control_2: control_2,
                    to: to,
                }
            }
            EllipticalArc { origin, radius, x_axis_rotation, large_arc, sweep, to } => {
                let to = self.to_absolute(to, origin);
                self.current_point = to;
                self.previous_cubic_control_2 = to;
                self.previous_quadratic_control = to;
                SimpleCommand::EllipticalArc {
                    radius: radius,
                    x_axis_rotation: x_axis_rotation,
                    large_arc: large_arc,
                    sweep: sweep,
                    to: to,
                }
            }
            ClosePath => {
                self.current_point = self.subpath_start_point;
                self.previous_cubic_control_2 = self.current_point;
                self.previous_quadratic_control = self.current_point;
                SimpleCommand::ClosePath
            }
        })
    }
}
