use std::time::Instant;
use std::{io::Write, thread::sleep, time::Duration};

use std::thread;
use std::sync::mpsc::{channel, Sender, Receiver};

use std::collections::VecDeque;

use rusb::{Device, GlobalContext};

use ggez::{Context, ContextBuilder, GameResult};
use ggez::conf::WindowMode;
use ggez::graphics::{self, Canvas, Color, DrawMode, DrawParam, Mesh, MeshBatch, Rect, get_window_color_format};
use ggez::event::{self, EventHandler};

mod controller;
use controller::Controller;
use controller::update_controllers;

const WIDTH: u16 = 700;
const HEIGHT: u16 = 700;

fn main() {

    let (mut ctx, event_loop) = ContextBuilder::new("gc viewer", "mister_person")
        .window_mode(WindowMode::dimensions(Default::default(), WIDTH.into(), HEIGHT.into()))
        .build()
        .expect("aieee, could not create ggez context!");

    let (sender, receiver) = channel::<ControllerPoll>();
    thread::spawn( || start_adapter_polling(sender));

	let my_game = GameState::new(&mut ctx, receiver);

	// Run!
	event::run(ctx, event_loop, my_game);
}

struct GameState {
    receiver: Receiver<ControllerPoll>,
    controllers: [Controller; 4],
    prev_coords: VecDeque<((i8, i8), Instant)>,
    background_canvas: Option<Canvas>,
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
        self.background_canvas = Some(background_canvas);
        Ok(())
    }

    pub fn get_background_canvas(&mut self, ctx: &mut Context) -> GameResult<&Canvas> {
        if self.background_canvas.is_none() {
            self.init_background_canvas(ctx);
        }
        let canvas = self.background_canvas.as_ref().unwrap();
        Ok(canvas)
    }

    pub fn new(ctx: &mut Context, receiver: Receiver<ControllerPoll>) -> GameState {
        GameState {
            receiver,
            controllers: [Controller::new(); 4],
            prev_coords: VecDeque::new(),
            background_canvas: None,
        }
    }
}

impl EventHandler<ggez::GameError> for GameState {
    fn update(&mut self, _ctx: &mut Context) -> GameResult<()> {
        // Update code here...
        while let Ok(poll) = self.receiver.try_recv() {
            update_controllers(&mut self.controllers, &poll.buffer);
            self.prev_coords.push_front((self.controllers[0].stick_clamp(), poll.time));
            if self.prev_coords.len() > 1000 {
                self.prev_coords.pop_back();
            }
        }
        Ok(())
    }

    fn draw(&mut self, ctx: &mut Context) -> GameResult<()> {
        graphics::clear(ctx, Color::WHITE);

        let canvas = Canvas::new(ctx, WIDTH, HEIGHT, ggez::conf::NumSamples::One, get_window_color_format(ctx))?;
        graphics::set_canvas(ctx, Some(&canvas));

        let rect = Mesh::new_rectangle(ctx, DrawMode::fill(), [5., 5., 30., 30.].into(), Color::BLUE)?;
        graphics::draw(ctx, &rect, DrawParam::new())?;

        for (coord, _time) in self.prev_coords.iter() {
            let rect = draw_controller_pixel(ctx, coord, Color::from_rgb(0x88, 0x88, 0xff))?;
            graphics::draw(ctx, &rect, DrawParam::new())?;
        }

        if self.controllers[0].stick_clamp() != self.controllers[0].stick_pos() {
            let rect = draw_controller_pixel(ctx, &self.controllers[0].stick_pos(), Color::RED)?;
            graphics::draw(ctx, &rect, DrawParam::new())?;
        }

        let rect = draw_controller_pixel(ctx, &self.controllers[0].stick_clamp(), Color::BLACK)?;
        graphics::draw(ctx, &rect, DrawParam::new())?;

        graphics::set_canvas(ctx, None);

        let background_canvas = self.get_background_canvas(ctx)?;
        graphics::draw(ctx, background_canvas, DrawParam::new())?;

        graphics::draw(ctx, &canvas, DrawParam::new())?;

        graphics::present(ctx)
    }
}

fn draw_controller_pixel(ctx: &mut Context, coords: &(i8, i8), color: Color) -> GameResult<Mesh> {
    let middlex = 350.;
    let middley = 350.;

    let x = middlex + (coords.0 as f32)*3.;
    let y = middley - (coords.1 as f32)*3.;
    let rect = Mesh::new_rectangle(ctx, DrawMode::fill(), [x, y, 3., 3.].into(), color)?;
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
        //update_controllers(&mut controllers, &buffer);
        let new_time = Instant::now();
        if new_time - last_print > Duration::from_millis(100) {
            print!("\rbuttons: [{:<30}] stick: [{:<4?}] c: [{:<4?}]", controllers[0].to_string(), controllers[0].stick_clamp(), controllers[0].c_stick_clamp());
            print!("p2: [{:<30}] stick: [{:<4?}] c: [{:<4?}]", controllers[1].to_string(), controllers[1].stick_clamp(), controllers[1].c_stick_clamp());
            print!("time: {}", time_diff);
            std::io::stdout().flush()?;
            last_print = new_time;
        }

        //sleep(Duration::from_micros(1000));
        time_diff = (new_time - time).as_micros();
        time = new_time;
    }
    //Ok(())
}
