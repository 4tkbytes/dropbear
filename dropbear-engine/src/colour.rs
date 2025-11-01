use glam::Vec4;

pub struct Colour {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Colour {
    pub const WHITE: Colour = Colour::new(255, 255, 255, 255);
    pub const BLACK: Colour = Colour::new(0, 0, 0, 255);
    pub const RED: Colour = Colour::new(255, 0, 0, 255);
    pub const GREEN: Colour = Colour::new(0, 255, 0, 255);
    pub const BLUE: Colour = Colour::new(0, 0, 255, 255);
    pub const ORANGE: Colour = Colour::new(255, 165, 0, 255);
    pub const YELLOW: Colour = Colour::new(255, 255, 0, 255);
    pub const PURPLE: Colour = Colour::new(128, 0, 128, 255);
    pub const CYAN: Colour = Colour::new(0, 255, 255, 255);
    pub const MAGENTA: Colour = Colour::new(255, 0, 255, 255);
    pub const LIGHT_GREY: Colour = Colour::new(192, 192, 192, 255);
    pub const DARK_GREY: Colour = Colour::new(128, 128, 128, 255);
    pub const LIGHT_BLUE: Colour = Colour::new(173, 216, 230, 255);

    pub const fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    pub fn to_raw_vec4(&self) -> [f32; 4] {
        [
            self.r as f32 / 255.0,
            self.g as f32 / 255.0,
            self.b as f32 / 255.0,
            self.a as f32 / 255.0,
        ]
    }

    pub fn to_vec4(&self) -> Vec4 {
        Vec4::new(
            self.r as f32 / 255.0,
            self.g as f32 / 255.0,
            self.b as f32 / 255.0,
            self.a as f32 / 255.0,
        )
    }

    pub fn to_hex(&self) -> u32 {
        ((self.r as u32) << 16) | ((self.g as u32) << 8) | (self.b as u32)
    }
}

impl From<Vec4> for Colour {
    fn from(value: Vec4) -> Self {
        Self {
            r: value.w as u8,
            g: value.x as u8,
            b: value.y as u8,
            a: value.z as u8,
        }
    }
}
