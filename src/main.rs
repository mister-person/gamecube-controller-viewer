use std::time::Instant;

use std::thread;
use std::sync::mpsc::{channel, Receiver};

use std::collections::VecDeque;

use ggez::{Context, ContextBuilder, GameResult};
use ggez::conf::WindowMode;
use ggez::graphics::{self, Color, DrawParam, Rect};
use ggez::event::{self, EventHandler};

mod controller;
use controller::Controller;
use controller::update_controllers;

mod zones;

mod oscilloscope;
use oscilloscope::Oscilloscope;
use oscilloscope::ScopeDirection;

mod gc_adapter;
use gc_adapter::ControllerPoll;
use gc_adapter::start_adapter_polling;

mod stick_display;
use stick_display::StickDisplay;

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

enum StickPosFormat {
    Integer,
    Decimal,
}

struct GameState {
    receiver: Receiver<ControllerPoll>,
    controllers: [Controller; 4],
    current_controller: usize,

    prev_coords: VecDeque<((i8, i8), Instant)>,
    c_prev_coords: VecDeque<((i8, i8), Instant)>,

    paused: bool,

    stick_display: StickDisplay,
    c_stick_display: StickDisplay,

    scope_y: Oscilloscope,
    scope_x: Oscilloscope,

    stick_pos_format: StickPosFormat,
}

impl GameState {
    pub fn get_controller(&self) -> Controller {
        self.controllers[self.current_controller]
    }

    pub fn new(ctx: &mut Context, receiver: Receiver<ControllerPoll>) -> GameResult<GameState> {
        let oscilloscope_y = Oscilloscope::new(ctx, 1500, 440, ScopeDirection::Horizontal)?;
        let oscilloscope_x = Oscilloscope::new(ctx, 400, 1500, ScopeDirection::Vertical)?;
        let mut c_stick_display = StickDisplay::new(ctx, 220, 220)?;
        c_stick_display.set_plane(Box::new(zones::CStick {}));
        Ok(GameState {
            receiver,
            controllers: [Controller::new(); 4],
            current_controller: 0,
            prev_coords: VecDeque::new(),
            c_prev_coords: VecDeque::new(),
            paused: false,
            scope_y: oscilloscope_y,
            scope_x: oscilloscope_x,
            stick_display: StickDisplay::new(ctx, 440, 440)?,
            c_stick_display,
            stick_pos_format: StickPosFormat::Integer,
        })
    }
}

impl EventHandler<ggez::GameError> for GameState {
    fn update(&mut self, ctx: &mut Context) -> GameResult<()> {
        loop {
            let poll = match self.receiver.try_recv() {
                Ok(poll) => poll,
                Err(std::sync::mpsc::TryRecvError::Empty) => break,
                Err(std::sync::mpsc::TryRecvError::Disconnected) => panic!("couldn't read from adapter"),
            };
            update_controllers(&mut self.controllers, &poll.buffer);

            for (i, controller) in self.controllers.iter().enumerate() {
                if controller.is_down(&controller::A_BUTTON) && i != self.current_controller {
                    self.current_controller = i;
                }
            }

            if self.get_controller().just_pressed(&controller::A_BUTTON) {
                //self.update_background(ctx)?;
            }
            if self.get_controller().just_pressed(&controller::D_RIGHT_BUTTON) {
                self.stick_pos_format = match self.stick_pos_format {
                    StickPosFormat::Integer => StickPosFormat::Decimal,
                    StickPosFormat::Decimal => StickPosFormat::Integer,
                }
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
            self.prev_coords.push_front((stick_pos, poll.time));
            self.stick_display.add_point(ctx, clamp_pos)?;
            if clamp_pos != stick_pos {
                self.stick_display.add_point(ctx, stick_pos)?;
            }

            //TODO correlate this size with oscilloscope trail size (maybe)
            if self.prev_coords.len() > 500 {
                let (old_stick_pos, _time) = self.prev_coords.pop_back().expect("I just checked len, it can't be empty");
                let old_clamp_pos = controller::clamp(old_stick_pos.0, old_stick_pos.1);
                self.stick_display.remove_point(ctx, old_stick_pos)?;
                if old_clamp_pos != old_stick_pos {
                    self.stick_display.remove_point(ctx, old_clamp_pos)?;
                }
            }

            let c_stick_pos = self.get_controller().c_stick_pos();
            let c_clamp_pos = controller::clamp(c_stick_pos.0, c_stick_pos.1);
            self.c_prev_coords.push_front((c_stick_pos, poll.time));
            self.c_stick_display.add_point(ctx, c_clamp_pos)?;
            if c_clamp_pos != c_stick_pos {
                self.c_stick_display.add_point(ctx, c_stick_pos)?;
            }

            //TODO correlate this size with oscilloscope trail size (maybe)
            if self.c_prev_coords.len() > 500 {
                let (old_c_stick_pos, _time) = self.c_prev_coords.pop_back().expect("I just checked len, it can't be empty");
                let old_c_clamp_pos = controller::clamp(old_c_stick_pos.0, old_c_stick_pos.1);
                self.c_stick_display.remove_point(ctx, old_c_stick_pos)?;
                if old_c_clamp_pos != old_c_stick_pos {
                    self.c_stick_display.remove_point(ctx, old_c_clamp_pos)?;
                }
            }

            self.scope_y.update(ctx, clamp_pos, poll.time)?;
            self.scope_x.update(ctx, clamp_pos, poll.time)?;

        }
        Ok(())
    }

    fn draw(&mut self, ctx: &mut Context) -> GameResult<()> {
        graphics::clear(ctx, Color::BLACK);

        self.stick_display.draw(ctx, 0., 0.)?;

        if self.get_controller().stick_clamp() != self.get_controller().stick_pos() {
            self.stick_display.draw_point(ctx, self.get_controller().stick_pos(), Color::RED)?;
        }
        self.stick_display.draw_point(ctx, self.get_controller().stick_clamp(), Color::WHITE)?;

        self.c_stick_display.draw(ctx, 400., 400.)?;

        self.scope_y.draw(ctx)?;
        self.scope_x.draw(ctx)?;

        let (x, y) = self.get_controller().stick_clamp();
        let (x, y) = match self.stick_pos_format {
            StickPosFormat::Integer => (x as f64, y as f64),
            StickPosFormat::Decimal => (x as f64 / 80., y as f64 / 80.),
        };
        let coords_text = graphics::Text::new(format!("({:<7}, {:<7})", x, y));
        graphics::draw(ctx, &coords_text, DrawParam::new())?;

        let (real_x, real_y) = self.get_controller().stick_pos();
        let (real_x, real_y) = match self.stick_pos_format {
            StickPosFormat::Integer => (real_x as f64, real_y as f64),
            StickPosFormat::Decimal => (real_x as f64 / 80., real_y as f64 / 80.),
        };
        if (real_x, real_y) != (x, y) {
            let coords_text = graphics::Text::new(format!("({:<7}, {:<7})", real_x, real_y));
            graphics::draw(ctx, &coords_text, DrawParam::new().dest([0., 15.]).color(Color::RED))?;
        }

        graphics::set_screen_coordinates(ctx, Rect::new(0., 0., WIDTH as f32, HEIGHT as f32))?;
        let (x, y) = self.get_controller().c_stick_clamp();
        let (x, y) = match self.stick_pos_format {
            StickPosFormat::Integer => (x as f64, y as f64),
            StickPosFormat::Decimal => (x as f64 / 80., y as f64 / 80.),
        };
        let coords_text = graphics::Text::new(format!("({:<7}, {:<7})", x, y));
        graphics::draw(ctx, &coords_text, DrawParam::new().dest([400., 400.]).color(Color::from_rgb(0xff, 0xff, 0x00)))?;

        let (real_x, real_y) = self.get_controller().c_stick_pos();
        let (real_x, real_y) = match self.stick_pos_format {
            StickPosFormat::Integer => (real_x as f64, real_y as f64),
            StickPosFormat::Decimal => (real_x as f64 / 80., real_y as f64 / 80.),
        };
        if (real_x, real_y) != (x, y) {
            let coords_text = graphics::Text::new(format!("({:<7}, {:<7})", real_x, real_y));
            graphics::draw(ctx, &coords_text, DrawParam::new().dest([400., 415.]).color(Color::from_rgb(0xc0, 0xc0, 0x00)))?;
        }

        let (raw_x, raw_y) = self.get_controller().stick_raw();
        let coords_text = graphics::Text::new(format!("({:<5}, {:<5})", raw_x, raw_y));
        graphics::draw(ctx, &coords_text, DrawParam::new().dest([0., 30.]).color(Color::BLUE))?;

        graphics::present(ctx)
    }
}

fn reset_graphics(ctx: &mut Context) -> GameResult<()> {
    graphics::set_canvas(ctx, None);
    graphics::set_screen_coordinates(ctx, Rect::new(0., 0., WIDTH as f32, HEIGHT as f32))?;
    Ok(())
}
