use std::fmt::Write;

#[derive(Clone, Debug)]
pub enum Command {
    MoveTo {
        x: f64,
        y: f64,
    },
    LineTo {
        x: f64,
        y: f64,
    },
    CubicTo {
        x1: f64,
        y1: f64,
        x2: f64,
        y2: f64,
        x: f64,
        y: f64,
    },
    QuadraticTo {
        x1: f64,
        y1: f64,
        x: f64,
        y: f64,
    },
    HorizontalTo {
        x: f64,
    },
    VerticalTo {
        y: f64,
    },
    Close,
}

#[derive(Clone, Debug)]
pub struct Path {
    pub commands: Vec<Command>,
}

impl Path {
    pub fn new(commands: Vec<Command>) -> Self {
        Self { commands }
    }

    pub fn to_svg(&self) -> String {
        let mut output = String::new();
        for command in &self.commands {
            match command {
                Command::MoveTo { x, y } => {
                    write!(output, "M{} {}", rounded_number(*x), rounded_number(*y)).unwrap();
                }
                Command::LineTo { x, y } => {
                    write!(output, "L{} {}", rounded_number(*x), rounded_number(*y)).unwrap();
                }
                Command::CubicTo {
                    x1,
                    y1,
                    x2,
                    y2,
                    x,
                    y,
                } => {
                    write!(
                        output,
                        "C{} {} {} {} {} {}",
                        rounded_number(*x1),
                        rounded_number(*y1),
                        rounded_number(*x2),
                        rounded_number(*y2),
                        rounded_number(*x),
                        rounded_number(*y)
                    )
                    .unwrap();
                }
                Command::QuadraticTo { x1, y1, x, y } => {
                    write!(
                        output,
                        "Q{} {} {} {}",
                        rounded_number(*x1),
                        rounded_number(*y1),
                        rounded_number(*x),
                        rounded_number(*y)
                    )
                    .unwrap();
                }
                Command::HorizontalTo { x } => {
                    write!(output, "H{}", rounded_number(*x)).unwrap();
                }
                Command::VerticalTo { y } => {
                    write!(output, "V{}", rounded_number(*y)).unwrap();
                }
                Command::Close => output.push('Z'),
            }
        }
        output
    }
}

pub fn rounded(value: f64) -> f64 {
    // Math.round chooses the integer toward positive infinity on a negative tie.
    let scaled = value * 100.0;
    let floor = scaled.floor();
    let value = (floor + if scaled - floor >= 0.5 { 1.0 } else { 0.0 }) / 100.0;
    if value == 0.0 { 0.0 } else { value }
}

pub fn rounded_number(value: f64) -> String {
    rounded(value).to_string()
}

pub fn superellipse(cx: f64, cy: f64, rx: f64, ry: f64, n: f64, rot: f64) -> Path {
    let k = ((8.0 * 2.0_f64.powf(-1.0 / n) - 4.0) / 3.0).min(1.0);
    let ak = rx * k;
    let bk = ry * k;
    let points = [
        (rx, 0.0),
        (rx, bk),
        (ak, ry),
        (0.0, ry),
        (-ak, ry),
        (-rx, bk),
        (-rx, 0.0),
        (-rx, -bk),
        (-ak, -ry),
        (0.0, -ry),
        (ak, -ry),
        (rx, -bk),
        (rx, 0.0),
    ];

    let radians = rot * std::f64::consts::PI / 180.0;
    let cos = radians.cos();
    let sin = radians.sin();
    let transform = |(x, y): (f64, f64)| (cx + x * cos - y * sin, cy + x * sin + y * cos);
    let points = points.map(transform);
    let mut commands = vec![Command::MoveTo {
        x: points[0].0,
        y: points[0].1,
    }];
    for index in [1, 4, 7, 10] {
        commands.push(Command::CubicTo {
            x1: points[index].0,
            y1: points[index].1,
            x2: points[index + 1].0,
            y2: points[index + 1].1,
            x: points[index + 2].0,
            y: points[index + 2].1,
        });
    }
    commands.push(Command::Close);
    Path::new(commands)
}

pub fn blob_path(cx: f64, cy: f64, rx: f64, ry: f64, radii: &[f64], rot: f64) -> Path {
    let count = radii.len();
    let rotation = rot * std::f64::consts::PI / 180.0;
    let points = radii
        .iter()
        .enumerate()
        .map(|(index, radius)| {
            let angle = rotation + 2.0 * std::f64::consts::PI * index as f64 / count as f64;
            (
                cx + rx * radius * angle.cos(),
                cy + ry * radius * angle.sin(),
            )
        })
        .collect::<Vec<_>>();
    let point = |index: isize| {
        let wrapped = index.rem_euclid(count as isize) as usize;
        points[wrapped]
    };

    let mut commands = vec![Command::MoveTo {
        x: points[0].0,
        y: points[0].1,
    }];
    for index in 0..count {
        let i = index as isize;
        let (x0, y0) = point(i - 1);
        let (x1, y1) = point(i);
        let (x2, y2) = point(i + 1);
        let (x3, y3) = point(i + 2);
        commands.push(Command::CubicTo {
            x1: x1 + (x2 - x0) / 6.0,
            y1: y1 + (y2 - y0) / 6.0,
            x2: x2 - (x3 - x1) / 6.0,
            y2: y2 - (y3 - y1) / 6.0,
            x: x2,
            y: y2,
        });
    }
    commands.push(Command::Close);
    Path::new(commands)
}

pub fn polygon(cx: f64, cy: f64, rx: f64, ry: f64, sides: usize, round: f64, rot: f64) -> Path {
    let k = if round > 0.0 {
        if round < 1.0 { round / 2.0 } else { 0.5 }
    } else {
        0.0
    };
    let first_angle = rot * std::f64::consts::PI / 180.0 - std::f64::consts::PI / 2.0;
    let vertices = (0..sides)
        .map(|index| {
            let angle = first_angle + 2.0 * std::f64::consts::PI * index as f64 / sides as f64;
            (cx + rx * angle.cos(), cy + ry * angle.sin())
        })
        .collect::<Vec<_>>();
    let vertex = |index: isize| vertices[index.rem_euclid(sides as isize) as usize];
    let cut = |from: isize, to: isize| {
        let (x0, y0) = vertex(from);
        let (x1, y1) = vertex(to);
        (x0 + (x1 - x0) * k, y0 + (y1 - y0) * k)
    };

    let start = cut(0, -1);
    let mut commands = vec![Command::MoveTo {
        x: start.0,
        y: start.1,
    }];
    for index in 0..sides {
        let (x, y) = vertex(index as isize);
        let outgoing = cut(index as isize, index as isize + 1);
        commands.push(Command::QuadraticTo {
            x1: x,
            y1: y,
            x: outgoing.0,
            y: outgoing.1,
        });
        if k < 0.5 {
            let incoming = cut(index as isize + 1, index as isize);
            commands.push(Command::LineTo {
                x: incoming.0,
                y: incoming.1,
            });
        }
    }
    commands.push(Command::Close);
    Path::new(commands)
}

pub fn box_path(cx: f64, cy: f64, rx: f64, ry: f64) -> Path {
    Path::new(vec![
        Command::MoveTo {
            x: cx - rx,
            y: cy - ry,
        },
        Command::HorizontalTo { x: cx + rx },
        Command::VerticalTo { y: cy + ry },
        Command::HorizontalTo { x: cx - rx },
        Command::Close,
    ])
}

pub fn taper(cx: f64, cy: f64, rx: f64, ry: f64, tip: f64) -> Path {
    let tip = tip.max(1.05);
    let tangent_x = rx * (1.0 - 1.0 / (tip * tip)).sqrt();
    let tangent_y = cy - ry / tip;
    let apex_y = cy - tip * ry;
    let flank_x = tangent_x * 0.14;
    let flank_y = tangent_y + 0.86 * (apex_y - tangent_y);
    Path::new(vec![
        Command::MoveTo {
            x: cx - tangent_x,
            y: tangent_y,
        },
        Command::LineTo {
            x: cx - flank_x,
            y: flank_y,
        },
        Command::QuadraticTo {
            x1: cx,
            y1: apex_y,
            x: cx + flank_x,
            y: flank_y,
        },
        Command::LineTo {
            x: cx + tangent_x,
            y: tangent_y,
        },
        Command::Close,
    ])
}
