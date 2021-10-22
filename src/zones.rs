

pub struct Zone {
    pub name: String,
    pub bg_color: (u8, u8, u8),
    pub fg_color: (u8, u8, u8),
}

pub trait Plane {
    fn get_zone(&self, point: (i8, i8)) -> Zone;
    fn get_name(&self) -> String;
}

pub struct Test1 {
}

pub struct Test2 {
}

const WHITE: (u8, u8, u8) = (0xff, 0xff, 0xff);

impl Plane for Test1 {
    fn get_zone(&self, point: (i8, i8)) -> Zone {
        if point.0.abs() < 23 && point.1.abs() < 23 {
            return Zone { name: "deadzone".to_string(), bg_color: (0x40, 0x40, 0x40), fg_color: (0xc0, 0xc0, 0xc0) };
        }
        return Zone { name: "live".to_string(), bg_color: (0x70, 0x70, 0x70), fg_color: WHITE };
    }

    fn get_name(&self) -> String {
        "test1".to_string()
    }
}

impl Plane for Test2 {
    fn get_zone(&self, point: (i8, i8)) -> Zone {
        if point.0.abs() < 23 && point.1.abs() < 23 {
            return Zone { name: "deadzone".to_string(), bg_color: (0x40, 0x40, 0x40), fg_color: (0xc0, 0xc0, 0xc0) };
        }
        if point.0.abs() >= 64 {
            return Zone { name: "f smash".to_string(), bg_color: (0x40, 0x00, 0x40), fg_color: (0x80, 0x00, 0xff) };
        }
        if point.1 >= 53 {
            return Zone { name: "up smash".to_string(), bg_color: (0x40, 0x00, 0x40), fg_color: (0x80, 0x00, 0xff) };
        }
        if point.1 <= -53 {
            return Zone { name: "d smash".to_string(), bg_color: (0x40, 0x00, 0x40), fg_color: (0x80, 0x00, 0xff) };
        }
        if point.1.abs() < 23 {
            return Zone { name: "f tilt".to_string(), bg_color: (0x00, 0x00, 0x60), fg_color: (0x00, 0x00, 0xff) };
        }
        if point.0.abs() < 23 && point.1 > 0 {
            return Zone { name: "up tilt".to_string(), bg_color: (0x00, 0x60, 0x00), fg_color: (0x00, 0xff, 0x00) };
        }
        if point.0.abs() < 23 && point.1 < 0 {
            return Zone { name: "d tilt".to_string(), bg_color: (0x60, 0x00, 0x00), fg_color: (0xff, 0x00, 0x00) };
        }
        return Zone { name: "idk lol".to_string(), bg_color: (0x70, 0x70, 0x70), fg_color: WHITE };
    }

    fn get_name(&self) -> String {
        "test2".to_string()
    }
}
