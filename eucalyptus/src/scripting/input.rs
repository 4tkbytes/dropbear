use std::{collections::{HashMap, HashSet}, time::{Duration, Instant}};

use rhai::*;
use winit::{event::MouseButton, keyboard::KeyCode};

#[derive(rhai::CustomType, Clone)]
pub struct InputState {
    pub last_key_press_times: HashMap<KeyCode, Instant>,
    pub double_press_threshold: Duration,
    pub mouse_pos: (f64, f64),
    pub mouse_button: HashSet<MouseButton>,    
    pub pressed_keys: HashSet<KeyCode>,
    pub mouse_delta: Option<(f64, f64)>,
}

impl Default for InputState {
    fn default() -> Self {
        Self::new()
    }
}

impl InputState {
    pub fn new() -> Self {
        Self {
            mouse_pos: Default::default(),
            mouse_button: Default::default(),
            pressed_keys: HashSet::new(),
            last_key_press_times: HashMap::new(),
            double_press_threshold: Duration::from_millis(300),
            mouse_delta: None,
        }
    }
    pub fn register_input_modules(engine: &mut Engine) {
        engine.register_type_with_name::<InputState>("InputState");
        engine.register_type_with_name::<Key>("Key");

        engine
            .register_fn("is_pressed", |s: &mut InputState, k: Key| s.pressed_keys.contains(&k.0))
            .register_get("mouse_x", |s: &mut InputState| s.mouse_pos.0)
            .register_get("mouse_y", |s: &mut InputState| s.mouse_pos.1)
            .register_get("has_mouse_delta", |s: &mut InputState| s.mouse_delta.is_some())
            .register_fn("mouse_dx", |s: &mut InputState| s.mouse_delta.map(|d| d.0).unwrap_or(0.0))
            .register_fn("mouse_dy", |s: &mut InputState| s.mouse_delta.map(|d| d.1).unwrap_or(0.0));

        let mut kmod = Module::new();

        macro_rules! add_key {
            ($name:ident, $kc:expr) => {
                kmod.set_var(stringify!($name), Key($kc));
            };
        }

        add_key!(A, KeyCode::KeyA);
        add_key!(B, KeyCode::KeyB);
        add_key!(C, KeyCode::KeyC);
        add_key!(D, KeyCode::KeyD);
        add_key!(E, KeyCode::KeyE);
        add_key!(F, KeyCode::KeyF);
        add_key!(G, KeyCode::KeyG);
        add_key!(H, KeyCode::KeyH);
        add_key!(I, KeyCode::KeyI);
        add_key!(J, KeyCode::KeyJ);
        add_key!(K, KeyCode::KeyK);
        add_key!(L, KeyCode::KeyL);
        add_key!(M, KeyCode::KeyM);
        add_key!(N, KeyCode::KeyN);
        add_key!(O, KeyCode::KeyO);
        add_key!(P, KeyCode::KeyP);
        add_key!(Q, KeyCode::KeyQ);
        add_key!(R, KeyCode::KeyR);
        add_key!(S, KeyCode::KeyS);
        add_key!(T, KeyCode::KeyT);
        add_key!(U, KeyCode::KeyU);
        add_key!(V, KeyCode::KeyV);
        add_key!(W, KeyCode::KeyW);
        add_key!(X, KeyCode::KeyX);
        add_key!(Y, KeyCode::KeyY);
        add_key!(Z, KeyCode::KeyZ);

        add_key!(Space, KeyCode::Space);
        add_key!(ShiftLeft, KeyCode::ShiftLeft);
        add_key!(ShiftRight, KeyCode::ShiftRight);
        add_key!(ControlLeft, KeyCode::ControlLeft);
        add_key!(ControlRight, KeyCode::ControlRight);
        add_key!(AltLeft, KeyCode::AltLeft);
        add_key!(AltRight, KeyCode::AltRight);
        add_key!(Escape, KeyCode::Escape);
        add_key!(Enter, KeyCode::Enter);
        add_key!(Tab, KeyCode::Tab);
        add_key!(Up, KeyCode::ArrowUp);
        add_key!(Down, KeyCode::ArrowDown);
        add_key!(Left, KeyCode::ArrowLeft);
        add_key!(Right, KeyCode::ArrowRight);

        engine.register_static_module("keys", kmod.into());
        log::info!("[Script] Initialised input module");
    }
}

#[derive(rhai::CustomType, Clone, Copy)]
pub struct Key(pub KeyCode);