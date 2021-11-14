use std::{collections::BTreeMap, time::{Duration, Instant}};

use ggez::{Context, GameResult, graphics::{self, BlendMode, Canvas, Color, DrawMode, DrawParam, Drawable, Mesh, Rect, get_window_color_format}};

use crate::{controller, reset_graphics, zones};

pub struct StickDisplay {
    pub plane: Box<dyn zones::Plane>,

    pub background_canvas: Canvas,
    pub trail_canvas: Canvas,

    background_progress_x: i8,
    background_updated: bool,

    x: f32,
    y: f32,
    width: f32,
    height: f32,

    pub prev_coords_counter: BTreeMap<(i8, i8), u32>,
}

impl StickDisplay {
    pub fn new(ctx: &mut Context, x: f32, y: f32, width: u16, height: u16) -> GameResult<Self> {
        let background_canvas = Canvas::new(ctx, width, height, ggez::conf::NumSamples::One, get_window_color_format(ctx))?;
        let trail_canvas = Canvas::new(ctx, width, height, ggez::conf::NumSamples::One, get_window_color_format(ctx))?;
        let plane = Box::new(zones::PlaneWithZones::default_plane());
        Ok(Self {
            plane,
            background_canvas,
            trail_canvas,
            prev_coords_counter: BTreeMap::new(),
            background_progress_x: -80,
            background_updated: false,
            width: width as f32,
            height: height as f32,
            x,
            y,
        })
    }

    pub fn update_background(&mut self, ctx: &mut Context) -> GameResult<()> {
        graphics::set_screen_coordinates(ctx, Rect::new(0., 0., self.width as f32, self.height as f32))?;
        graphics::set_canvas(ctx, Some(&self.background_canvas));
        let start_time = Instant::now();
        for x in self.background_progress_x..=80 {
            for y in -80..=80 {
                if controller::clamp(x, y) == (x, y) {
                    let color = self.plane.get_zone((x, y)).bg_color.into();
                    //let color = Color::GREEN;
                    let rect = self.draw_controller_pixel(ctx, &(x, y), color)?;
                    graphics::draw(ctx, &rect, DrawParam::new())?;
                }

            }

            //small hack to not draw it all at once, apparently drawing
            //lots of squares is slooow
            if Instant::now() - start_time > Duration::from_millis(3) {
                self.background_progress_x = x;//off by one?
                reset_graphics(ctx)?;
                return Ok(())
            }
        }
        self.background_updated = true;
        reset_graphics(ctx)?;
        Ok(())
    }

    pub fn set_plane(&mut self, plane: Box<dyn zones::Plane>) {
        self.plane = plane;
        self.background_updated = false;
    }

    pub fn add_point(&mut self, ctx: &mut Context, point: (i8, i8)) -> GameResult<()> {
        graphics::set_canvas(ctx, Some(&self.trail_canvas));
        graphics::set_screen_coordinates(ctx, Rect::new(0., 0., self.width as f32, self.height as f32))?;
        
        *self.prev_coords_counter.entry(point).or_insert(0) += 1;

        let zone = self.plane.get_zone(point);
        let rect = self.draw_controller_pixel(ctx, &point, zone.fg_color.into())?;
        graphics::draw(ctx, &rect, DrawParam::new())?;

        reset_graphics(ctx)?;
        
        Ok(())
    }

    pub fn remove_point(&mut self, ctx: &mut Context, point: (i8, i8)) -> GameResult<()> {
        graphics::set_canvas(ctx, Some(&self.trail_canvas));
        graphics::set_screen_coordinates(ctx, Rect::new(0., 0., self.width as f32, self.height as f32))?;
        let num_points = self.prev_coords_counter.entry(point).or_insert(0);
        *num_points -= 1;
        if *num_points <= 0 {
            let mut rect = self.draw_controller_pixel(ctx, &point, Color::from_rgba(0x00, 0x00, 0x00, 0x00))?;
            rect.set_blend_mode(Some(BlendMode::Replace));
            graphics::draw(ctx, &rect, DrawParam::new())?;
        }
        reset_graphics(ctx)?;
        Ok(())
    }

    pub fn draw(&mut self, ctx: &mut Context) -> GameResult<()> {
        if !self.background_updated {
            self.update_background(ctx)?;
        }
        let offset_param = DrawParam::new().dest([self.x, self.y]);
        graphics::draw(ctx, &self.background_canvas, offset_param)?;
        graphics::draw(ctx, &self.trail_canvas, offset_param)?;
        Ok(())
    }

    pub fn draw_point(&self, ctx: &mut Context, point: (i8, i8), color: Color) -> GameResult<()> {

        let rect = self.draw_controller_pixel(ctx, &point, color)?;
        graphics::draw(ctx, &rect, DrawParam::new())?;

        Ok(())
    }

    fn to_screen_coords(&self, coords: &(i8, i8)) -> [f32; 2] {
        let middlex = self.width/2.;
        let middley = self.height/2.;
        let scale_x = self.width/220.;
        let scale_y = self.height/220.;
        let x = middlex + (coords.0 as f32)*scale_x;
        let y = middley - (coords.1 as f32)*scale_y;
        [x, y]
    }

    fn draw_controller_pixel(&self, ctx: &mut Context, coords: &(i8, i8), color: Color) -> GameResult<Mesh> {
        let [x, y] = self.to_screen_coords(coords);
        let scale_x = self.width/220.;
        let scale_y = self.height/220.;
        let rect = Mesh::new_rectangle(ctx, DrawMode::fill(), [x, y, scale_x, scale_y].into(), color)?;
        Ok(rect)
    }
}
