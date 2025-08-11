use rhai::*;
use rhai::plugin::*;

#[rhai::export_module]
pub mod math_functions {
    // Basic math
    pub fn abs(x: f64) -> f64 { x.abs() }
    pub fn sqrt(x: f64) -> f64 { x.sqrt() }
    pub fn pow(x: f64, y: f64) -> f64 { x.powf(y) }
    pub fn min(x: f64, y: f64) -> f64 { x.min(y) }
    pub fn max(x: f64, y: f64) -> f64 { x.max(y) }
    pub fn clamp(x: f64, min: f64, max: f64) -> f64 { x.clamp(min, max) }

    // Trig functions
    pub fn sin(x: f64) -> f64 { x.sin() }
    pub fn cos(x: f64) -> f64 { x.cos() }
    pub fn tan(x: f64) -> f64 { x.tan() }
    pub fn asin(x: f64) -> f64 { x.asin() }
    pub fn acos(x: f64) -> f64 { x.acos() }
    pub fn atan(x: f64) -> f64 { x.atan() }
    pub fn atan2(y: f64, x: f64) -> f64 { y.atan2(x) }

    // Hyperbolic functions
    pub fn sinh(x: f64) -> f64 { x.sinh() }
    pub fn cosh(x: f64) -> f64 { x.cosh() }
    pub fn tanh(x: f64) -> f64 { x.tanh() }

    // Logarithmic and exponential
    pub fn exp(x: f64) -> f64 { x.exp() }
    pub fn ln(x: f64) -> f64 { x.ln() }
    pub fn log10(x: f64) -> f64 { x.log10() }
    pub fn log2(x: f64) -> f64 { x.log2() }

    // Rounding functions
    pub fn floor(x: f64) -> f64 { x.floor() }
    pub fn ceil(x: f64) -> f64 { x.ceil() }
    pub fn round(x: f64) -> f64 { x.round() }
    pub fn trunc(x: f64) -> f64 { x.trunc() }
    pub fn fract(x: f64) -> f64 { x.fract() }

    // Conversion
    pub fn to_radians(degrees: f64) -> f64 { degrees.to_radians() }
    pub fn to_degrees(radians: f64) -> f64 { radians.to_degrees() }

    // Utility functions
    pub fn lerp(a: f64, b: f64, t: f64) -> f64 { a + (b - a) * t }
    pub fn smoothstep(edge0: f64, edge1: f64, x: f64) -> f64 {
        let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
        t * t * (3.0 - 2.0 * t)
    }

    // Consts
    pub const PI: f64 = std::f64::consts::PI;
    pub const E: f64 = std::f64::consts::E;
    pub const TAU: f64 = std::f64::consts::TAU;
}

pub fn register_math_functions(engine: &mut Engine) {
    engine.register_static_module("math", exported_module!(math_functions).into());
}