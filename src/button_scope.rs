use std::time::{Duration, Instant};

use ggez::{Context, GameResult, graphics::{self, Color, DrawMode, DrawParam, Mesh}};

use crate::{controller::{BUTTONS, Controller}, oscilloscope::{Scope, ScopeCanvas, ScopeDirection}, reset_graphics};

pub struct ButtonScope {
    scope_canvas: ScopeCanvas,
    scope_start_time: Instant,
    controller: Controller,
    last_buttons: [Option<Instant>; 12],
    button_order: Vec<usize>,
    latest_time: Instant,
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
    pub fn new(ctx: &mut Context, x: f32, y: f32, width: f32, height: f32, direction: ScopeDirection) -> GameResult<Self> {
        Ok(ButtonScope {
            scope_canvas: ScopeCanvas::new(ctx, x, y, width, height, direction)?,
            scope_start_time: Instant::now(),
            last_buttons: Default::default(),
            controller: Controller::new(),
            button_order: vec![0, 1, 2, 3, 9, 10, 11, 6],
            latest_time: Instant::now(),
        })
    }

    fn time_offset_rev(pos: f32) -> Duration {
        Duration::from_micros((pos * 1000.).floor() as u64)
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
        let rect = self.get_rect(ctx, button_index, to.saturating_duration_since(from))?;
        let offset = ButtonScope::time_offset(from.saturating_duration_since(self.scope_start_time));
        graphics::draw(ctx, &rect, DrawParam::new().dest([offset, 0.]))?;
        
        Ok(())
    }

    pub fn draw_line_at_time(&self, ctx: &mut Context, time: Instant) -> GameResult<()> {
        let now = self.latest_time;
        let offset = ButtonScope::time_offset(now.saturating_duration_since(time));
        
        self.scope_canvas.draw_line_at_offset(ctx, offset)?;
        Ok(())
    }
}

impl Scope for ButtonScope {
    type Data = [u8; 2];

    fn update(&mut self, ctx: &mut Context, new_item: Self::Data, time: Instant) -> GameResult<()> {
        let mut buffer = [0; 8];
        buffer[0..2].copy_from_slice(&new_item);
        self.controller.from_buffer(&buffer);
        self.latest_time = time;

        let point_time_offset = ButtonScope::time_offset(time.saturating_duration_since(self.scope_start_time));

        self.scope_canvas.setup_drawing(ctx, point_time_offset)?;

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
        if self.scope_canvas.update(ctx)? {
            for (i, _button) in self.button_order.iter().enumerate() {
                if let Some(button_time) = self.last_buttons[i] {
                    self.draw_to_canvas(ctx, button_time, time, i)?;

                    self.last_buttons[i] = Some(time);
                }
            }

            self.scope_start_time = time;
        }

        reset_graphics(ctx)?;

        Ok(())
    }

    fn draw(&self, ctx: &mut Context) -> GameResult<()> {
        let now = Instant::now();
        self.scope_canvas.draw(ctx, self.scope_canvas.x, -self.scope_canvas.y)?;
        for (i, _button) in self.button_order.iter().enumerate() { 
            if let Some(press_time) = self.last_buttons[i] {
                let rect = self.get_rect(ctx, i, now.saturating_duration_since(press_time))?;
                graphics::draw(ctx, &rect, DrawParam::new().dest([self.scope_canvas.x, self.scope_canvas.y]))?;
            }
        }
        Ok(())
    }

    fn reset(&mut self, ctx: &mut Context) {//TODO
        self.scope_canvas.reset(ctx);
    }

    fn get_time_from_pos(&mut self, x: f32, y: f32) -> Option<Instant> {
        let value = self.scope_canvas.get_offset_from_pos(x - self.scope_canvas.x, y - self.scope_canvas.y)?;
        let time = ButtonScope::time_offset_rev(value);
        let now = self.latest_time;
        Some(now - time)
    }
}
