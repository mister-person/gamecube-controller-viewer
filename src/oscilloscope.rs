use std::time::{Duration, Instant};

use ggez::{Context, GameResult, graphics::{self, Canvas, Color, DrawMode, DrawParam, Mesh, Rect, get_window_color_format}};

use crate::zones;

pub trait Scope {
    type Data;

    fn update(&mut self, ctx: &mut Context, new_item: Self::Data, time: Instant) -> GameResult<()>;
    fn draw(&self, ctx: &mut Context, x: f32, y: f32) -> GameResult<()>;
    fn reset(&mut self, ctx: &mut Context);

    fn get_time_from_pos(&mut self, x: f32, y: f32) -> Option<Instant>;
}

pub struct ScopePoint {
    point: (i8, i8),
    time: Instant,
}
#[derive(PartialEq)]
pub enum ScopeDirection {
    Vertical,
    Horizontal,
}
pub struct Oscilloscope {
    scope_start_time: Instant,
    scope_canvas: Canvas,
    scope_canvas_old: Canvas,
    scope_offset: f32,
    scope_offset_old: f32,
    last_point: Option<ScopePoint>,
    plane: Box<dyn zones::Plane>,
    canvas_width: u16,
    canvas_height: u16,
    direction: ScopeDirection,
}

impl Oscilloscope {
    pub fn new(ctx: &mut Context, width: u16, height: u16, direction: ScopeDirection) -> GameResult<Self> {
        let scope_canvas = Canvas::new(ctx, width, height, ggez::conf::NumSamples::One, get_window_color_format(ctx))?;
        let scope_canvas_old = Canvas::new(ctx, width, height, ggez::conf::NumSamples::One, get_window_color_format(ctx))?;
        Ok(Oscilloscope {
            scope_canvas,
            scope_canvas_old,
            scope_start_time: Instant::now(),
            scope_offset: 0.,
            scope_offset_old: 0.,
            last_point: None,
            plane: Box::new(zones::Test2 {}),
            canvas_width: width,
            canvas_height: height,
            direction,
        })
    }
}
    
impl Scope for Oscilloscope {
    type Data = (i8, i8);

    fn update(&mut self, ctx: &mut Context, new_point: (i8, i8), time: Instant) -> GameResult<()> {
        //maybe move to trait?
        fn time_offset(time: Duration) -> f32 {
            time.as_micros() as f32 / 1000.
        }

        //TODO this assumes canvas height
        let point_screen = to_screen_coords(&new_point);
        let point_time_offset = time_offset(time.saturating_duration_since(self.scope_start_time));

        //TODO let people change speed of oscilloscope

        //TODO get rid of all these magic numbers at some point, 
        //refactor setting canvas so I don't have to un set it all the time ditto set_screen_coordinates
        //break up this function it's waay too big
        let last_point = &self.last_point;
        if let Some(ScopePoint { point: last_point, time: last_time }) = last_point {
            let last_point_screen = to_screen_coords(&last_point);
            let last_point_time_offset = time_offset(last_time.saturating_duration_since(self.scope_start_time));

            let color = self.plane.get_zone(*last_point).fg_color.into();

            self.scope_offset = point_time_offset;
            
            graphics::set_canvas(ctx, Some(&self.scope_canvas));
            graphics::set_screen_coordinates(ctx, Rect::new(0., 0., self.canvas_width as f32, self.canvas_height as f32))?;

            let line_coords;
            if self.direction == ScopeDirection::Horizontal {
                line_coords = [[last_point_time_offset, last_point_screen[1]], [point_time_offset, point_screen[1]]];
            }
            else {
                line_coords = [[last_point_screen[0], last_point_time_offset], [point_screen[0], point_time_offset]];
            }
            let line = graphics::Mesh::new_line(ctx, &line_coords, 1., color);
            if let Ok(line) = line {
                graphics::draw(ctx, &line, DrawParam::new())?;
            }
            let point = Mesh::new_rectangle(ctx, DrawMode::fill(), [line_coords[0][0] - 1., line_coords[0][1] - 1., 3., 3.].into(), color)?;
            graphics::draw(ctx, &point, DrawParam::new())?;

            //TODO magic number, where should it come from
            if point_time_offset > 1200. {
                self.scope_offset_old = point_time_offset;
                self.scope_start_time = time;
                let testrect = Mesh::new_rectangle(ctx, DrawMode::fill(), [0., 0., 5., 5.].into(), Color::RED)?;
                std::mem::swap(&mut self.scope_canvas, &mut self.scope_canvas_old);
                graphics::set_canvas(ctx, Some(&self.scope_canvas));
                graphics::clear(ctx, Color::from_rgba(0, 0, 0, 0));
                graphics::draw(ctx, &testrect, DrawParam::new())?;
            }

            crate::reset_graphics(ctx)?;
        }

        self.last_point = Some(ScopePoint { point: new_point, time } );
        Ok(())
    }
    
    fn draw(&self, ctx: &mut Context, x: f32, y: f32) -> GameResult<()> {
        //TODO ability to translate, pass in coords maybe?
        let scale;
        let offset;
        let offset2;
        if self.direction == ScopeDirection::Horizontal {
            scale = [-1., 1.];
            offset = [x + self.scope_offset, y];
            offset2 = [x + self.scope_offset + self.scope_offset_old, y];
        }
        else {
            scale = [1., -1.];
            offset = [x, y + self.scope_offset];
            offset2 = [x, y + self.scope_offset + self.scope_offset_old];
        }
        graphics::draw(ctx, &self.scope_canvas, DrawParam::new().scale(scale).offset(offset))?;
        graphics::draw(ctx, &self.scope_canvas_old, DrawParam::new().scale(scale).offset(offset2))?;
        Ok(())
    }

    fn reset(&mut self, ctx: &mut Context) {
        self.scope_start_time = Instant::now();//this might not work
        self.scope_offset = 0.;
        self.last_point = None;
        graphics::set_canvas(ctx, Some(&self.scope_canvas));
        graphics::clear(ctx, Color::from_rgba(0, 0, 0, 0));
        graphics::set_canvas(ctx, Some(&self.scope_canvas_old));
        graphics::clear(ctx, Color::from_rgba(0, 0, 0, 0));
        graphics::set_canvas(ctx, None);
    }

    fn get_time_from_pos(&mut self, x: f32, y: f32) -> Option<Instant> {
        None
    }
}

fn to_screen_coords(coords: &(i8, i8)) -> [f32; 2] {
    let middlex = 220.;
    let middley = 220.;
    let x = middlex + (coords.0 as f32)*2.;
    let y = middley - (coords.1 as f32)*2.;
    [x, y]
}
