#[allow(unused)]
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Attenuation {
    pub range: f32,
    pub constant: f32,
    pub linear: f32,
    pub quadratic: f32,
}

#[allow(unused)]
pub const RANGE_7: Attenuation = Attenuation {
    range: 7.0,
    constant: 1.0,
    linear: 0.7,
    quadratic: 1.8,
};

#[allow(unused)]
pub const RANGE_13: Attenuation = Attenuation {
    range: 13.0,
    constant: 1.0,
    linear: 0.35,
    quadratic: 0.44,
};

#[allow(unused)]
pub const RANGE_20: Attenuation = Attenuation {
    range: 20.0,
    constant: 1.0,
    linear: 0.22,
    quadratic: 0.20,
};

#[allow(unused)]
pub const RANGE_32: Attenuation = Attenuation {
    range: 32.0,
    constant: 1.0,
    linear: 0.14,
    quadratic: 0.07,
};

#[allow(unused)]
pub const RANGE_50: Attenuation = Attenuation {
    range: 50.0,
    constant: 1.0,
    linear: 0.09,
    quadratic: 0.032,
};

#[allow(unused)]
pub const RANGE_65: Attenuation = Attenuation {
    range: 65.0,
    constant: 1.0,
    linear: 0.07,
    quadratic: 0.017,
};

#[allow(unused)]
pub const RANGE_100: Attenuation = Attenuation {
    range: 100.0,
    constant: 1.0,
    linear: 0.045,
    quadratic: 0.0075,
};

#[allow(unused)]
pub const RANGE_160: Attenuation = Attenuation {
    range: 160.0,
    constant: 1.0,
    linear: 0.027,
    quadratic: 0.0028,
};

#[allow(unused)]
pub const RANGE_200: Attenuation = Attenuation {
    range: 200.0,
    constant: 1.0,
    linear: 0.022,
    quadratic: 0.0019,
};

#[allow(unused)]
pub const RANGE_325: Attenuation = Attenuation {
    range: 325.0,
    constant: 1.0,
    linear: 0.014,
    quadratic: 0.0007,
};

#[allow(unused)]
pub const RANGE_600: Attenuation = Attenuation {
    range: 600.0,
    constant: 1.0,
    linear: 0.007,
    quadratic: 0.0002,
};

#[allow(unused)]
pub const RANGE_3250: Attenuation = Attenuation {
    range: 3250.0,
    constant: 1.0,
    linear: 0.0014,
    quadratic: 0.000007,
};
