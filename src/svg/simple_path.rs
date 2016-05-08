use std::fmt;
use svg::geometry::Pair;
use svg::path::{Command, EllipticalArcCommand, Origin, Parser, Error};
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
    EllipticalArc(EllipticalArcCommand),
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
            SimpleCommand::EllipticalArc(EllipticalArcCommand {
                radius, x_axis_rotation, large_arc, sweep, to
            }) => {
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

pub fn simplify<I: Iterator<Item=Command>>(iter: I) -> Simplify<I> {
    let dummy = Pair { x: 0., y: 0. };
    Simplify {
        iter: iter,
        current_point: None,
        subpath_start_point: dummy,
        previous_cubic_control_2: dummy,
        previous_quadratic_control: dummy,
    }
}

pub struct Simplify<I: Iterator<Item=Command>> {
    iter: I,
    current_point: Option<Pair>,
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
    fn current_point(&self) -> Pair {
        self.current_point.expect("drawing command before any move-to command")
    }

    fn to_absolute(&self, point: Pair, origin: Origin) -> Pair {
        match origin {
            Absolute => point,
            Relative => self.current_point() + point,
        }
    }
}

impl Pair {
    fn reflect(self, center: Pair) -> Pair {
        // self + 2 * (center - self) == center * 2 - self
        center * 2. - self
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
                self.current_point = Some(to);
                self.previous_cubic_control_2 = to;
                self.previous_quadratic_control = to;
                SimpleCommand::Move { to: to }
            }
            Line { origin, to } => {
                let to = self.to_absolute(to, origin);
                self.current_point = Some(to);
                self.previous_cubic_control_2 = to;
                self.previous_quadratic_control = to;
                SimpleCommand::Line { to: to }
            }
            HorizontalLine { origin, to } => {
                let to = match origin {
                    Absolute => to,
                    Relative => to + self.current_point().x
                };
                let to_point = Pair { x: to, y: self.current_point().y };
                self.current_point = Some(to_point);
                self.previous_cubic_control_2 = to_point;
                self.previous_quadratic_control = to_point;
                SimpleCommand::Line { to: to_point }
            }
            VerticalLine { origin, to } => {
                let to = match origin {
                    Absolute => to,
                    Relative => to + self.current_point().y
                };
                let to_point = Pair { x: self.current_point().x, y: to };
                self.current_point = Some(to_point);
                self.previous_cubic_control_2 = to_point;
                self.previous_quadratic_control = to_point;
                SimpleCommand::Line { to: to_point }
            }
            Curve { origin, control_1, control_2, to } => {
                let control_1 = self.to_absolute(control_1, origin);
                let control_2 = self.to_absolute(control_2, origin);
                let to = self.to_absolute(to, origin);
                self.current_point = Some(to);
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
                let control_1 = self.previous_cubic_control_2.reflect(self.current_point());


                let control_2 = self.to_absolute(control_2, origin);
                let to = self.to_absolute(to, origin);
                self.current_point = Some(to);
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
                let control_1 = self.current_point() * (1./3.) - control * (2./3.);
                let control_2 = to * (1./3.) - control * (2./3.);

                let to = self.to_absolute(to, origin);
                self.current_point = Some(to);
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
                let control = self.previous_quadratic_control.reflect(self.current_point());

                let control_1 = self.current_point() * (1./3.) - control * (2./3.);
                let control_2 = to * (1./3.) - control * (2./3.);

                let to = self.to_absolute(to, origin);
                self.current_point = Some(to);
                self.previous_cubic_control_2 = to;
                self.previous_quadratic_control = control;
                SimpleCommand::Curve {
                    control_1: control_1,
                    control_2: control_2,
                    to: to,
                }
            }
            EllipticalArc(origin, mut arc) => {
                arc.to = self.to_absolute(arc.to, origin);
                self.current_point = Some(arc.to);
                self.previous_cubic_control_2 = arc.to;
                self.previous_quadratic_control = arc.to;
                SimpleCommand::EllipticalArc(arc)
            }
            ClosePath => {
                self.current_point = Some(self.subpath_start_point);
                self.previous_cubic_control_2 = self.subpath_start_point;
                self.previous_quadratic_control = self.subpath_start_point;
                SimpleCommand::ClosePath
            }
        })
    }
}
