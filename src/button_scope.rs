use std::time::{Duration, Instant};

use ggez::{Context, GameResult, graphics::{self, Canvas, Color, DrawMode, DrawParam, Mesh, get_window_color_format}};

use crate::{controller::{BUTTONS, Controller}, oscilloscope::{Scope, ScopeDirection}, reset_graphics};

pub struct ButtonScope {
    scope_start_time: Instant,
    scope_canvas: Canvas,
    scope_canvas_old: Canvas,
    scope_offset: f32,
    scope_offset_old: f32,
    controller: Controller,
    last_buttons: [Option<Instant>; 12],
    button_order: Vec<usize>,
    canvas_width: u16,
    canvas_height: u16,
    direction: ScopeDirection,//TODO
}

pub const BUTTON_COLORS: [Color; 12] = [
    Color::GREEN,
    Color::RED,
    Color {r: 0.8, g: 0.8, b: 0.8, a: 1.0},
    Color {r: 0.75, g: 0.75, b: 0.75, a: 1.0},
    Color {r: 0.3, g: 0.3, b: 0.3, a: 1.0},
    Color {r: 0.3, g: 0.3, b: 0.3, a: 1.0},
    Color {r: 0.3, g: 0.3, b: 0.3, a: 1.0},
    Color {r: 0.3, g: 0.3, b: 0.3, a: 1.0},
    Color {r: 0.3, g: 0.3, b: 0.3, a: 1.0},
    Color {r: 0.2, g: 0.0, b: 1.0, a: 1.0},
    Color {r: 0.6, g: 0.6, b: 0.6, a: 1.0},
    Color {r: 0.6, g: 0.6, b: 0.6, a: 1.0},
];

impl ButtonScope {
    pub fn new(ctx: &mut Context, width: u16, height: u16, direction: ScopeDirection) -> GameResult<Self> {
        let scope_canvas = Canvas::new(ctx, width, height, ggez::conf::NumSamples::One, get_window_color_format(ctx))?;
        let scope_canvas_old = Canvas::new(ctx, width, height, ggez::conf::NumSamples::One, get_window_color_format(ctx))?;
        Ok(ButtonScope {
            scope_canvas,
            scope_canvas_old,
            scope_start_time: Instant::now(),
            scope_offset: 0.,
            scope_offset_old: 0.,
            last_buttons: Default::default(),
            canvas_width: width,
            canvas_height: height,
            direction,
            controller: Controller::new(),
            button_order: vec![0, 1, 2, 3, 9, 10, 11, 6],
        })
    }

    fn time_offset(time: Duration) -> f32 {
        time.as_micros() as f32 / 1000.
    }

    fn get_rect(&self, ctx: &mut Context, button_index: usize, duration: Duration) -> GameResult<Mesh> {
        let length = ButtonScope::time_offset(duration);
        let dimensions = [0., (button_index as f32)*15., length, 5.].into();
        Mesh::new_rectangle(ctx, DrawMode::fill(), dimensions, BUTTON_COLORS[self.button_order[button_index]])
    }

    fn draw_to_canvas(&self, ctx: &mut Context, from: Instant, to: Instant, button_index: usize) -> GameResult<()> {
        graphics::set_canvas(ctx, Some(&self.scope_canvas));
        graphics::set_screen_coordinates(ctx, [0., 0., self.canvas_width as f32, self.canvas_height as f32].into())?;
        let rect = self.get_rect(ctx, button_index, to.saturating_duration_since(from))?;
        let offset = ButtonScope::time_offset(from.saturating_duration_since(self.scope_start_time));
        graphics::draw(ctx, &rect, DrawParam::new().dest([offset, 0.]))?;

        reset_graphics(ctx)?;
        
        Ok(())
    }
}

impl Scope for ButtonScope {
    type Data = [u8; 2];

    fn update(&mut self, ctx: &mut Context, new_item: Self::Data, time: Instant) -> GameResult<()> {
        let mut buffer = [0; 8];
        buffer[0..2].copy_from_slice(&new_item);
        self.controller.from_buffer(&buffer);

        for (i, button) in self.button_order.iter().enumerate() {
            if self.controller.just_pressed(&BUTTONS[*button]) {
                self.last_buttons[i] = Some(time);
            }
            else if !self.controller.is_down(&BUTTONS[*button]) {

                if let Some(button_time) = self.last_buttons[i] {
                    self.draw_to_canvas(ctx, button_time, time, i)?;

                    self.last_buttons[i] = None;
                }

            }
        };

        let point_time_offset = ButtonScope::time_offset(time.saturating_duration_since(self.scope_start_time));

        self.scope_offset = point_time_offset;
        if point_time_offset > self.canvas_width as f32 {
            for (i, button) in self.button_order.iter().enumerate() {
                if let Some(button_time) = self.last_buttons[i] {
                    self.draw_to_canvas(ctx, button_time, time, i)?;

                    self.last_buttons[i] = Some(time);
                }
            }

            self.scope_offset_old = point_time_offset;
            self.scope_start_time = time;
            //let testrect = Mesh::new_rectangle(ctx, DrawMode::fill(), [0., 0., 5., 5.].into(), Color::MAGENTA)?;
            std::mem::swap(&mut self.scope_canvas, &mut self.scope_canvas_old);
            graphics::set_canvas(ctx, Some(&self.scope_canvas));
            graphics::clear(ctx, Color::from_rgba(0, 0, 0, 0));
            //graphics::draw(ctx, &testrect, DrawParam::new())?;
        }

        Ok(())
    }

    fn draw(&self, ctx: &mut Context, x: f32, y: f32) -> GameResult<()> {
        let now = Instant::now();
        graphics::draw(ctx, &self.scope_canvas, DrawParam::new().dest([x - 0. + self.scope_offset, y]).scale([-1., 1.]))?;
        graphics::draw(ctx, &self.scope_canvas_old, DrawParam::new().dest([x - 0. + self.scope_offset + self.scope_offset_old, y]).scale([-1., 1.]))?;
        for (i, button) in self.button_order.iter().enumerate() { 
            if let Some(press_time) = self.last_buttons[i] {
                let rect = self.get_rect(ctx, i, now.saturating_duration_since(press_time))?;
                graphics::draw(ctx, &rect, DrawParam::new().dest([x, y]))?;
            }
        }
        Ok(())
    }

    fn reset(&mut self, ctx: &mut Context) {//TODO
        
    }

    fn get_time_from_pos(&mut self, x: f32, y: f32) -> Option<Instant> {
        None
    }
}
