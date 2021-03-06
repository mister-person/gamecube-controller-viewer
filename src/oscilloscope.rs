use std::time::{Duration, Instant};

use ggez::{Context, GameResult, graphics::{self, Canvas, Color, DrawMode, DrawParam, Mesh, Rect, get_window_color_format}};

use crate::{reset_graphics, zones};

pub trait Scope {
    type Data;

    fn update(&mut self, ctx: &mut Context, new_item: Self::Data, time: Instant) -> GameResult<()>;
    fn draw(&self, ctx: &mut Context) -> GameResult<()>;
    fn reset(&mut self, ctx: &mut Context);

    fn get_time_from_pos(&mut self, x: f32, y: f32) -> Option<Instant>;
}

pub struct ScopeCanvas {
    pub canvas: Canvas,
    pub canvas_old: Canvas,
    canvas_scratch: Canvas,
    pub offset: f32,
    pub offset_old: f32,
    pub direction: ScopeDirection,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl ScopeCanvas {
    pub fn new(ctx: &mut Context, x: f32, y: f32, width: f32, height: f32, direction: ScopeDirection) -> GameResult<ScopeCanvas> {
        let scope_canvas = Canvas::new(ctx, width as u16, height as u16, ggez::conf::NumSamples::One, get_window_color_format(ctx))?;
        let scope_canvas_old = Canvas::new(ctx, width as u16, height as u16, ggez::conf::NumSamples::One, get_window_color_format(ctx))?;
        let canvas_scratch = Canvas::new(ctx, width as u16, height as u16, ggez::conf::NumSamples::One, get_window_color_format(ctx))?;
        graphics::set_canvas(ctx, Some(&scope_canvas));
        graphics::clear(ctx, Color::from_rgba(0, 0, 0, 0));
        graphics::set_canvas(ctx, Some(&scope_canvas_old));
        graphics::clear(ctx, Color::from_rgba(0, 0, 0, 0));
        graphics::set_canvas(ctx, None);
        Ok(ScopeCanvas {
            width,
            height,
            x,
            y,
            canvas: scope_canvas,
            canvas_old: scope_canvas_old,
            canvas_scratch,
            offset: 0.,
            offset_old: 0.,
            direction,
        })
    }

    pub fn setup_drawing(&mut self, ctx: &mut Context, new_offset: f32) -> GameResult<()> {
        graphics::set_canvas(ctx, Some(&self.canvas));
        graphics::set_screen_coordinates(ctx, Rect::new(0., 0., self.width as f32, self.height as f32))?;
        self.offset = new_offset;
        Ok(())
    }

    pub fn draw(&self, ctx: &mut Context, x: f32, y: f32) -> GameResult<()> {
        let scale;
        let offset;
        let offset2;
        let rect;
        if self.direction == ScopeDirection::Horizontal {
            scale = [1., 1.];
            //offset = [-self.width + -x + self.offset, -y];
            offset = [-x - self.offset, -y];
            offset2 = [-self.width + self.offset, 0.];
            //offset2 = [-x, -y];
            //offset2 = [x + self.offset, -y];
            //rect = [0.5, 0., 1., 1.].into();
            rect = [1. - self.offset / self.width, 0., 1., 1.].into();
            //rect = [0., 0., 1. - self.offset / self.width, 1.].into();
        }
        else {//TODO
            scale = [1., -1.];
            offset = [x, y + self.offset];
            offset2 = [x, y + self.offset + self.offset_old];
            rect = [0., 0., 1., 0.5].into();
        }

        //hack because drawparam src is buggy
        graphics::set_screen_coordinates(ctx, Rect::new(0., 0., self.width as f32, self.height as f32))?;
        graphics::set_canvas(ctx, Some(&self.canvas_scratch));
        graphics::clear(ctx, Color::from_rgba(0, 0, 0, 0));
        graphics::draw(ctx, &self.canvas_old, DrawParam::new().src(rect).offset(offset2).scale(scale))?;
        reset_graphics(ctx)?;

        graphics::draw(ctx, &self.canvas_scratch, DrawParam::new().offset([-x + self.width - self.offset, -y]).scale(scale))?;
        graphics::draw(ctx, &self.canvas, DrawParam::new().offset(offset).scale(scale))?;
        Ok(())
    }

    pub fn update(&mut self, ctx: &mut Context) -> GameResult<bool> {
        let point_time_offset = self.offset;
        if point_time_offset <= 0. {
            self.offset_old = self.offset;
            let testrect = Mesh::new_rectangle(ctx, DrawMode::fill(), [0., 0., 5., 5.].into(), Color::BLUE)?;
            std::mem::swap(&mut self.canvas, &mut self.canvas_old);
            graphics::set_canvas(ctx, Some(&self.canvas));
            graphics::clear(ctx, Color::from_rgba(0, 0, 0, 0));
            //uncomment to see where the canvases are
            graphics::clear(ctx, Color::from_rgba(0, 0, 0, 0));
            graphics::draw(ctx, &testrect, DrawParam::new())?;
            return Ok(true)
        }

        Ok(false)

    }

    pub fn reset(&mut self, ctx: &mut Context) {
        self.offset = 0.;
        graphics::set_canvas(ctx, Some(&self.canvas));
        graphics::clear(ctx, Color::from_rgba(0, 0, 0, 0));
        graphics::set_canvas(ctx, Some(&self.canvas_old));
        graphics::clear(ctx, Color::from_rgba(0, 0, 0, 0));
        graphics::set_canvas(ctx, None);
    }

    pub fn get_offset_from_pos(&mut self, x: f32, y: f32) -> Option<f32> {
        let (value, orthogonal_val) = match self.direction { ScopeDirection::Horizontal => (x, y), ScopeDirection::Vertical => (y, x), };
        let (max, orthogonal_max) = match self.direction {
            ScopeDirection::Horizontal => (self.width, self.height),
            ScopeDirection::Vertical => (self.height, self.width),
        };
        if value >= 0. && value < max as f32 && orthogonal_val >= 0. && orthogonal_val < orthogonal_max as f32 {
            return Some(value)
        }
        None
    }

    pub fn draw_line_at_offset(&self, ctx: &mut Context, offset: f32) -> GameResult<()> {
        let line_coords = match self.direction {
            ScopeDirection::Vertical => [[self.x, self.y + offset], [self.x + self.width as f32, self.y + offset]],
            ScopeDirection::Horizontal => [[self.x + offset, self.y], [self.x + offset, self.y + self.height as f32]],
        };
        let line = Mesh::new_line(ctx, &line_coords, 1.0, Color::WHITE)?;
        graphics::draw(ctx, &line, DrawParam::new())?;
        Ok(())
    }
}

#[derive(Clone, Copy)]
pub struct ScopePoint {
    point: (i8, (i8, i8)),
    time: Instant,
}
#[derive(PartialEq)]
pub enum ScopeDirection {
    Vertical,
    Horizontal,
}
pub struct Oscilloscope {
    pub scope_start_time: Instant,
    last_point: Option<ScopePoint>,
    pub plane: Box<dyn zones::Plane>,
    scope_canvas: ScopeCanvas,
    x: f32,
    y: f32,
}

impl Oscilloscope {
    pub fn new(ctx: &mut Context, x: f32, y: f32, width: f32, height: f32, direction: ScopeDirection) -> GameResult<Self> {
        Ok(Oscilloscope {
            scope_canvas: ScopeCanvas::new(ctx, x, y, width, height, direction)?,
            x: x as f32,
            y: y as f32,
            scope_start_time: Instant::now(),
            last_point: None,
            plane: Box::new(zones::PlaneWithZones::default_plane()),
        })
    }

    fn time_offset(&self, time: Duration) -> f32 {
        self.scope_canvas.width - time.as_micros() as f32 / 1000.
    }

    fn time_offset_rev(&self, pos: f32) -> Duration {
        Duration::from_micros(((self.scope_canvas.width - pos) * 1000.).floor() as u64)
    }

    pub fn draw_line_at_time(&self, ctx: &mut Context, time: Instant) -> GameResult<()> {
        let now = match self.last_point {
            Some(point) => point.time,
            None => self.scope_start_time,
        };
        let offset = self.time_offset(now.saturating_duration_since(time));
        self.scope_canvas.draw_line_at_offset(ctx, offset)?;
        Ok(())
    }

    fn to_screen_coords(&self, coords: i8) -> f32 {
        let width = match self.scope_canvas.direction {
            ScopeDirection::Vertical => self.scope_canvas.width,
            ScopeDirection::Horizontal => self.scope_canvas.height,
        };
        let middle = width / 2.;
        let scale = width / 2. / 90.;
        middle + (coords as f32)*scale
    }
}
    
impl Scope for Oscilloscope {
    type Data = (i8, (i8, i8));

    fn update(&mut self, ctx: &mut Context, new_point: (i8, (i8, i8)), time: Instant) -> GameResult<()> {
        //maybe move to trait?

        let point_screen = self.to_screen_coords(new_point.0);
        let point_time_offset = self.time_offset(time.saturating_duration_since(self.scope_start_time));

        //TODO let people change speed of oscilloscope

        //TODO get rid of all these magic numbers at some point, 
        //refactor setting canvas so I don't have to un set it all the time ditto set_screen_coordinates
        //break up this function it's waay too big
        let last_point = &self.last_point;
        if let Some(ScopePoint { point: last_point, time: last_time }) = last_point {
            let last_point_screen = self.to_screen_coords(last_point.0);
            let last_point_time_offset = self.time_offset(last_time.saturating_duration_since(self.scope_start_time));

            let color = self.plane.get_zone(last_point.1).fg_color.into();

            self.scope_canvas.setup_drawing(ctx, point_time_offset)?;

            let line_coords;
            if self.scope_canvas.direction == ScopeDirection::Horizontal {
                line_coords = [
                    [self.scope_canvas.width - last_point_time_offset, self.scope_canvas.height - last_point_screen],
                    [self.scope_canvas.width - point_time_offset, self.scope_canvas.height - point_screen]
                ];
            }
            else {
                line_coords = [[last_point_screen, last_point_time_offset], [point_screen, point_time_offset]];
            }
            let line = graphics::Mesh::new_line(ctx, &line_coords, 1., color);
            if let Ok(line) = line {
                graphics::draw(ctx, &line, DrawParam::new())?;
            }
            let point = Mesh::new_rectangle(ctx, DrawMode::fill(), [line_coords[0][0] - 1., line_coords[0][1] - 1., 3., 3.].into(), color)?;
            graphics::draw(ctx, &point, DrawParam::new())?;

            if self.scope_canvas.update(ctx)? {
                self.scope_start_time = time;
            }

            crate::reset_graphics(ctx)?;
        }

        self.last_point = Some(ScopePoint { point: new_point, time } );
        Ok(())
    }
    
    fn draw(&self, ctx: &mut Context) -> GameResult<()> {
        self.scope_canvas.draw(ctx, self.x, self.y)?;
        Ok(())
    }

    fn reset(&mut self, ctx: &mut Context) {
        self.scope_start_time = Instant::now();//this might not work
        self.last_point = None;
        self.scope_canvas.reset(ctx);
    }

    fn get_time_from_pos(&mut self, x: f32, y: f32) -> Option<Instant> {
        let value = self.scope_canvas.get_offset_from_pos(x - self.x, y - self.y)?;
        let now = match self.last_point {
            Some(point) => point.time,
            None => Instant::now(),
        };
        let time = self.time_offset_rev(value);
        Some(now - time)
    }

}
