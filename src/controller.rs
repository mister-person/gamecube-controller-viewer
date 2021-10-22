#[derive(Clone, Copy)]
pub struct Controller {
    buffer: [u8; 8],
    startx: u8,
    starty: u8,
    c_startx: u8,
    c_starty: u8,
}

/*
//ty altimor!!
let magnitude = Math.sqrt(x*x + y*y);
let scale = 80 / magnitude;
if (scale >= 1.0)
    return [x, y]
                        
return [Math.trunc(x * scale), Math.trunc(y * scale)]
*/
pub fn clamp(x: i8, y: i8) -> (i8, i8) {
    let x = x as f64;
    let y = y as f64;
    let magnitude = f64::sqrt((x*x + y*y) as f64);
    let scale = 80f64 / magnitude;
    if scale >= 1.0 {
        return (x as i8, y as i8)
    }
    return ((x*scale).trunc() as i8, (y*scale).trunc() as i8)
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

const BUTTONS: [Button; 12] = [A_BUTTON, B_BUTTON, X_BUTTON, Y_BUTTON, D_LEFT_BUTTON, D_RIGHT_BUTTON, D_UP_BUTTON, D_DOWN_BUTTON, START_BUTTON, Z_BUTTON, R_BUTTON, L_BUTTON];

impl Controller {
    pub fn new() -> Controller {
        let c = Controller { buffer: [0; 8], startx: 0, starty: 0, c_startx: 0, c_starty: 0};
        c
    }

    fn from_buffer(&mut self, buffer: &[u8]) {
        if self.buffer[2..6] == [0; 4] && buffer[2..6] != [0; 4] {
            self.startx = buffer[2];
            self.starty = buffer[3];
            self.c_startx = buffer[4];
            self.c_starty = buffer[5];
            println!("setting start pos's {} {} {} {}", self.startx, self.starty, self.c_startx, self.c_starty);
        }
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
    
    pub fn stick_pos(&self) -> (i8, i8) {
        (self.buffer[2].wrapping_sub(self.startx) as i8, self.buffer[3].wrapping_sub(self.starty) as i8)
    }

    pub fn c_stick_pos(&self) -> (i8, i8) {
        (self.buffer[4].wrapping_sub(self.c_startx) as i8, self.buffer[5].wrapping_sub(self.c_starty) as i8)
    }

    pub fn stick_clamp(&self) -> (i8, i8) {
        let pos = self.stick_pos();
        clamp(pos.0, pos.1)
    }

    pub fn c_stick_clamp(&self) -> (i8, i8) {
        let pos = self.c_stick_pos();
        clamp(pos.0, pos.1)
    }

}

impl ToString for Controller {
    fn to_string(&self) -> String {
        self.pressed_buttons().join(", ")
    }
}

pub fn update_controllers(controllers: &mut [Controller], buffer: &[u8]) {
    let mut index = 2;
    for controller in controllers {
        controller.from_buffer(&buffer[index..(index+8)]);
        index += 9;
    }
}
