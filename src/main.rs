use std::convert::TryInto;
use std::time::{Duration, Instant};

use std::thread;
use std::sync::mpsc::{channel, Receiver};

use std::collections::{BTreeMap, HashMap, VecDeque};

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
use zones::Zone;

mod button_display;

use crate::input_sequence::{ActionSuccess, ControllerAction, InputSequence};
use crate::oscilloscope::Scope;
use crate::zones::ZoneTrait;

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
    my_game.input_sequences_states = input_sequences.iter().map(|seq| InputSequenceState::new(seq)).collect();

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
    c_scope_y: Oscilloscope,
    c_scope_x: Oscilloscope,

    button_scope: ButtonScope,

    stick_pos_format: StickPosFormat,

    input_sequences_states: Vec<input_sequence::InputSequenceState<'a>>,
    completed_sequences: Vec<(Vec<(ControllerAction, Duration, ActionSuccess)>, f64)>,
    used_zones: Vec<(&'a Zone, bool)>,
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
        let scope_y = Oscilloscope::new(ctx, 440., 0., 1500., 200., ScopeDirection::Horizontal)?;
        let scope_x = Oscilloscope::new(ctx, 440., 200., 1500., 200., ScopeDirection::Horizontal)?;
        let mut c_scope_y = Oscilloscope::new(ctx, 440., 400., 1500., 130., ScopeDirection::Horizontal)?;
        let mut c_scope_x = Oscilloscope::new(ctx, 440., 530., 1500., 130., ScopeDirection::Horizontal)?;
        c_scope_y.plane = Box::new(zones::CStick{});
        c_scope_x.plane = Box::new(zones::CStick{});
        let button_scope = ButtonScope::new(ctx, 440., 660., 1000., 180., ScopeDirection::Horizontal)?;
        let stick_display = StickDisplay::new(ctx, 0., 0., 440, 440)?;
        let mut c_stick_display = StickDisplay::new(ctx, 0., 400., 220, 220)?;
        c_stick_display.set_plane(Box::new(zones::CStick {}));
        let used_zones = zones::get_some_zones().iter().map(|z| (*z, false)).collect();
        Ok(GameState {
            receiver,
            controllers: [Controller::new(); 4],
            current_controller: 0,
            prev_coords: VecDeque::new(),
            c_prev_coords: VecDeque::new(),
            paused: false,
            scope_y,
            scope_x,
            c_scope_y,
            c_scope_x,
            stick_display,
            c_stick_display,
            stick_pos_format: StickPosFormat::Integer,
            button_scope,
            prev_input_map: BTreeMap::new(),
            input_sequences_states: vec![],
            used_zones,
            completed_sequences: vec![],
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
                    self.c_scope_y.reset(ctx);
                    self.c_scope_x.reset(ctx);
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

            let stick_pos = self.get_controller().stick_pos();
            let clamp_pos = controller::clamp(stick_pos.0, stick_pos.1);

            let controller = self.get_controller();

            let mut actions = vec![];
            actions.extend(
                controller.buttons_just_pressed().iter().map(|button| ControllerAction::Press(**button))
            );
            actions.extend(
                controller.buttons_just_released().iter().map(|button| ControllerAction::Release(**button))
            );
            for (i, (zone, in_last)) in self.used_zones.clone().iter().enumerate() {
                let in_zone = zone.in_zone(clamp_pos);
                if in_zone && !in_last {
                    actions.push(ControllerAction::Enter((**zone).clone()));
                }
                if !in_zone && *in_last {
                    actions.push(ControllerAction::Leave((**zone).clone()));
                }
                self.used_zones[i].1 = in_zone;
            }
            for seq in self.input_sequences_states.iter_mut() {
                for action in &actions {
                    let finished = seq.action(action.clone(), &controller, poll.time);
                    if finished {
                        if let Some(actions) = seq.sequence_info() {
                            let rate = seq.success_rate().unwrap_or(0.);
                            self.completed_sequences.push((actions, rate));
                        }
                    }
                }
            };

            //add trail points to stick display
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

            self.scope_x.update(ctx, (clamp_pos.0, clamp_pos), poll.time)?;
            self.scope_y.update(ctx, (clamp_pos.1, clamp_pos), poll.time)?;
            self.c_scope_x.update(ctx, (c_clamp_pos.0, c_clamp_pos), poll.time)?;
            self.c_scope_y.update(ctx, (c_clamp_pos.1, c_clamp_pos), poll.time)?;

        }
        Ok(())
    }

    fn draw(&mut self, ctx: &mut Context) -> GameResult<()> {
        graphics::clear(ctx, Color::BLACK);

        self.stick_display.draw(ctx)?;

        if self.get_controller().stick_clamp() != self.get_controller().stick_pos() {
            self.stick_display.draw_point(ctx, self.get_controller().stick_pos(), Color::RED)?;
        }
        self.stick_display.draw_point(ctx, self.get_controller().stick_clamp(), Color::WHITE)?;

        self.c_stick_display.draw(ctx)?;

        self.button_scope.draw(ctx)?;

        button_display::draw_buttons(ctx, &self.get_controller(), 410., 660., button_display::LINE_LAYOUT)?;

        self.scope_y.draw(ctx)?;
        self.scope_x.draw(ctx)?;
        self.c_scope_y.draw(ctx)?;
        self.c_scope_x.draw(ctx)?;

        let mouse_pos = mouse::position(ctx);
        let mut instant = None;
        instant = instant.or_else(|| self.scope_y.get_time_from_pos(mouse_pos.x, mouse_pos.y));
        instant = instant.or_else(|| self.scope_x.get_time_from_pos(mouse_pos.x, mouse_pos.y));
        instant = instant.or_else(|| self.c_scope_y.get_time_from_pos(mouse_pos.x, mouse_pos.y));
        instant = instant.or_else(|| self.c_scope_x.get_time_from_pos(mouse_pos.x, mouse_pos.y));
        instant = instant.or_else(|| self.button_scope.get_time_from_pos(mouse_pos.x, mouse_pos.y));
        if let Some(instant) = instant {
            let text = self.scope_y.scope_start_time.saturating_duration_since(instant).as_millis().to_string();
            draw_text(ctx, text, 200., 0., Color::WHITE)?;
            self.scope_y.draw_line_at_time(ctx, instant)?;
            self.scope_x.draw_line_at_time(ctx, instant)?;
            self.c_scope_y.draw_line_at_time(ctx, instant)?;
            self.c_scope_x.draw_line_at_time(ctx, instant)?;
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

        if let Some((actions, success_rate)) = self.completed_sequences.get(self.completed_sequences.len() - 1) {
            for (i, (input, since_last, success)) in actions.iter().enumerate() {
                let color = match success {
                    ActionSuccess::EarlyMiss => Color::from_rgb(255, 0, 128),
                    ActionSuccess::Early => Color::MAGENTA,
                    ActionSuccess::Success => Color::CYAN,
                    ActionSuccess::Late => Color::from_rgb(255, 127, 0),
                    ActionSuccess::LateMiss => Color::RED,
                };
                draw_text(ctx, format!("{}, time: {:.3} frames ({} ms) {}", input.to_string(), duration_to_frame_count(*since_last), since_last.as_millis(), success), 400., 850. + (15*i) as f32, color)?;
            }
            draw_text(ctx, format!("chance of success: {}%", success_rate * 100.), 400., 850. + (15*actions.len()) as f32, Color::CYAN)?;
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
