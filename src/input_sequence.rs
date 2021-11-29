use std::{fmt::Display, ops::{Range}, time::{Duration, Instant}};

use crate::{controller::{A_BUTTON, B_BUTTON, Button, Controller, X_BUTTON, Y_BUTTON}, duration_to_frame_count, zones::{self, Zone, ZoneTrait}};

#[derive(Debug, PartialEq, Clone)]
pub enum ControllerAction {
    Nothing,
    Press(Button),
    Release(Button),
    Enter(Zone),
    Leave(Zone),
}

impl ToString for ControllerAction {
    fn to_string(&self) -> String {
        match self {
            ControllerAction::Nothing => "nothing".to_owned(),
            ControllerAction::Press(button) => format!("Pressed {}", button.name()),
            ControllerAction::Release(button) => format!("Released {}", button.name()),
            ControllerAction::Enter(zone) => format!("Entered {}", zone.get_name()),
            ControllerAction::Leave(zone) => format!("Left {}", zone.get_name()),
        }
    }
}

type SequenceState = usize;

pub struct InputSequence {
    name: &'static str,
    actions: Vec<(ControllerAction,(i32, i32))>,
}

trait FrameRange {
    fn get_range(&self) -> (i32, i32);
}

impl FrameRange for Range<i32> {
    fn get_range(&self) -> (i32, i32) {
        (self.start, self.end)
    }
}

impl FrameRange for i32 {
    fn get_range(&self) -> (i32, i32) {
        (*self, *self)
    }
}

impl InputSequence {
    pub fn new(name: &'static str) -> InputSequence {
        InputSequence {
            name,
            actions: Vec::new(),
        }
    }

    pub fn add(&mut self, action: ControllerAction, frame_number: impl FrameRange) {
        self.actions.push((action, frame_number.get_range()));
    }

    /// Get a reference to the controller sequence's name.
    pub fn name(&self) -> &'static str {
        self.name
    }
}

pub enum ActionSuccess {
    EarlyMiss,
    Early,
    Success,
    Late,
    LateMiss,
}

impl Display for ActionSuccess {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ActionSuccess::EarlyMiss => f.write_str("EARLY"),
            ActionSuccess::Early => f.write_str("SLIGHTLY EARLY"),
            ActionSuccess::Success => f.write_str("SUCCESS"),
            ActionSuccess::Late => f.write_str("SLIGHTLY LATE"),
            ActionSuccess::LateMiss => f.write_str("LATE"),
        }
    }
}

pub struct InputSequenceState<'a> {
    pub sequence: &'a InputSequence,
    pub state: SequenceState,
    pub history: Vec<(ControllerAction, Instant)>,
    pub completed: Option<Vec<(ControllerAction, Instant)>>,
}

impl<'a> InputSequenceState<'a> {
    pub fn new(sequence: &'a InputSequence) -> Self {
        Self { sequence, state: 0, history: Vec::new(), completed: None }
    }

    pub fn action(&mut self, action: ControllerAction, controller: &Controller, now: Instant) -> bool {
        if let Some((expected_action, frame_number)) = self.sequence.actions.get(self.state) {
            if action == *expected_action {

                if let Some((_, last_time)) = self.history.last() {
                    let time = duration_to_frame_count(now - *last_time);
                    let window = &self.sequence.actions[self.state].1;
                    println!("{}, {:?}", time, window);
                    if window.0 as f64 - time > 5. || time - window.1 as f64 > 5. {
                        self.reset();
                        return false
                    }
                }

                self.state += 1;
                self.history.push((action, now));
            }
        }
        if self.state >= self.sequence.actions.len() {
            self.completed = Some(Vec::new());
            std::mem::swap(&mut self.history, self.completed.as_mut().unwrap());
            self.reset();
            return true
        }
        return false
    }

    pub fn sequence_info(&self) -> Option<Vec<(ControllerAction, Duration, ActionSuccess)>> {
        let seq = self.completed_sequence()?;
        let start = seq.get(0)?.1;
        Some(seq.iter().enumerate().scan(start, |last, (i, (action, time))| {
            let since_last = *time - *last;
            let frames_since_last = duration_to_frame_count(since_last);
            *last = *time;
            let (window_start, window_end) = self.sequence.actions[i].1;
            if frames_since_last > window_end as f64 + 1. {
                return Some((action.clone(), since_last, ActionSuccess::LateMiss));
            }
            if frames_since_last > window_end as f64 {
                return Some((action.clone(), since_last, ActionSuccess::Late));
            }
            if frames_since_last < window_start as f64 - 1.{
                return Some((action.clone(), since_last, ActionSuccess::EarlyMiss));
            }
            if frames_since_last < window_start as f64 {
                return Some((action.clone(), since_last, ActionSuccess::Early));
            }
            else {
                return Some((action.clone(), since_last, ActionSuccess::Success))
            }
        }).collect())
    }

    pub fn completed_sequence(&self) -> Option<&Vec<(ControllerAction, Instant)>> {
        self.completed.as_ref()
    }

    pub fn is_successful(&self, actions: &[(&'a ControllerAction, i32)]) -> bool {
        let mut last_frame_number = 0;
        for (i, (action, frame_number)) in actions.iter().enumerate() {
            let frame_diff = self.sequence.actions[i].1;
            let expected_frame = (frame_diff.0 + last_frame_number, frame_diff.1 + last_frame_number);
            if *frame_number < expected_frame.0 {
                return false;
            }
            else if *frame_number > expected_frame.1 {
                return false;
            }
            last_frame_number = *frame_number;
        }
        true
    }

    pub fn success_rate(&self) -> Option<f64> {
        let mut answer = 1.;
        let inputs = self.completed.as_ref()?;
        let start_time = inputs.get(0)?.1;
        let mut deltas = vec![];
        //TODO make this algorithm less convoluted
        for (action, time) in inputs {
            let frame_time = duration_to_frame_count(time.checked_duration_since(start_time)?);
            let delta = (1. - (frame_time % 1.)) % 1.;
            deltas.push(delta);
        }
        deltas.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let delta_with_next = deltas.iter().zip(deltas.iter().cycle().skip(1));
        for (delta, next) in delta_with_next {
            let mut frame_numbers = vec![];
            for (i, (action, time)) in inputs.iter().enumerate() {
                let frame_time = duration_to_frame_count(time.checked_duration_since(start_time)?);

                let frame_number = (frame_time + delta).floor() as i32;
                frame_numbers.push((action, frame_number));
            }
            if !self.is_successful(&frame_numbers) {
                answer -= (next - delta + 1.) % 1.;
            }
        }
        Some(answer)
    }

    pub fn reset(&mut self) {
        self.state = 0;
        self.history.clear();
    }
}

pub fn make_some_sequences() -> Vec<InputSequence> {
    let mut ret = Vec::new();

    let mut short_hop_3f = InputSequence::new("3f short hop");
    short_hop_3f.add(ControllerAction::Press(Y_BUTTON), 0);
    short_hop_3f.add(ControllerAction::Release(Y_BUTTON), 1..2);
    ret.push(short_hop_3f);

    let mut short_hop_3f = InputSequence::new("3f short hop");
    short_hop_3f.add(ControllerAction::Press(X_BUTTON), 0);
    short_hop_3f.add(ControllerAction::Release(X_BUTTON), 1..2);
    ret.push(short_hop_3f);

    let mut jc_shine = InputSequence::new("jc shine");
    jc_shine.add(ControllerAction::Press(Y_BUTTON), 0);
    jc_shine.add(ControllerAction::Press(B_BUTTON), 3);
    ret.push(jc_shine);

    let mut a_b_same_frame = InputSequence::new("press A and B on the same frame");
    a_b_same_frame.add(ControllerAction::Press(A_BUTTON), 0);
    a_b_same_frame.add(ControllerAction::Press(B_BUTTON), 0);
    ret.push(a_b_same_frame);

    let mut pivot = InputSequence::new("pivot");
    pivot.add(ControllerAction::Leave(Zone::SquareZone(zones::RIGHT_SMASH)), 0);
    pivot.add(ControllerAction::Enter(Zone::SquareZone(zones::LEFT_SMASH)), 0..5);
    pivot.add(ControllerAction::Leave(Zone::SquareZone(zones::LEFT_SMASH)), 1);
    ret.push(pivot);

    ret
}
