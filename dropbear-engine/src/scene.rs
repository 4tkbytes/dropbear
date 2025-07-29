use winit::event_loop::ActiveEventLoop;

use crate::{graphics::Graphics, input};
use std::{cell::RefCell, collections::HashMap, rc::Rc};

pub trait Scene {
    fn load(&mut self, graphics: &mut Graphics);
    fn update(&mut self, dt: f32, graphics: &mut Graphics);
    fn render(&mut self, graphics: &mut Graphics);
    fn exit(&mut self, event_loop: &ActiveEventLoop);
    /// By far a mess of a trait however it works.
    ///
    /// This struct allows you to add in a SceneCommand enum and send it to the scene management for them
    /// to parse through.
    fn run_command(&mut self) -> SceneCommand {
        SceneCommand::None
    }
    fn clear_ui(&mut self) {}
}

#[derive(Clone)]
pub enum SceneCommand {
    None,
    Quit,
    SwitchScene(String),
    DebugMessage(String),
}

impl Default for SceneCommand {
    fn default() -> Self {
        Self::None
    }
}

pub type SceneImpl = Rc<RefCell<dyn Scene>>;

#[derive(Clone)]
pub struct Manager {
    current_scene: Option<String>,
    next_scene: Option<String>,
    scenes: HashMap<String, SceneImpl>,
    scene_input_map: HashMap<String, String>,
}

impl Manager {
    pub fn new() -> Self {
        Self {
            scenes: HashMap::new(),
            current_scene: None,
            next_scene: None,
            scene_input_map: HashMap::new(),
        }
    }

    pub fn switch(&mut self, name: &str) {
        if self.scenes.contains_key(name) {
            self.next_scene = Some(name.to_string());
            log::debug!("Switching to scene: {}", name)
        } else {
            log::warn!("No such scene as {}, not switching", name);
        }
    }

    pub fn add(&mut self, name: &str, scene: Rc<RefCell<dyn Scene>>) {
        self.scenes.insert(name.to_string(), scene);
    }

    pub fn attach_input(&mut self, scene_name: &str, input_name: &str) {
        self.scene_input_map
            .insert(scene_name.to_string(), input_name.to_string());
    }

    pub async fn update(
        &mut self,
        dt: f32,
        graphics: &mut Graphics<'_>,
        event_loop: &ActiveEventLoop,
    ) {
        // transition scene
        if let Some(next_scene_name) = self.next_scene.take() {
            if let Some(current_scene_name) = &self.current_scene {
                if let Some(scene) = self.scenes.get_mut(current_scene_name) {
                    scene.borrow_mut().exit(event_loop);
                }
            }
            if let Some(scene) = self.scenes.get_mut(&next_scene_name) {
                scene.borrow_mut().load(graphics);
            }
            self.current_scene = Some(next_scene_name);
        }

        // update scene
        if let Some(scene_name) = &self.current_scene {
            if let Some(scene) = self.scenes.get_mut(scene_name) {
                scene.borrow_mut().update(dt, graphics);
                let command = scene.borrow_mut().run_command();
                match command {
                    SceneCommand::SwitchScene(target) => {
                        if let Some(current) = &self.current_scene {
                            if current == &target {
                                // reload the scene
                                if let Some(scene) = self.scenes.get_mut(current) {
                                    scene.borrow_mut().exit(event_loop);
                                    scene.borrow_mut().load(graphics);
                                    log::debug!("Reloaded scene: {}", current);
                                }
                            } else {
                                self.switch(&target);
                            }
                        } else {
                            self.switch(&target);
                        }
                    }
                    SceneCommand::Quit => {
                        log::info!("Exiting app!");
                        event_loop.exit();
                    }
                    SceneCommand::None => {}
                    SceneCommand::DebugMessage(msg) => log::debug!("{}", msg),
                }
            }
        }
    }

    pub async fn render(&mut self, graphics: &mut Graphics<'_>) {
        if let Some(scene_name) = &self.current_scene {
            if let Some(scene) = self.scenes.get_mut(scene_name) {
                scene.borrow_mut().render(graphics);
            }
        }
    }

    pub fn has_scene(&self) -> bool {
        self.current_scene.is_some()
    }

    pub fn get_current_scene(&self) -> Option<(&String, &SceneImpl)> {
        if let Some(scene_name) = &self.current_scene {
            if let Some(scene) = self.scenes.get(scene_name) {
                return Some((scene_name, scene));
            }
            return None;
        }
        None
    }
}

pub fn add_scene_with_input<
    S: 'static + Scene + input::Keyboard + input::Mouse + input::Controller,
>(
    scene_manager: &mut Manager,
    input_manager: &mut input::Manager,
    scene: Rc<RefCell<S>>,
    scene_name: &str,
) {
    scene_manager.add(scene_name, scene.clone());
    input_manager.add_keyboard(&format!("{}_keyboard", scene_name), scene.clone());
    input_manager.add_mouse(&format!("{}_mouse", scene_name), scene.clone());
    input_manager.add_controller(&format!("{}_controller", scene_name), scene.clone());
    scene_manager.attach_input(scene_name, &format!("{}_keyboard", scene_name));
    scene_manager.attach_input(scene_name, &format!("{}_mouse", scene_name));
    scene_manager.attach_input(scene_name, &format!("{}_controller", scene_name));
}
