use std::{ops::{Range, RangeBounds, RangeFrom}, time::Instant};

use crate::{controller::{B_BUTTON, Button, Y_BUTTON}, duration_to_frame_count};

#[derive(PartialEq)]
pub enum ControllerAction {
    Nothing,
    Press(Button),
    Release(Button),
}

impl ToString for ControllerAction {
    fn to_string(&self) -> String {
        match self {
            ControllerAction::Nothing => "nothing".to_owned(),
            ControllerAction::Press(button) => format!("Pressed {}", button.name()),
            ControllerAction::Release(button) => format!("Released {}", button.name()),
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

    pub fn action(&mut self, action: ControllerAction, now: Instant) -> bool {
        if let Some((expected_action, frame_number)) = self.sequence.actions.get(self.state) {
            if action == *expected_action {
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

    pub fn completed_sequence(&self) -> Option<&Vec<(ControllerAction, Instant)>> {
        self.completed.as_ref()
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
        let delta_with_next = deltas.iter().zip(deltas.iter().cycle().skip(1));
        for (delta, next) in delta_with_next {
            for (i, (action, time)) in inputs.iter().enumerate() {
                let frame_time = duration_to_frame_count(time.checked_duration_since(start_time)?);

                let frame_number = (frame_time + delta).floor() as i32;
                let expected_frame = self.sequence.actions[i].1;
                if !(frame_number >= expected_frame.0 && frame_number <= expected_frame.1) {
                    answer -= if (next - delta) >= 0. {next - delta} else {next - delta + 1.};
                    break;
                }
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
    let mut jc_shine = InputSequence::new("jc shine");
    jc_shine.add(ControllerAction::Press(Y_BUTTON), 0);
    jc_shine.add(ControllerAction::Press(B_BUTTON), 3);
    ret.push(jc_shine);
    ret
}
