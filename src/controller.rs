use std::convert::TryInto;

/*
//ty altimor!!
let magnitude = Math.sqrt(x*x + y*y);
let scale = 80 / magnitude;
if (scale >= 1.0)
    return [x, y]
                        
return [Math.trunc(x * scale), Math.trunc(y * scale)]
*/
pub fn clamp(x_in: i8, y_in: i8) -> (i8, i8) {
    let x = x_in as f64;
    let y = y_in as f64;
    let magnitude = f64::sqrt(x*x + y*y);
    let scale = 80. / magnitude;
    if scale >= 1.0 {
        return (x_in, y_in)
    }
    return ((x*scale).trunc() as i8, (y*scale).trunc() as i8)
}

#[derive(Clone, Copy)]
pub struct Controller {
    pub buffer: [u8; 8],
    buffer_last: [u8; 8],
    startx: i8,
    starty: i8,
    c_startx: i8,
    c_starty: i8,
}

pub struct Button {
    index: u8,
    mask: u8,
    name: &'static str,
}

pub const A_BUTTON: Button = Button { index: 0, mask: 0x1, name: "A" };
pub const B_BUTTON: Button = Button { index: 0, mask: 0x2, name: "B" };
pub const X_BUTTON: Button = Button { index: 0, mask: 0x4, name: "X" };
pub const Y_BUTTON: Button = Button { index: 0, mask: 0x8, name: "Y" };
pub const D_LEFT_BUTTON: Button = Button { index: 0, mask: 0x10, name: "D_LEFT" };
pub const D_RIGHT_BUTTON: Button = Button { index: 0, mask: 0x20, name: "D_RIGHT" };
pub const D_DOWN_BUTTON: Button = Button { index: 0, mask: 0x40, name: "D_DOWN" };
pub const D_UP_BUTTON: Button = Button { index: 0, mask: 0x80, name: "D_UP" };

pub const START_BUTTON: Button = Button { index: 1, mask: 0x1, name: "START" };
pub const Z_BUTTON: Button = Button { index: 1, mask: 0x2, name: "Z" };
pub const R_BUTTON: Button = Button { index: 1, mask: 0x4, name: "R" };
pub const L_BUTTON: Button = Button { index: 1, mask: 0x8, name: "L" };

pub const BUTTONS: [Button; 12] = [A_BUTTON, B_BUTTON, X_BUTTON, Y_BUTTON, D_LEFT_BUTTON, D_RIGHT_BUTTON, D_UP_BUTTON, D_DOWN_BUTTON, START_BUTTON, Z_BUTTON, R_BUTTON, L_BUTTON];

impl Controller {
    pub fn new() -> Controller {
        let c = Controller { buffer: [0; 8], buffer_last: [0; 8], startx: 0, starty: 0, c_startx: 0, c_starty: 0};
        c
    }

    pub fn from_buffer(&mut self, buffer: &[u8; 8]) {
        if self.buffer[2..6] == [0; 4] && buffer[2..6] != [0; 4] {
            self.startx = (buffer[2] as i8).wrapping_sub(-128);
            self.starty = (buffer[3] as i8).wrapping_sub(-128);
            self.c_startx = (buffer[4] as i8).wrapping_sub(-128);
            self.c_starty = (buffer[5] as i8).wrapping_sub(-128);
            println!("setting start pos's {} {} {} {}", self.startx, self.starty, self.c_startx, self.c_starty);
        }
        self.buffer_last = self.buffer;
        self.buffer.copy_from_slice(buffer);
    }

    pub fn pressed_buttons(&self) -> Vec<String> {
        let mut buttons = vec!{};
        for button in BUTTONS {
            if self.is_down(&button) {
                buttons.push(button.name.to_string());
            }
        }
        buttons
    }

    pub fn is_down(&self, button: &Button) -> bool {
        self.buffer[button.index as usize] & button.mask != 0
    }

    pub fn just_pressed(&self, button: &Button) -> bool {
        let pressed_last = self.buffer_last[button.index as usize] & button.mask != 0;
        self.is_down(button) && !pressed_last
    }
    
    pub fn stick_pos(&self) -> (i8, i8) {
        let (x, y) = self.stick_raw();
        (
            x.saturating_sub(self.startx),
            y.saturating_sub(self.starty)
        )
    }

    pub fn c_stick_pos(&self) -> (i8, i8) {
        let (x, y) = self.c_stick_raw();
        (
            x.saturating_sub(self.c_startx),
            y.saturating_sub(self.c_starty)
        )
    }

    pub fn stick_clamp(&self) -> (i8, i8) {
        let pos = self.stick_pos();
        clamp(pos.0, pos.1)
    }

    pub fn c_stick_clamp(&self) -> (i8, i8) {
        let pos = self.c_stick_pos();
        clamp(pos.0, pos.1)
    }

    pub fn stick_raw(&self) -> (i8, i8) {
        ((self.buffer[2] as i8).wrapping_sub(-128), (self.buffer[3] as i8).wrapping_sub(-128))
    }

    pub fn c_stick_raw(&self) -> (i8, i8) {
        ((self.buffer[4] as i8).wrapping_sub(-128), (self.buffer[5] as i8).wrapping_sub(-128))
    }
}

impl ToString for Controller {
    fn to_string(&self) -> String {
        self.pressed_buttons().join(", ")
    }
}

pub fn update_controllers(controllers: &mut [Controller], buffer: &[u8; 37]) {
    let mut index = 2;
    for controller in controllers {
        controller.from_buffer(&buffer[index..(index+8)].try_into().unwrap());//TODO unwrap
        index += 9;
    }
}
