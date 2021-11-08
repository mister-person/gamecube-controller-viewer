use std::convert::TryInto;
use std::time::{Duration, Instant};

use std::thread;
use std::sync::mpsc::{channel, Receiver};

use std::collections::{BTreeMap, VecDeque};

use ggez::input::mouse;
use ggez::{Context, ContextBuilder, GameResult};
use ggez::conf::WindowMode;
use ggez::graphics::{self, Color, DrawParam, Rect, Text, TextFragment};
use ggez::event::{self, EventHandler};

mod controller;
use controller::Controller;
use controller::update_controllers;

mod zones;

mod oscilloscope;
use input_sequence::InputSequenceState;
use oscilloscope::Oscilloscope;
use oscilloscope::ScopeDirection;

mod gc_adapter;
use gc_adapter::ControllerPoll;
use gc_adapter::start_adapter_polling;

mod stick_display;
use stick_display::StickDisplay;

mod button_display;

use crate::input_sequence::InputSequence;
use crate::oscilloscope::Scope;

mod button_scope;
use button_scope::ButtonScope;

mod input_sequence;

const WIDTH: u16 = 1600;
const HEIGHT: u16 = 1000;

fn main() {
    let (mut ctx, event_loop) = ContextBuilder::new("gc viewer", "mister_person")
        .window_mode(WindowMode::dimensions(Default::default(), WIDTH.into(), HEIGHT.into()))
        .build()
        .expect("aieee, could not create ggez context!");

    let (sender, receiver) = channel::<ControllerPoll>();
    thread::spawn( || start_adapter_polling(sender));

    let input_sequences = Box::new(input_sequence::make_some_sequences()).leak();
	let mut my_game = GameState::new(&mut ctx, receiver).unwrap();
    my_game.input_sequences_states = Some(InputSequenceState::new(&input_sequences[3]));

	// Run!
	event::run(ctx, event_loop, my_game);
}

enum StickPosFormat {
    Integer,
    Decimal,
}

struct GameState<'a> {
    receiver: Receiver<ControllerPoll>,
    controllers: [Controller; 4],
    current_controller: usize,

    prev_coords: VecDeque<((i8, i8), Instant)>,
    c_prev_coords: VecDeque<((i8, i8), Instant)>,

    prev_input_map: BTreeMap<Instant, [u8; 8]>,

    paused: bool,

    stick_display: StickDisplay,
    c_stick_display: StickDisplay,

    scope_y: Oscilloscope,
    scope_x: Oscilloscope,

    button_scope: ButtonScope,

    stick_pos_format: StickPosFormat,

    input_sequences_states: Option<input_sequence::InputSequenceState<'a>>,
}

impl<'a> GameState<'a> {
    pub fn get_controller(&self) -> Controller {
        self.controllers[self.current_controller]
    }

    pub fn get_inputs_at_time(&self, time: Instant) -> [u8; 8] {
        if let Some(inputs) = self.prev_input_map.range(time..).next() {
            inputs.1.to_owned()
        }
        else {
            [0; 8]
        }
    }

    pub fn new<'b>(ctx: &'b mut Context, receiver: Receiver<ControllerPoll>) -> GameResult<GameState<'a>> {
        let oscilloscope_y = Oscilloscope::new(ctx, 440., 0., 1500., 400., ScopeDirection::Horizontal)?;
        let oscilloscope_x = Oscilloscope::new(ctx, 0., 440., 400., 1500., ScopeDirection::Vertical)?;
        let button_scope = ButtonScope::new(ctx, 440., 660., 1000., 180., ScopeDirection::Horizontal)?;
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
            button_scope,
            prev_input_map: BTreeMap::new(),
            input_sequences_states: None,
        })
    }
}

impl<'a> EventHandler<ggez::GameError> for GameState<'a> {
    fn update(&mut self, ctx: &mut Context) -> GameResult<()> {
        loop {
            let poll = match self.receiver.try_recv() {
                Ok(poll) => poll,
                Err(std::sync::mpsc::TryRecvError::Empty) => break,
                Err(std::sync::mpsc::TryRecvError::Disconnected) => panic!("couldn't read from adapter"),
            };
            update_controllers(&mut self.controllers, &poll.buffer);

            if !self.paused {
                self.prev_input_map.insert(poll.time, self.get_controller().buffer);
                if self.prev_input_map.len() > 5000 {//TODO magic number
                    let (first, _) = self.prev_input_map.iter().next().unwrap();
                    let first = first.clone();
                    self.prev_input_map.remove(&first);
                }
            }

            //push the A button on a controller to switch to it
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
            //pause if start is pressed
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

            let buttons = self.get_controller().buffer[0..2].try_into();
            if let Ok(buttons) = buttons {
                self.button_scope.update(ctx, buttons, poll.time)?;
            }

            let controller = self.get_controller();
            for button in controller.buttons_just_pressed() {
                self.input_sequences_states.as_mut().unwrap().action(input_sequence::ControllerAction::Press(*button), &controller, poll.time);
            }
            for button in controller.buttons_just_released() {
                self.input_sequences_states.as_mut().unwrap().action(input_sequence::ControllerAction::Release(*button), &controller, poll.time);
            }

            //add trail points to stick display
            let stick_pos = self.get_controller().stick_pos();
            let clamp_pos = controller::clamp(stick_pos.0, stick_pos.1);
            self.prev_coords.push_front((stick_pos, poll.time));
            self.stick_display.add_point(ctx, clamp_pos)?;
            if clamp_pos != stick_pos {
                self.stick_display.add_point(ctx, stick_pos)?;
            }

            //TODO correlate this size with oscilloscope trail size (maybe)
            //remove old points from stick display
            if self.prev_coords.len() > 500 {
                let (old_stick_pos, _time) = self.prev_coords.pop_back().expect("I just checked len, it can't be empty");
                let old_clamp_pos = controller::clamp(old_stick_pos.0, old_stick_pos.1);
                self.stick_display.remove_point(ctx, old_stick_pos)?;
                if old_clamp_pos != old_stick_pos {
                    self.stick_display.remove_point(ctx, old_clamp_pos)?;
                }
            }

            //ditto above but c stick, TODO dry
            let c_stick_pos = self.get_controller().c_stick_pos();
            let c_clamp_pos = controller::clamp(c_stick_pos.0, c_stick_pos.1);
            self.c_prev_coords.push_front((c_stick_pos, poll.time));
            self.c_stick_display.add_point(ctx, c_clamp_pos)?;
            if c_clamp_pos != c_stick_pos {
                self.c_stick_display.add_point(ctx, c_stick_pos)?;
            }

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

        self.button_scope.draw(ctx)?;

        button_display::draw_buttons(ctx, &self.get_controller(), 410., 660., button_display::LINE_LAYOUT)?;

        self.scope_y.draw(ctx)?;
        self.scope_x.draw(ctx)?;

        let mouse_pos = mouse::position(ctx);
        let mut instant = None;
        instant = instant.or_else(|| self.scope_y.get_time_from_pos(mouse_pos.x, mouse_pos.y));
        instant = instant.or_else(|| self.scope_x.get_time_from_pos(mouse_pos.x, mouse_pos.y));
        instant = instant.or_else(|| self.button_scope.get_time_from_pos(mouse_pos.x, mouse_pos.y));
        if let Some(instant) = instant {
            let text = self.scope_y.scope_start_time.saturating_duration_since(instant).as_millis().to_string();
            draw_text(ctx, text, 200., 0., Color::WHITE)?;
            self.scope_y.draw_line_at_time(ctx, instant)?;
            self.scope_x.draw_line_at_time(ctx, instant)?;
            self.button_scope.draw_line_at_time(ctx, instant)?;

            let mut controller = self.get_controller().clone();
            controller.from_buffer(&self.get_inputs_at_time(instant));
            let point = controller.stick_clamp();
            for x in -1..=1 {
                for y in -1..=1 {
                    let color = if x == 0 && y == 0 {Color::BLACK} else {Color::WHITE};
                    self.stick_display.draw_point(ctx, (point.0 + x, point.1 + y), color)?;
                }
            }
        }

        let get_text_from_coords = |x, y| {
            let (x, y) = match self.stick_pos_format {
                StickPosFormat::Integer => (x as f64, y as f64),
                StickPosFormat::Decimal => (x as f64 / 80., y as f64 / 80.),
            };
            let mx = if x < 0.0 {'-'} else {' '};
            let my = if y < 0.0 {'-'} else {' '};
            format!("({}{:<6}, {}{:<6})", mx, x.abs(), my, y.abs())
        };

        let (x, y) = self.get_controller().stick_clamp();
        let (real_x, real_y) = self.get_controller().stick_pos();
        draw_text(ctx, get_text_from_coords(x, y), 0., 0., Color::WHITE)?;
        if (real_x, real_y) != (x, y) {
            draw_text(ctx, get_text_from_coords(real_x, real_y), 0., 15., Color::RED)?;
        }

        let (raw_x, raw_y) = self.get_controller().stick_raw();
        draw_text(ctx, format!("({:<5}, {:<5})", raw_x, raw_y), 0., 30., Color::BLUE)?;

        let (c_x, c_y) = self.get_controller().c_stick_clamp();
        let (real_c_x, real_c_y) = self.get_controller().c_stick_pos();
        draw_text(ctx, get_text_from_coords(c_x, c_y), 400., 400., Color::from_rgb(0xff, 0xff, 0x00))?;
        if (real_c_x, real_c_y) != (c_x, c_y) {
            draw_text(ctx, get_text_from_coords(real_c_x, real_c_y), 400., 415., Color::from_rgb(0xc0, 0xc0, 0x00))?;
        }

        draw_text(ctx, format!("(fpx: {})", ggez::timer::fps(ctx)), 250., 0., Color::WHITE)?;

        if let Some(sequence_states) = &self.input_sequences_states {
            if let Some(sequence) = sequence_states.completed_sequence() {
                let mut start = None;
                for (i, (input, time)) in sequence.iter().enumerate() {
                    if start == None {
                        start = Some(time)
                    }
                    let since_start = *time - *start.unwrap();
                    draw_text(ctx, format!("{}, time: {:.3} frames ({} ms)", input.to_string(), duration_to_frame_count(since_start), since_start.as_millis()), 400., 850. + (15*i) as f32, Color::CYAN)?;
                }
                draw_text(ctx, format!("chance of success: {}%", sequence_states.success_rate().unwrap() * 100.), 400., 850. + (15*sequence.len()) as f32, Color::CYAN)?;
            }
            else {
                draw_text(ctx, format!(": {}", sequence_states.state), 400., 850., Color::CYAN)?;
            }
        }

        graphics::present(ctx)
    }
}

fn duration_to_frame_count(duration: Duration) -> f64 {
    return duration.as_micros() as f64 / (1_000_000. / 60.)
}

fn draw_text<F>(ctx: &mut Context, text: F, x: f32, y: f32, color: Color) -> GameResult<()>
where F: Into<TextFragment>
{
    let coords_text = TextFragment::new(text).color(color);
    graphics::draw(ctx, &Text::new(coords_text), DrawParam::new().dest([x, y]))?;
    Ok(())
}

fn reset_graphics(ctx: &mut Context) -> GameResult<()> {
    graphics::set_canvas(ctx, None);
    graphics::set_screen_coordinates(ctx, Rect::new(0., 0., WIDTH as f32, HEIGHT as f32))?;
    Ok(())
}
