use std::{ops::{Range, RangeBounds, RangeFrom}, time::Instant};

use crate::{controller::{A_BUTTON, B_BUTTON, Button, Controller, Y_BUTTON}, duration_to_frame_count, zones::Zone};

pub enum ControllerAction {
    Nothing,
    Press(Button),
    Release(Button),
    Enter(Box<dyn Zone>),
    Leave(Box<dyn Zone>),
}

impl PartialEq for ControllerAction {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Press(l0), Self::Press(r0)) => l0 == r0,
            (Self::Release(l0), Self::Release(r0)) => l0 == r0,
            (Self::Enter(l0), Self::Enter(r0)) => l0.get_name() == r0.get_name(),
            (Self::Leave(l0), Self::Leave(r0)) => l0.get_name() == r0.get_name(),
            _ => core::mem::discriminant(self) == core::mem::discriminant(other),
        }
    }
}

impl std::fmt::Debug for ControllerAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Nothing => write!(f, "Nothing"),
            Self::Press(arg0) => f.debug_tuple("Press").field(arg0).finish(),
            Self::Release(arg0) => f.debug_tuple("Release").field(arg0).finish(),
            Self::Enter(arg0) => f.debug_tuple("Enter").field(&arg0.get_name()).finish(),
            Self::Leave(arg0) => f.debug_tuple("Leave").field(&arg0.get_name()).finish(),
        }
    }
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

pub struct InputSequenceState<'a> {
    pub sequence: &'a InputSequence,
    pub state: SequenceState,
    pub history: Vec<(ControllerAction, Instant)>,
    pub completed: Option<Vec<(ControllerAction, Instant)>>,
}

pub enum MissInfo<'a> {
    Success,
    Early(&'a ControllerAction),
    Late(&'a ControllerAction),
}

impl<'a> InputSequenceState<'a> {
    pub fn new(sequence: &'a InputSequence) -> Self {
        Self { sequence, state: 0, history: Vec::new(), completed: None }
    }

    pub fn action(&mut self, action: ControllerAction, controller: &Controller, now: Instant) -> bool {
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

    pub fn is_successful(&self, actions: &[(&'a ControllerAction, i32)]) -> MissInfo<'a> {
        let mut last_frame_number = 0;
        for (i, (action, frame_number)) in actions.iter().enumerate() {
            let frame_diff = self.sequence.actions[i].1;
            let expected_frame = (frame_diff.0 + last_frame_number, frame_diff.1 + last_frame_number);
            if *frame_number < expected_frame.0 {
                return MissInfo::Early(*action)
            }
            else if *frame_number > expected_frame.1 {
                return MissInfo::Early(*action)
            }
            last_frame_number = *frame_number;
        }
        MissInfo::Success
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
            let mut frame_numbers = vec![];
            for (i, (action, time)) in inputs.iter().enumerate() {
                let frame_time = duration_to_frame_count(time.checked_duration_since(start_time)?);

                let frame_number = (frame_time + delta).floor() as i32;
                frame_numbers.push((action, frame_number));
            }
            if let MissInfo::Early(_) | MissInfo::Late(_) = self.is_successful(&frame_numbers) {
                answer -= if (next - delta) >= 0. {next - delta} else {next - delta + 1.};
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

    let mut pivot_grab = InputSequence::new("pivot grab");
    pivot_grab.add(ControllerAction::Press(Y_BUTTON), 0);
    pivot_grab.add(ControllerAction::Press(B_BUTTON), 3);
    ret.push(pivot_grab);

    let mut a_b_same_frame = InputSequence::new("press A and B on the same frame");
    a_b_same_frame.add(ControllerAction::Press(A_BUTTON), 0);
    a_b_same_frame.add(ControllerAction::Press(B_BUTTON), 0);
    ret.push(a_b_same_frame);

    ret
}
