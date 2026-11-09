//! Geometry copied from the retained ResultWindow SVG icon system (24-unit grid).
use crate::{Color, DrawCommand, Point, Rect, Scene};
#[derive(Clone, Copy)]
pub(super) enum Icon {
    Close,
    Pin,
    Language,
    Scan,
    Upload,
    Retry,
    Clear,
    Copy,
    Favorite,
    Volume,
    Up,
    Down,
    Swap,
}
pub(super) fn draw(scene: &mut Scene, rect: Rect, icon: Icon, color: Color) {
    let s = rect.width / 24.;
    let point = |x: f32, y: f32| Point {
        x: rect.x + x * s,
        y: rect.y + y * s,
    };
    let mut path = |points: &[(f32, f32)]| {
        for pair in points.windows(2) {
            scene.commands.push(DrawCommand::Line {
                from: point(pair[0].0, pair[0].1),
                to: point(pair[1].0, pair[1].1),
                width: 2. * s,
                color,
            });
        }
    };
    match icon {
        Icon::Close => {
            path(&[(18., 6.), (6., 18.)]);
            path(&[(6., 6.), (18., 18.)]);
        }
        Icon::Pin => {
            path(&[
                (14., 4.),
                (14., 8.),
                (18., 12.),
                (18., 14.),
                (6., 14.),
                (6., 12.),
                (10., 8.),
                (10., 4.),
            ]);
            path(&[(9., 4.), (15., 4.)]);
            path(&[(12., 14.), (12., 21.)]);
        }
        Icon::Language => {
            path(&[(5., 8.), (11., 14.)]);
            path(&[(4., 14.), (10., 8.), (12., 5.)]);
            path(&[(2., 5.), (14., 5.)]);
            path(&[(7., 2.), (8., 2.)]);
            path(&[(22., 22.), (17., 12.), (12., 22.)]);
            path(&[(14., 18.), (20., 18.)]);
        }
        Icon::Scan => {
            path(&[(4., 7.), (4., 5.), (5., 4.), (7., 4.)]);
            path(&[(17., 4.), (19., 4.), (20., 5.), (20., 7.)]);
            path(&[(20., 17.), (20., 19.), (19., 20.), (17., 20.)]);
            path(&[(7., 20.), (5., 20.), (4., 19.), (4., 17.)]);
            path(&[(7., 12.), (17., 12.)]);
        }
        Icon::Upload => {
            path(&[(12., 16.), (12., 4.)]);
            path(&[(7., 9.), (12., 4.), (17., 9.)]);
            path(&[
                (20., 16.),
                (20., 19.),
                (18., 21.),
                (6., 21.),
                (4., 19.),
                (4., 16.),
            ]);
        }
        Icon::Retry => {
            let points: Vec<_> = (0..=22)
                .map(|i| {
                    let a = (180. + i as f32 * 285. / 22.).to_radians();
                    (12. + 8. * a.cos(), 12. + 8. * a.sin())
                })
                .collect();
            path(&points);
            path(&[(4., 4.), (4., 8.7), (8.7, 8.7)]);
        }
        Icon::Clear => {
            path(&[(5., 7.), (15., 7.)]);
            path(&[(5., 12.), (12., 12.)]);
            path(&[(5., 17.), (11., 17.)]);
            path(&[(16., 10.), (20., 14.)]);
            path(&[(20., 10.), (16., 14.)]);
        }
        Icon::Copy => {
            path(&[
                (5., 15.),
                (4., 15.),
                (2., 13.),
                (2., 4.),
                (4., 2.),
                (13., 2.),
                (15., 4.),
                (15., 5.),
            ]);
            scene.commands.push(DrawCommand::StrokeRect {
                rect: Rect {
                    x: rect.x + 9. * s,
                    y: rect.y + 9. * s,
                    width: 11. * s,
                    height: 11. * s,
                },
                radius: 2. * s,
                width: 2. * s,
                color,
            });
        }
        Icon::Favorite => path(&[
            (12., 3.),
            (14.75, 8.57),
            (20.9, 9.47),
            (16.45, 13.8),
            (17.5, 19.92),
            (12., 17.03),
            (6.5, 19.92),
            (7.55, 13.8),
            (3.1, 9.47),
            (9.25, 8.57),
            (12., 3.),
        ]),
        Icon::Volume => {
            path(&[
                (11., 5.),
                (6., 9.),
                (3., 9.),
                (3., 15.),
                (6., 15.),
                (11., 19.),
                (11., 5.),
            ]);
            path(&[(16., 9.5), (17., 11.), (17., 13.), (16., 14.5)]);
            path(&[(19., 7.), (21., 10.), (21., 14.), (19., 17.)]);
        }
        Icon::Up => path(&[(18., 15.), (12., 9.), (6., 15.)]),
        Icon::Down => path(&[(6., 9.), (12., 15.), (18., 9.)]),
        Icon::Swap => {
            path(&[(6., 7.), (18., 7.)]);
            path(&[(15., 4.), (18., 7.), (15., 10.)]);
            path(&[(18., 17.), (6., 17.)]);
            path(&[(9., 14.), (6., 17.), (9., 20.)]);
        }
    }
}
