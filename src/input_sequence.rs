use std::{fmt::Display, ops::{Range}, time::{Duration, Instant}};

use crate::{controller::{A_BUTTON, B_BUTTON, Button, Controller, L_BUTTON, R_BUTTON, X_BUTTON, Y_BUTTON, Z_BUTTON}, duration_to_frame_count, zones::{self, Zone, ZoneTrait}};

#[derive(Debug, PartialEq, Clone)]
pub enum ControllerAction {
    Nothing,
    Press(Button),
    Release(Button),
    Enter(Zone),
    Leave(Zone),
    CEnter(Zone),
    CLeave(Zone),
    LEnter((u8, u8)),
    LLeave((u8, u8)),
    REnter((u8, u8)),
    RLeave((u8, u8)),
}

impl ToString for ControllerAction {
    fn to_string(&self) -> String {
        match self {
            ControllerAction::Nothing => "nothing".to_owned(),
            ControllerAction::Press(button) => format!("Pressed {}", button.name()),
            ControllerAction::Release(button) => format!("Released {}", button.name()),
            ControllerAction::Enter(zone) => format!("Entered {}", zone.get_name()),
            ControllerAction::Leave(zone) => format!("Left {}", zone.get_name()),
            ControllerAction::CEnter(zone) => format!("C Entered {}", zone.get_name()),
            ControllerAction::CLeave(zone) => format!("C Left {}", zone.get_name()),
            ControllerAction::LEnter(zone) => format!("L Entered [{}, {}]", zone.0, zone.1),
            ControllerAction::LLeave(zone) => format!("L Left [{}, {}]", zone.0, zone.1),
            ControllerAction::REnter(zone) => format!("R Entered [{}, {}]", zone.0, zone.1),
            ControllerAction::RLeave(zone) => format!("R Left [{}, {}]", zone.0, zone.1),
        }
    }
}

type SequenceState = usize;

pub struct InputSequenceAction {
    actions: Vec<ControllerAction>,
    start: i32,
    end: i32,
    from: usize,
    is_fail: bool, //TODO
}

pub struct InputSequence {
    name: &'static str,
    actions: Vec<InputSequenceAction>,
}

pub trait FrameRange {
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

impl Into<Vec<ControllerAction>> for ControllerAction {
    fn into(self) -> Vec<ControllerAction> {
        vec![self]
    }
}

impl InputSequence {
    pub fn new(name: &'static str) -> InputSequence {
        InputSequence {
            name,
            actions: Vec::new(),
        }
    }

    pub fn add(&mut self, action: impl Into<Vec<ControllerAction>>, frame_number: impl FrameRange) {
        self.add_from(action, frame_number, self.actions.len() - 1)
    }

    pub fn add_from(&mut self, action: impl Into<Vec<ControllerAction>>, frame_number: impl FrameRange, from: usize) {
        let range = frame_number.get_range();
        self.actions.push(InputSequenceAction { actions: action.into(), start: range.0, end: range.1, from, is_fail: false });
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
        if let Some(expected_action) = self.sequence.actions.get(self.state) {
            if let Some((_, last_time)) = self.history.last() {
                let time = duration_to_frame_count(now - *last_time);
                let window = (expected_action.start, expected_action.end);
                println!("{}, {:?}", time, window);
                if window.0 as f64 - time > 5. || time - window.1 as f64 > 5. {
                    self.reset();
                }
            }

            //if action == expected_action.action {
            if expected_action.actions.contains(&action) {

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
        //TODO refactor this function so it uses is_successful
        let seq = self.completed_sequence()?;
        let start = seq.get(0)?.1;
        Some(seq.iter().enumerate().scan(vec![start], |lasts, (i, (action, time))| {
            let expected_action = &self.sequence.actions[i];
            if expected_action.from == usize::MAX {
                return Some((action.clone(), Duration::ZERO, ActionSuccess::Success))
            }
            let since_last = *time - lasts[expected_action.from];//TODO rename
            let frames_since_last = duration_to_frame_count(since_last);
            let since_last = *time - lasts[lasts.len() - 1];
            lasts.push(*time);
            let window_start = expected_action.start;
            let window_end = expected_action.end;
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

    fn is_successful(&self, actions: &[(&'a ControllerAction, i32)]) -> bool {
        let mut last_frame_numbers = vec![0];
        for (i, (action, frame_number)) in actions.iter().enumerate().skip(1) {
            let expected_action = &self.sequence.actions[i];
            let frame_diff = (expected_action.start, expected_action.end);
            let expected_frame = (frame_diff.0 + last_frame_numbers[expected_action.from], frame_diff.1 + last_frame_numbers[expected_action.from]);
            if *frame_number < expected_frame.0 {
                return false;
            }
            else if *frame_number > expected_frame.1 {
                return false;
            }
            last_frame_numbers.push(*frame_number);
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
    short_hop_3f.add(vec![ControllerAction::Press(Y_BUTTON), ControllerAction::Press(X_BUTTON)], 0);
    short_hop_3f.add(vec![ControllerAction::Release(Y_BUTTON), ControllerAction::Release(X_BUTTON)], 1..2);
    ret.push(short_hop_3f);

/*
    let mut short_hop_3f = InputSequence::new("3f short hop");
    short_hop_3f.add(ControllerAction::Press(X_BUTTON), 0);
    short_hop_3f.add(ControllerAction::Release(X_BUTTON), 1..2);
    ret.push(short_hop_3f);
*/

    let mut wavedash_3f = InputSequence::new("3f wavedash");
    wavedash_3f.add(ControllerAction::Press(Y_BUTTON), 0);
    wavedash_3f.add(ControllerAction::Press(R_BUTTON), 3);
    ret.push(wavedash_3f);

    let mut wavedash_3f = InputSequence::new("3f wavedash");
    wavedash_3f.add(ControllerAction::Press(Y_BUTTON), 0);
    wavedash_3f.add(ControllerAction::Press(L_BUTTON), 3);
    ret.push(wavedash_3f);

    let mut wavedash_3f = InputSequence::new("hax OS wavedash 3f");//TODO this is wrong
    wavedash_3f.add(vec![ControllerAction::Press(Y_BUTTON), ControllerAction::Press(X_BUTTON)], 0);
    wavedash_3f.add(vec![ControllerAction::Press(L_BUTTON), ControllerAction::Press(R_BUTTON)], 2..3);
    wavedash_3f.add_from(vec![ControllerAction::Press(L_BUTTON), ControllerAction::Press(R_BUTTON)], 0..1, 1);
    ret.push(wavedash_3f);

    let mut wavedash_3f = InputSequence::new("hax OS wavedash 3f");
    wavedash_3f.add(ControllerAction::Press(Y_BUTTON), 0);
    wavedash_3f.add(ControllerAction::Press(R_BUTTON), 2..3);
    wavedash_3f.add_from(ControllerAction::Press(L_BUTTON), 0..1, 0);
    ret.push(wavedash_3f);

    let mut jc_shine = InputSequence::new("jc shine");
    jc_shine.add(ControllerAction::Press(Y_BUTTON), 0);
    jc_shine.add(ControllerAction::Press(B_BUTTON), 3);
    ret.push(jc_shine);

    let mut jc_grab = InputSequence::new("jc grab");
    jc_grab.add(ControllerAction::Press(Y_BUTTON), 0);
    jc_grab.add(ControllerAction::Press(Z_BUTTON), 1..2);
    ret.push(jc_grab);

    let mut jc_grab = InputSequence::new("jc grab");
    jc_grab.add(ControllerAction::Press(X_BUTTON), 0);
    jc_grab.add(ControllerAction::Press(Z_BUTTON), 1..2);
    ret.push(jc_grab);

    let mut jc_up_smash = InputSequence::new("jc up_smash");
    jc_up_smash.add(ControllerAction::Press(Y_BUTTON), 0);
    jc_up_smash.add(ControllerAction::CEnter(Zone::SquareZone(zones::UP_SMASH)), 1..2);
    ret.push(jc_up_smash);

    let mut jc_up_smash = InputSequence::new("jc up_smash");
    jc_up_smash.add(ControllerAction::Press(X_BUTTON), 0);
    jc_up_smash.add(ControllerAction::CEnter(Zone::SquareZone(zones::UP_SMASH)), 1..2);
    ret.push(jc_up_smash);

    let mut a_b_same_frame = InputSequence::new("press A+B on same frame");
    a_b_same_frame.add(ControllerAction::Press(A_BUTTON), 0);
    a_b_same_frame.add(ControllerAction::Press(B_BUTTON), 0);
    ret.push(a_b_same_frame);

    let mut a_b_same_frame = InputSequence::new("press A+B on same frame");
    a_b_same_frame.add(ControllerAction::Press(B_BUTTON), 0);
    a_b_same_frame.add(ControllerAction::Press(A_BUTTON), 0);
    ret.push(a_b_same_frame);

    let mut pivot = InputSequence::new("pivot right");
    pivot.add(ControllerAction::Leave(Zone::SquareZone(zones::RIGHT_SMASH)), 0);
    pivot.add(ControllerAction::Enter(Zone::SquareZone(zones::LEFT_SMASH)), 0..5);
    pivot.add(ControllerAction::Leave(Zone::SquareZone(zones::LEFT_SMASH)), 1);
    ret.push(pivot);

    let mut pivot = InputSequence::new("pivot left");
    pivot.add(ControllerAction::Leave(Zone::SquareZone(zones::LEFT_SMASH)), 0);
    pivot.add(ControllerAction::Enter(Zone::SquareZone(zones::RIGHT_SMASH)), 0..5);
    pivot.add(ControllerAction::Leave(Zone::SquareZone(zones::RIGHT_SMASH)), 1);
    ret.push(pivot);

    let mut adt = InputSequence::new("adt");
    adt.add(vec![ControllerAction::LEnter((43, 140)), ControllerAction::REnter((43, 140))], 0);
    adt.add(vec![ControllerAction::Press(L_BUTTON), ControllerAction::Press(R_BUTTON)], 1);
    ret.push(adt);

    ret
}
