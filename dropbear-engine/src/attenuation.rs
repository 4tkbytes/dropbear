//! The attenuation values of light.
//!
//! See https://en.wikipedia.org/wiki/Attenuation

#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct Attenuation {
    pub range: f32,
    pub constant: f32,
    pub linear: f32,
    pub quadratic: f32,
}

impl Default for Attenuation {
    fn default() -> Self {
        RANGE_50
    }
}

pub const ATTENUATION_PRESETS: &[(Attenuation, &str)] = &[
    (RANGE_7, "Range 7"),
    (RANGE_13, "Range 13"),
    (RANGE_20, "Range 20"),
    (RANGE_32, "Range 32"),
    (RANGE_50, "Range 50"),
    (RANGE_65, "Range 65"),
    (RANGE_100, "Range 100"),
    (RANGE_160, "Range 160"),
    (RANGE_200, "Range 200"),
    (RANGE_325, "Range 325"),
    (RANGE_600, "Range 600"),
    (RANGE_3250, "Range 3250"),
];

pub const RANGE_7: Attenuation = Attenuation {
    range: 7.0,
    constant: 1.0,
    linear: 0.7,
    quadratic: 1.8,
};

pub const RANGE_13: Attenuation = Attenuation {
    range: 13.0,
    constant: 1.0,
    linear: 0.35,
    quadratic: 0.44,
};

pub const RANGE_20: Attenuation = Attenuation {
    range: 20.0,
    constant: 1.0,
    linear: 0.22,
    quadratic: 0.20,
};

pub const RANGE_32: Attenuation = Attenuation {
    range: 32.0,
    constant: 1.0,
    linear: 0.14,
    quadratic: 0.07,
};

pub const RANGE_50: Attenuation = Attenuation {
    range: 50.0,
    constant: 1.0,
    linear: 0.09,
    quadratic: 0.032,
};

pub const RANGE_65: Attenuation = Attenuation {
    range: 65.0,
    constant: 1.0,
    linear: 0.07,
    quadratic: 0.017,
};

pub const RANGE_100: Attenuation = Attenuation {
    range: 100.0,
    constant: 1.0,
    linear: 0.045,
    quadratic: 0.0075,
};

pub const RANGE_160: Attenuation = Attenuation {
    range: 160.0,
    constant: 1.0,
    linear: 0.027,
    quadratic: 0.0028,
};

pub const RANGE_200: Attenuation = Attenuation {
    range: 200.0,
    constant: 1.0,
    linear: 0.022,
    quadratic: 0.0019,
};

pub const RANGE_325: Attenuation = Attenuation {
    range: 325.0,
    constant: 1.0,
    linear: 0.014,
    quadratic: 0.0007,
};

pub const RANGE_600: Attenuation = Attenuation {
    range: 600.0,
    constant: 1.0,
    linear: 0.007,
    quadratic: 0.0002,
};

pub const RANGE_3250: Attenuation = Attenuation {
    range: 3250.0,
    constant: 1.0,
    linear: 0.0014,
    quadratic: 0.000007,
};
