use crate::controller::clamp;

/*
cstick smashes use the same thresholds as control stick smashes (+/- .8000 X and +/- .6625 Y) while aerials happen as soon as you leave the deadzone (.2875)
-altimor
*/

#[derive(PartialEq, Debug, Clone)]
pub struct ZoneColor {
    pub name: String,
    pub bg_color: (u8, u8, u8),
    pub fg_color: (u8, u8, u8),
}

pub trait ZoneTrait {
    fn in_zone(&self, pos: (i8, i8)) -> bool;
    fn get_name(&self) -> &'static str;
}

#[derive(Debug, PartialEq, Hash, Eq, Clone)]
pub enum Zone {
    SquareZone(SquareZone),
}

impl ZoneTrait for Zone {
    fn in_zone(&self, pos: (i8, i8)) -> bool {
        match self {
            Zone::SquareZone(zone) => zone.in_zone(pos),
        }
    }

    fn get_name(&self) -> &'static str {
        match self {
            Zone::SquareZone(zone) => zone.get_name(),
        }
    }
}

#[derive(Debug, PartialEq, Hash, Eq, Clone)]
pub struct SquareZone {
    min_x: i8,
    max_x: i8,
    min_y: i8,
    max_y: i8,
    name: &'static str,
}

impl ZoneTrait for SquareZone {
    fn in_zone(&self, pos: (i8, i8)) -> bool {
        return pos.0 >= self.min_x && pos.0 <= self.max_x && pos.1 >= self.min_y && pos.1 <= self.max_y
    }

    fn get_name(&self) -> &'static str {
        self.name
    }
}

pub const DEADZONE: SquareZone = SquareZone {
    min_x: -23, max_x: 23, min_y: -23, max_y: 23, name: "deadzone"
};
pub const EVERYTHING: SquareZone = SquareZone {
    min_x: -128, max_x: 127, min_y: -128, max_y: 127, name: "everything"
};
pub const RIGHT_SMASH: SquareZone = SquareZone {
    min_x: 64, max_x: 127, min_y: -128, max_y: 127, name: "f smash left"
};
pub const LEFT_SMASH: SquareZone = SquareZone {
    min_x: -128, max_x: -64, min_y: -128, max_y: 127, name: "f smash right"
};
pub const UP_SMASH: SquareZone = SquareZone {
    min_x: -128, max_x: 127, min_y: 53, max_y: 127, name: "up smash"
};
pub const DOWN_SMASH: SquareZone = SquareZone {
    min_x: -128, max_x: 127, min_y: -128, max_y: -53, name: "down smash"
};

pub fn get_some_zones<'a>() -> Vec<&'a Zone> {
    vec![
        &Zone::SquareZone(DEADZONE),
        &Zone::SquareZone(LEFT_SMASH),
        &Zone::SquareZone(RIGHT_SMASH),
        &Zone::SquareZone(UP_SMASH),
        &Zone::SquareZone(DOWN_SMASH),
    ]
}

pub trait Plane {
    fn get_zone(&self, point: (i8, i8)) -> ZoneColor;
    fn get_name(&self) -> String;
}

pub struct PlaneWithZones<'a> {
    zones: Vec<(&'a dyn ZoneTrait, ZoneColor)>,
}

impl<'a> PlaneWithZones<'a> {
    pub fn new() -> Self {
        Self { zones: Vec::new() }
    }

    pub fn add_zone(&mut self, zone: &'a dyn ZoneTrait, color: ZoneColor) {
        self.zones.push((zone, color));
    }

    pub fn default_plane() -> Self {
        let mut plane = PlaneWithZones::new();
        plane.add_zone(&DEADZONE, ZoneColor { name: "deadzone".to_string(), bg_color: (0x40, 0x40, 0x40), fg_color: (0xc0, 0xc0, 0xc0) });
        plane.add_zone(&LEFT_SMASH, ZoneColor { name: "f smash".to_string(), bg_color: (0x40, 0x00, 0x40), fg_color: (0x80, 0x00, 0xff) });
        plane.add_zone(&RIGHT_SMASH, ZoneColor { name: "f smash".to_string(), bg_color: (0x40, 0x00, 0x40), fg_color: (0x80, 0x00, 0xff) });
        plane.add_zone(&UP_SMASH, ZoneColor { name: "up smash".to_string(), bg_color: (0x40, 0x00, 0x40), fg_color: (0x80, 0x00, 0xff) });
        plane.add_zone(&DOWN_SMASH, ZoneColor { name: "d smash".to_string(), bg_color: (0x40, 0x00, 0x40), fg_color: (0x80, 0x00, 0xff) });
        plane
    }
}

impl<'a> Plane for PlaneWithZones<'a> {
    fn get_zone(&self, point: (i8, i8)) -> ZoneColor {
        for zone in self.zones.iter() {
            if zone.0.in_zone(point) {
                return zone.1.clone()
            }
        }
        return ZoneColor { name: "idk lol".to_string(), bg_color: (0x70, 0x70, 0x70), fg_color: WHITE };
    }

    fn get_name(&self) -> String {
        "zones".to_string()
    }
}

pub struct Test1 {
}

pub struct Test2 {
}

pub struct CStick {
}

const WHITE: (u8, u8, u8) = (0xff, 0xff, 0xff);

impl Plane for Test1 {
    fn get_zone(&self, point: (i8, i8)) -> ZoneColor {
        if point.0.abs() < 23 && point.1.abs() < 23 {
            return ZoneColor { name: "deadzone".to_string(), bg_color: (0x40, 0x40, 0x40), fg_color: (0xc0, 0xc0, 0xc0) };
        }
        return ZoneColor { name: "live".to_string(), bg_color: (0x70, 0x70, 0x70), fg_color: WHITE };
    }

    fn get_name(&self) -> String {
        "test1".to_string()
    }
}

impl Plane for Test2 {
    fn get_zone(&self, point: (i8, i8)) -> ZoneColor {
        if clamp(point.0, point.1) != point {
            return ZoneColor { name: "out of bounds".to_string(), bg_color: (0x00, 0x00, 0x00), fg_color: (0x30, 0x00, 0x00) };
        }
        if point.0.abs() < 23 && point.1.abs() < 23 {
            return ZoneColor { name: "deadzone".to_string(), bg_color: (0x40, 0x40, 0x40), fg_color: (0xc0, 0xc0, 0xc0) };
        }
        if point.0.abs() >= 64 {
            return ZoneColor { name: "f smash".to_string(), bg_color: (0x40, 0x00, 0x40), fg_color: (0x80, 0x00, 0xff) };
        }
        if point.1 >= 53 {
            return ZoneColor { name: "up smash".to_string(), bg_color: (0x40, 0x00, 0x40), fg_color: (0x80, 0x00, 0xff) };
        }
        if point.1 <= -53 {
            return ZoneColor { name: "d smash".to_string(), bg_color: (0x40, 0x00, 0x40), fg_color: (0x80, 0x00, 0xff) };
        }
        if point.1.abs() < 23 {
            return ZoneColor { name: "f tilt".to_string(), bg_color: (0x00, 0x00, 0x60), fg_color: (0x00, 0x00, 0xff) };
        }
        if point.0.abs() < 23 && point.1 > 0 {
            return ZoneColor { name: "up tilt".to_string(), bg_color: (0x00, 0x60, 0x00), fg_color: (0x00, 0xff, 0x00) };
        }
        if point.0.abs() < 23 && point.1 < 0 {
            return ZoneColor { name: "d tilt".to_string(), bg_color: (0x60, 0x00, 0x00), fg_color: (0xff, 0x00, 0x00) };
        }
        return ZoneColor { name: "idk lol".to_string(), bg_color: (0x70, 0x70, 0x70), fg_color: WHITE };
    }

    fn get_name(&self) -> String {
        "test2".to_string()
    }
}

impl Plane for CStick {
    fn get_zone(&self, point: (i8, i8)) -> ZoneColor {
        if clamp(point.0, point.1) != point {
            return ZoneColor { name: "out of bounds".to_string(), bg_color: (0x00, 0x00, 0x00), fg_color: (0x50, 0x00, 0x00) };
        }
        if point.0.abs() < 23 && point.1.abs() < 23 {
            return ZoneColor { name: "deadzone".to_string(), bg_color: (0x30, 0x30, 0x00), fg_color: (0x70, 0x70, 0x00) };
        }
        else if true {
            return ZoneColor { name: "yellow".to_string(), bg_color: (0x80, 0x80, 0x00), fg_color: (0xff, 0xff, 0x00) };
        }
        return ZoneColor { name: "idk lol".to_string(), bg_color: (0x70, 0x70, 0x70), fg_color: WHITE };
    }

    fn get_name(&self) -> String {
        "c stick".to_string()
    }
}
