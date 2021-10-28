use std::collections::{HashMap, HashSet};

use ggez::{Context, GameResult, graphics::{self, Canvas, Color, DrawMode, DrawParam, Mesh, Text, get_window_color_format}};

use crate::{button_scope::BUTTON_COLORS, controller::{self, Controller}};

type Layout = [(controller::Button, char, [f32; 2], usize); 8];

pub const GC_LAYOUT: Layout = [
    (controller::A_BUTTON, 'A', [40., 60.], 1),
    (controller::B_BUTTON, 'B', [20., 70.], 2),
    (controller::X_BUTTON, 'X', [60., 50.], 3),
    (controller::Y_BUTTON, 'Y', [36., 40.], 4),
    (controller::Z_BUTTON, 'Z', [60., 20.], 9),
    (controller::L_BUTTON, 'L', [0., 0.], 10),
    (controller::R_BUTTON, 'R', [60., 0.], 11),
    (controller::D_UP_BUTTON, '+', [0., 40.], 6),
];

pub const LINE_LAYOUT: Layout = [
    (controller::A_BUTTON, 'A', [0., 0.], 0),
    (controller::B_BUTTON, 'B', [15., 15.], 1),
    (controller::X_BUTTON, 'X', [0., 30.], 2),
    (controller::Y_BUTTON, 'Y', [15., 45.], 3),
    (controller::Z_BUTTON, 'Z', [0., 60.], 9),
    (controller::L_BUTTON, 'L', [15., 75.], 10),
    (controller::R_BUTTON, 'R', [0., 90.], 11),
    (controller::D_UP_BUTTON, '+', [15., 105.], 6),
];

pub fn draw_buttons(ctx: &mut Context, controller: &Controller, x: f32, y: f32, layout: Layout) -> GameResult<()> {
    for (button, letter, pos, button_index) in layout {
        draw_button(ctx, [pos[0] + x, pos[1] + y], letter, BUTTON_COLORS[button_index], controller.is_down(&button))?;
    }
    Ok(())
}

pub fn draw_button(ctx: &mut Context, pos: [f32; 2], button: char, color: Color, pressed: bool) -> GameResult<()> {
    let drawmode = if pressed {DrawMode::fill()} else {DrawMode::stroke(1.)};
    let circle = Mesh::new_circle(ctx, drawmode, pos, 10., 0.1, color)?;
    let text = Text::new(button);
    let text_color = if pressed {Color::BLACK} else {Color::WHITE};
    graphics::draw(ctx, &circle, DrawParam::new())?;
    graphics::draw(ctx, &text, DrawParam::new().dest(pos).offset([0.5, 0.5]).color(text_color))?;
    Ok(())
}
