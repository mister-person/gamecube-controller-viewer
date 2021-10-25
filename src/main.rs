use std::ops::Sub;
use std::time::Instant;
use std::{io::Write, time::Duration};

use std::thread;
use std::sync::mpsc::{channel, Sender, Receiver};

use std::collections::{BTreeMap, HashSet, VecDeque};

use rusb::{Device, GlobalContext};

use ggez::{Context, ContextBuilder, GameResult};
use ggez::conf::WindowMode;
use ggez::graphics::{self, BlendMode, Canvas, Color, DrawMode, DrawParam, Drawable, Mesh, MeshBuilder, Rect, get_window_color_format};
use ggez::event::{self, EventHandler};

mod controller;
use controller::Controller;
use controller::update_controllers;

mod zones;

const WIDTH: u16 = 1600;
const HEIGHT: u16 = 1000;

fn main() {

    let (mut ctx, event_loop) = ContextBuilder::new("gc viewer", "mister_person")
        .window_mode(WindowMode::dimensions(Default::default(), WIDTH.into(), HEIGHT.into()))
        .build()
        .expect("aieee, could not create ggez context!");

    let (sender, receiver) = channel::<ControllerPoll>();
    thread::spawn( || start_adapter_polling(sender));

	let my_game = GameState::new(&mut ctx, receiver).unwrap();

	// Run!
	event::run(ctx, event_loop, my_game);
}

struct ScopePoint {
    point: (i8, i8),
    time: Instant,
}
#[derive(PartialEq)]
enum ScopeDirection {
    Vertical,
    Horizontal,
}
struct Oscilliscope {
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

impl Oscilliscope {
    fn new(ctx: &mut Context, width: u16, height: u16, direction: ScopeDirection) -> GameResult<Self> {
        let scope_canvas = Canvas::new(ctx, width, height, ggez::conf::NumSamples::One, get_window_color_format(ctx))?;
        let scope_canvas_old = Canvas::new(ctx, width, height, ggez::conf::NumSamples::One, get_window_color_format(ctx))?;
        Ok(Oscilliscope {
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
    
    fn update(&mut self, ctx: &mut Context, new_point: (i8, i8), time: Instant) -> GameResult<()> {
        fn time_offset(time: Duration) -> f32 {
            time.as_micros() as f32 / 1000.
        }

        //TODO this assumes canvas height
        let point_screen = to_screen_coords(&new_point);
        let point_time_offset = time_offset(time.saturating_duration_since(self.scope_start_time));

        //TODO let people change speed of oscilliscope

        //TODO get rid of all these magic numbers at some point, stop using "WIDTH" where it doesn't belong
        //refactor setting canvas so I don't have to un set it all the time ditto set_screen_coordinates
        //maybe call draw on canvas not on graphics::
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

            graphics::set_canvas(ctx, None);
            graphics::set_screen_coordinates(ctx, Rect::new(0., 0., WIDTH as f32, HEIGHT as f32))?;
        }

        self.last_point = Some(ScopePoint { point: new_point, time } );
        Ok(())
    }
    
    fn draw(&self, ctx: &mut Context) -> GameResult<()> {
        //TODO ability to translate, pass in coords maybe?
        let scale;
        let offset;
        let offset2;
        if self.direction == ScopeDirection::Horizontal {
            scale = [-1., 1.];
            offset = [440. + self.scope_offset, 0.];
            offset2 = [440. + self.scope_offset + self.scope_offset_old, 0.];
        }
        else {
            scale = [1., -1.];
            offset = [0., 440. + self.scope_offset];
            offset2 = [0., 440. + self.scope_offset + self.scope_offset_old];
        }
        graphics::draw(ctx, &self.scope_canvas, DrawParam::new().scale(scale).offset(offset))?;
        graphics::draw(ctx, &self.scope_canvas_old, DrawParam::new().scale(scale).offset(offset2))?;
        Ok(())
    }

    fn reset(&mut self, ctx: &mut Context) {
        self.scope_start_time = Instant::now();//this might not work
        self.scope_offset = 0.;
        graphics::set_canvas(ctx, Some(&self.scope_canvas));
        graphics::clear(ctx, Color::from_rgba(0, 0, 0, 0));
        graphics::set_canvas(ctx, None);
    }
}

struct GameState {
    receiver: Receiver<ControllerPoll>,
    controllers: [Controller; 4],
    current_controller: usize,

    plane: Box<dyn zones::Plane>,
    prev_coords: VecDeque<((i8, i8), Instant)>,
    prev_coords_map: BTreeMap<(i8, i8), u32>,
    oob_coords_map: BTreeMap<(i8, i8), u32>,

    a_press: bool,

    paused: bool,

    background_canvas: Option<Canvas>,
    trail_canvas: Canvas,

    scope_y: Oscilliscope,
    scope_x: Oscilliscope,
}

impl GameState {
    pub fn init_background_canvas(&mut self, ctx: &mut Context) -> GameResult<()> {
        let background_canvas = Canvas::new(ctx, WIDTH, HEIGHT, ggez::conf::NumSamples::One, get_window_color_format(ctx))?;
        graphics::set_canvas(ctx, Some(&background_canvas));
        for x in -80..=80 {
            for y in -80..=80 {
                if controller::clamp(x, y) == (x, y) {
                    let rect = draw_controller_pixel(ctx, &(x, y), Color::from_rgb(0xdd, 0xdd, 0xdd))?;
                    graphics::draw(ctx, &rect, DrawParam::new())?;
                }
            }
        }
        graphics::set_canvas(ctx, None);
        self.background_canvas = Some(background_canvas);
        Ok(())
    }

    pub fn get_background_canvas(&mut self, ctx: &mut Context) -> GameResult<&Canvas> {
        if self.background_canvas.is_none() {
            self.init_background_canvas(ctx)?;
            self.update_background(ctx)?;
        }
        let canvas = self.background_canvas.as_ref().unwrap();
        Ok(canvas)
    }

    pub fn update_background(&mut self, ctx: &mut Context) -> GameResult<()> {
        let canvas = self.get_background_canvas(ctx)?;
        
        graphics::set_canvas(ctx, Some(&canvas));
        for x in -80..=80 {
            for y in -80..=80 {
                if controller::clamp(x, y) == (x, y) {
                    let zone = self.plane.get_zone((x, y));
                    let rect = draw_controller_pixel(ctx, &(x, y), zone.bg_color.into())?;
                    graphics::draw(ctx, &rect, DrawParam::new())?;
                }
            }
        }
        graphics::set_canvas(ctx, None);

        Ok(())
    }

    pub fn get_controller(&self) -> Controller {
        self.controllers[self.current_controller]
    }

    pub fn new(ctx: &mut Context, receiver: Receiver<ControllerPoll>) -> GameResult<GameState> {
        let trail_canvas = Canvas::new(ctx, WIDTH, HEIGHT, ggez::conf::NumSamples::One, get_window_color_format(ctx))?;
        let oscilliscope_y = Oscilliscope::new(ctx, 1500, 440, ScopeDirection::Horizontal)?;
        let oscilliscope_x = Oscilliscope::new(ctx, 400, 1500, ScopeDirection::Vertical)?;
        Ok(GameState {
            receiver,
            controllers: [Controller::new(); 4],
            current_controller: 0,
            plane: Box::new(zones::Test2 {}),
            prev_coords: VecDeque::new(),
            prev_coords_map: BTreeMap::new(),
            oob_coords_map: BTreeMap::new(),
            a_press: false,
            paused: false,
            background_canvas: None,
            trail_canvas,
            scope_y: oscilliscope_y,
            scope_x: oscilliscope_x,
        })
    }
}

impl EventHandler<ggez::GameError> for GameState {
    fn update(&mut self, ctx: &mut Context) -> GameResult<()> {
        // Update code here...
        while let Ok(poll) = self.receiver.try_recv() {
            update_controllers(&mut self.controllers, &poll.buffer);

            for (i, controller) in self.controllers.iter().enumerate() {
                if controller.is_down(&controller::A_BUTTON) && i != self.current_controller {
                    self.current_controller = i;
                }
            }

            //TODO some actual input handling, probably in mod controller
            if self.get_controller().is_down(&controller::A_BUTTON) && !self.a_press {
                self.a_press = true;
                self.update_background(ctx)?;
            }
            if !self.get_controller().is_down(&controller::A_BUTTON) {
                self.a_press = false;
            }
            if self.paused {
                if self.get_controller().just_pressed(&controller::START_BUTTON) {
                    self.paused = false;
                    self.scope_y.reset(ctx);
                    self.scope_x.reset(ctx);
                }
                continue
            }
            else if self.get_controller().just_pressed(&controller::START_BUTTON) {
                self.paused = true;
            }

            let stick_pos = self.get_controller().stick_pos();
            let clamp_pos = controller::clamp(stick_pos.0, stick_pos.1);

            graphics::set_canvas(ctx, Some(&self.trail_canvas));

            self.prev_coords.push_front((stick_pos, poll.time));
            *self.prev_coords_map.entry(clamp_pos).or_insert(0) += 1;

            let zone = self.plane.get_zone(clamp_pos);
            let rect = draw_controller_pixel(ctx, &clamp_pos, zone.fg_color.into())?;
            graphics::draw(ctx, &rect, DrawParam::new())?;

            if clamp_pos != stick_pos {
                *self.oob_coords_map.entry(stick_pos).or_insert(0) += 1;

                let rect = draw_controller_pixel(ctx, &stick_pos, Color::from_rgb(0x40, 0x00, 0x00))?;
                graphics::draw(ctx, &rect, DrawParam::new())?;
            }

            //TODO correlate this size with oscilliscope trail size (maybe)
            if self.prev_coords.len() > 500 {
                let (old_stick_pos, _time) = self.prev_coords.pop_back().expect("I just checked len, it can't be empty");
                let old_clamp_pos = controller::clamp(old_stick_pos.0, old_stick_pos.1);
                let num_points = self.prev_coords_map.entry(old_clamp_pos).or_insert(0);
                *num_points -= 1;
                if *num_points <= 0 {
                    let mut rect = draw_controller_pixel(ctx, &old_clamp_pos, Color::from_rgba(0x00, 0x00, 0x00, 0x00))?;
                    rect.set_blend_mode(Some(BlendMode::Replace));
                    graphics::draw(ctx, &rect, DrawParam::new())?;
                }
                if controller::clamp(old_stick_pos.0, old_stick_pos.1) != old_stick_pos {
                    let num_points = self.oob_coords_map.entry(old_stick_pos).or_insert(0);
                    *num_points -= 1;
                    if *num_points <= 0 {
                        let mut rect = draw_controller_pixel(ctx, &old_stick_pos, Color::from_rgba(0x00, 0x00, 0x00, 0x00))?;
                        rect.set_blend_mode(Some(BlendMode::Replace));
                        graphics::draw(ctx, &rect, DrawParam::new())?;
                    }
                }
            }

            //graphics::clear(ctx, Color::from_rgba(0x00, 0x00, 0x00, 0x00));

            //TODO let people change speed of oscilliscope

            //TODO get rid of all these magic numbers at some point, stop using "WIDTH" where it doesn't belong
            //refactor setting canvas so I don't have to un set it all the time ditto set_screen_coordinates
            //maybe call draw on canvas not on graphics::
            //break up this function it's waay too big

            self.scope_y.update(ctx, clamp_pos, poll.time)?;
            self.scope_x.update(ctx, clamp_pos, poll.time)?;

        }
        Ok(())
    }

    fn draw(&mut self, ctx: &mut Context) -> GameResult<()> {
        graphics::clear(ctx, Color::BLACK);

        let background_canvas = self.get_background_canvas(ctx)?;
        graphics::draw(ctx, background_canvas, DrawParam::new())?;

        graphics::draw(ctx, &self.trail_canvas, DrawParam::new())?;

        self.scope_y.draw(ctx)?;
        self.scope_x.draw(ctx)?;

        let (x, y) = self.get_controller().stick_clamp();
        let coords_text = graphics::Text::new(format!("({:<5}, {:<5})", x, y));
        graphics::draw(ctx, &coords_text, DrawParam::new())?;

        if self.get_controller().stick_clamp() != self.get_controller().stick_pos() {
            let rect = draw_controller_pixel(ctx, &self.get_controller().stick_pos(), Color::RED)?;
            graphics::draw(ctx, &rect, DrawParam::new())?;
        }
        let rect = draw_controller_pixel(ctx, &self.get_controller().stick_clamp(), Color::WHITE)?;
        graphics::draw(ctx, &rect, DrawParam::new())?;

        graphics::present(ctx)
    }
}

fn to_screen_coords(coords: &(i8, i8)) -> [f32; 2] {
    let middlex = 220.;
    let middley = 220.;
    
    let x = middlex + (coords.0 as f32)*2.;
    let y = middley - (coords.1 as f32)*2.;
    [x, y]
}

fn draw_controller_pixel(ctx: &mut Context, coords: &(i8, i8), color: Color) -> GameResult<Mesh> {
    let [x, y] = to_screen_coords(coords);
    let rect = Mesh::new_rectangle(ctx, DrawMode::fill(), [x, y, 2., 2.].into(), color)?;
    Ok(rect)
}

struct ControllerPoll {
    buffer: [u8; 37],
    time: Instant,
}

fn start_adapter_polling(sender: Sender<ControllerPoll>) {
    for device in rusb::devices().unwrap().iter() {
        let device_desc = device.device_descriptor().unwrap();
        println!("Bus {:03} Device {:03} ID {:04x}:{:04x}",
            device.bus_number(),
            device.address(),
            device_desc.vendor_id(),
            device_desc.product_id());

        if is_gc_adapter(&device) {
            println!("found gc adapter");
            match poll_loop(device, sender) {
                Err(x) => {
                    println!("error: {:?}", x);
                    println!("source: {:?}", x.source());
                    println!("description: {:?}", x.to_string());
                },
                _ => {},
            }
            break
        }
    }
}

fn is_gc_adapter(device: &Device<GlobalContext>) -> bool {
    if let Ok (device_desc) = device.device_descriptor() {
        device_desc.vendor_id() == 0x057e && device_desc.product_id() == 0x0337
    }
    else {
        false
    }
}

fn poll_loop(device: Device<GlobalContext>, sender: Sender<ControllerPoll>) -> Result<(), Box<dyn std::error::Error>> {
    device.port_number();
    let handle = device.open()?;
    println!("{:?}", handle);
    let config = device.config_descriptor(0)?;
    let mut endpoint_in = 0;
    let mut endpoint_out = 0;
    for interface in config.interfaces() {
        for descriptor in interface.descriptors() {
            for endpoint_descriptor in descriptor.endpoint_descriptors() {
                if endpoint_descriptor.address() & rusb::constants::LIBUSB_ENDPOINT_IN != 0 {
                    endpoint_in = endpoint_descriptor.address();
                }
                else {
                    endpoint_out = endpoint_descriptor.address();
                }
            }
        }
    }
    handle.write_interrupt(endpoint_out, &[0x13], Duration::from_millis(32))?;
    let mut controllers = [Controller::new(); 4];
    let mut time = Instant::now();
    let mut time_diff = 0;
    let mut last_print = Instant::now();
    loop {
        let mut buffer = [0u8; 37];
        let res = handle.read_interrupt(endpoint_in, &mut buffer, Duration::from_millis(32));
        let now = Instant::now();
        if let Err(res) = res {
            println!("error reading: {:?}", res);
        }
        else {
            sender.send(ControllerPoll { buffer, time: now })?;
        }
        let new_time = Instant::now();
        if new_time - last_print > Duration::from_millis(100) {
            update_controllers(&mut controllers, &buffer);
            print!("\rbtns: [{:<20}] stk: [{:<4?}] c: [{:<4?}]", controllers[0].to_string(), controllers[0].stick_clamp(), controllers[0].c_stick_clamp());
            print!("p2: [{:<20}] stk: [{:<4?}] c: [{:<4?}]", controllers[1].to_string(), controllers[1].stick_clamp(), controllers[1].c_stick_clamp());
            print!("time: {:>4}", time_diff);
            std::io::stdout().flush()?;
            last_print = new_time;
        }

        //sleep(Duration::from_micros(1000));
        time_diff = (new_time - time).as_micros();
        time = new_time;
    }
    //Ok(())
}
