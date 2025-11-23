use crate::camera::{CameraComponent, CameraType};
use crate::config::{ProjectConfig, ResourceConfig, SourceConfig};
use crate::scene::SceneConfig;
use crate::traits::SerializableComponent;
use dropbear_engine::camera::Camera;
use dropbear_engine::entity::{MaterialOverride, MeshRenderer, Transform};
use dropbear_engine::lighting::LightComponent;
use dropbear_engine::utils::ResourceReference;
use dropbear_macro::SerializableComponent;
use egui::Ui;
use once_cell::sync::Lazy;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::borrow::Borrow;
use std::collections::HashMap;
use std::fmt;
use std::fmt::{Display, Formatter};
use std::ops::{Deref, DerefMut};
use std::path::PathBuf;

pub static PROJECT: Lazy<RwLock<ProjectConfig>> =
    Lazy::new(|| RwLock::new(ProjectConfig::default()));

pub static RESOURCES: Lazy<RwLock<ResourceConfig>> =
    Lazy::new(|| RwLock::new(ResourceConfig::default()));

pub static SOURCE: Lazy<RwLock<SourceConfig>> = Lazy::new(|| RwLock::new(SourceConfig::default()));

pub static SCENES: Lazy<RwLock<Vec<SceneConfig>>> = Lazy::new(|| RwLock::new(Vec::new()));

/// Removes a scene with the provided name from the in-memory scene cache.
/// Returns `true` when a scene was removed and `false` when no matching scene existed.
pub fn unload_scene(scene_name: &str) -> bool {
    let mut scenes = SCENES.write();
    let initial_len = scenes.len();
    scenes.retain(|scene| scene.scene_name != scene_name);
    let removed = scenes.len() != initial_len;

    if removed {
        log::info!("Unloaded scene '{}' from memory", scene_name);
    } else {
        log::debug!("Scene '{}' was not loaded; nothing to unload", scene_name);
    }

    removed
}

/// Reads a scene configuration from disk based on the active project's path.
pub fn load_scene(scene_name: &str) -> anyhow::Result<SceneConfig> {
    let scene_path = {
        let project = PROJECT.read();
        if project.project_path.as_os_str().is_empty() {
            return Err(anyhow::anyhow!(
                "Project path is not set; cannot load scenes"
            ));
        }

        project
            .project_path
            .join("scenes")
            .join(format!("{}.eucs", scene_name))
    };

    let scene = SceneConfig::read_from(&scene_path)?;
    log::info!(
        "Loaded scene '{}' from {}",
        scene_name,
        scene_path.display()
    );
    Ok(scene)
}

/// Reloads a scene into the in-memory cache by unloading any existing copy first.
pub fn load_scene_into_memory(scene_name: &str) -> anyhow::Result<()> {
    unload_scene(scene_name);

    let scene = load_scene(scene_name)?;
    {
        let mut scenes = SCENES.write();
        scenes.insert(0, scene);
    }

    log::info!("Scene '{}' loaded into memory", scene_name);

    Ok(())
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Node {
    File(File),
    Folder(Folder),
}

#[derive(Default, Debug, Serialize, Deserialize, Clone)]
pub enum File {
    #[default]
    Unknown,
    ResourceFile {
        name: String,
        path: PathBuf,
        resource_type: ResourceType,
    },
    SourceFile {
        name: String,
        path: PathBuf,
    },
}

#[derive(Default, Debug, Serialize, Deserialize, Clone)]
pub struct Folder {
    pub name: String,
    pub path: PathBuf,
    pub nodes: Vec<Node>,
}

/// The type of resource
#[derive(Debug, Serialize, Deserialize, Clone, Hash)]
pub enum ResourceType {
    Unknown,
    Config,
    Script,
    Model,
    Thumbnail,
    Texture,
    Shader,
}

impl Display for ResourceType {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let str = match self {
            ResourceType::Unknown => "unknown",
            ResourceType::Model => "model",
            ResourceType::Texture => "texture",
            ResourceType::Shader => "shader",
            ResourceType::Thumbnail => "thumbnail",
            ResourceType::Script => "script",
            ResourceType::Config => "eucalyptus project config",
        };
        write!(f, "{}", str)
    }
}

#[derive(Default, Debug, Serialize, Deserialize, Clone, SerializableComponent)]
pub struct Script {
    pub tags: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, SerializableComponent)]
pub struct Camera3D {
    pub label: String,
    pub transform: Transform,
    pub camera_type: CameraType,

    pub aspect: f64,
    pub fov: f32,
    pub near: f32,
    pub far: f32,

    pub speed: f32,
    pub sensitivity: f32,

    pub starting_camera: bool,
}

impl Default for Camera3D {
    fn default() -> Self {
        let default = CameraComponent::new();
        Self {
            transform: Transform::default(),
            aspect: 16.0 / 9.0,
            fov: 45.0,
            near: 0.1,
            far: 100.0,
            label: String::new(),
            camera_type: CameraType::Normal,
            speed: default.settings.speed as f32,
            sensitivity: default.settings.sensitivity as f32,
            starting_camera: false,
        }
    }
}

impl Camera3D {
    pub fn from_ecs_camera(
        camera: &Camera,
        component: &CameraComponent,
    ) -> Self {
        let position = glam::DVec3::from_array(camera.eye.to_array());
        let target = glam::DVec3::from_array(camera.target.to_array());
        let up = glam::DVec3::from_array(camera.up.to_array());
        
        let rotation = if (target - position).length_squared() > 0.0001 {
             glam::DQuat::from_mat4(&glam::DMat4::look_at_lh(position, target, up)).inverse()
        } else {
             glam::DQuat::IDENTITY
        };

        let transform = Transform {
            position: position,
            rotation: rotation,
            scale: glam::DVec3::ONE,
        };

        Self {
            transform,
            label: camera.label.clone(),
            camera_type: component.camera_type,
            aspect: camera.aspect,
            fov: camera.settings.fov_y as f32,
            near: camera.znear as f32,
            far: camera.zfar as f32,
            speed: component.settings.speed as f32,
            sensitivity: component.settings.sensitivity as f32,
            starting_camera: component.starting_camera,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct Property {
    pub id: u64,
    pub key: String,
    pub value: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Value {
    String(String),
    Int(i64),
    Float(f64),
    Bool(bool),
    Vec3([f32; 3]),
}

impl Default for Value {
    fn default() -> Self {
        Self::String(String::new())
    }
}

impl Display for Value {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let string: String = match self {
            Value::String(_) => "String".into(),
            Value::Int(_) => "Int".into(),
            Value::Float(_) => "Float".into(),
            Value::Bool(_) => "Bool".into(),
            Value::Vec3(_) => "Vec3".into(),
        };
        write!(f, "{}", string)
    }
}

/// Properties for an entity, typically queries with `entity.getProperty<Float>` and `entity.setProperty(67)`
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone, SerializableComponent)]
pub struct ModelProperties {
    pub custom_properties: Vec<Property>,
    pub next_id: u64,
}

impl ModelProperties {
    /// Creates a new [ModelProperties]
    pub fn new() -> Self {
        Self {
            custom_properties: Vec::new(),
            next_id: 0,
        }
    }

    /// Sets the property based on the [Value] (type) and its key.
    ///
    /// If the value does NOT exist, it will be created.
    /// If the value does exist, it will replace the contents of that item.
    pub fn set_property(&mut self, key: String, value: Value) {
        if let Some(prop) = self.custom_properties.iter_mut().find(|p| p.key == key) {
            prop.value = value;
        } else {
            self.custom_properties.push(Property {
                id: self.next_id,
                key,
                value,
            });
            self.next_id += 1;
        }
    }

    /// Fetches the property by its key.
    pub fn get_property(&self, key: &str) -> Option<&Value> {
        self.custom_properties
            .iter()
            .find(|p| p.key == key)
            .map(|p| &p.value)
    }

    /// Fetches the float property
    pub fn get_float(&self, key: &str) -> Option<f64> {
        match self.get_property(key)? {
            Value::Float(f) => Some(*f),
            _ => None,
        }
    }

    /// Fetches the integer property
    pub fn get_int(&self, key: &str) -> Option<i64> {
        match self.get_property(key)? {
            Value::Int(i) => Some(*i),
            _ => None,
        }
    }

    /// Creates a new property based on a key and a value.
    ///
    /// It will push that value again to the property vector.
    pub fn add_property(&mut self, key: String, value: Value) {
        self.custom_properties.push(Property {
            id: self.next_id,
            key,
            value,
        });
        self.next_id += 1;
    }

    /// Shows a template of the different values when inspected as a component in the editor.
    pub fn show_value_editor(ui: &mut Ui, value: &mut Value) -> bool {
        match value {
            Value::String(s) => ui.text_edit_singleline(s).changed(),
            Value::Int(i) => ui
                .add(egui::Slider::new(i, -1000..=1000).text(""))
                .changed(),
            Value::Float(f) => ui
                .add(egui::Slider::new(f, -100.0..=100.0).text(""))
                .changed(),
            Value::Bool(b) => ui.checkbox(b, "").changed(),
            Value::Vec3(vec) => {
                let mut changed = false;
                ui.horizontal(|ui| {
                    changed |= ui
                        .add(
                            egui::Slider::new(&mut vec[0], -10.0..=10.0)
                                .text("X")
                                .fixed_decimals(2),
                        )
                        .changed();
                    changed |= ui
                        .add(
                            egui::Slider::new(&mut vec[1], -10.0..=10.0)
                                .text("Y")
                                .fixed_decimals(2),
                        )
                        .changed();
                    changed |= ui
                        .add(
                            egui::Slider::new(&mut vec[2], -10.0..=10.0)
                                .text("Z")
                                .fixed_decimals(2),
                        )
                        .changed();
                });
                changed
            }
        }
    }
}

impl Default for ModelProperties {
    fn default() -> Self {
        Self::new()
    }
}

// A serializable configuration struct for the [Light] type
#[derive(Debug, Serialize, Deserialize, Clone, SerializableComponent)]
pub struct Light {
    pub label: String,
    pub transform: Transform,
    pub light_component: LightComponent,
    pub enabled: bool,

    #[serde(skip)]
    pub entity_id: Option<hecs::Entity>,
}

impl Default for Light {
    fn default() -> Self {
        Self {
            label: "New Light".to_string(),
            transform: Transform::default(),
            light_component: LightComponent::default(),
            enabled: true,
            entity_id: None,
        }
    }
}

/// Describes the settings of the editor, not the project or the scene.
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct EditorSettings {
    pub is_debug_menu_shown: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum EditorTab {
    AssetViewer,       // bottom side,
    ResourceInspector, // left side,
    ModelEntityList,   // right side,
    Viewport,          // middle,
    ErrorConsole,
    Plugin(usize),
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct PluginInfo {
    pub display_name: String,
}

/// An enum that describes the status of loading the world.
///
/// This is enum is used by [`SceneConfig::load_into_world`] heavily. This enum
/// is recommended to be used with an [`UnboundedSender`]
pub enum WorldLoadingStatus {
    Idle,
    LoadingEntity {
        index: usize,
        name: String,
        total: usize,
    },
    Completed,
}

#[derive(Clone, Debug, Serialize, Deserialize, bincode::Encode, bincode::Decode)]
pub struct RuntimeData {
    #[bincode(with_serde)]
    pub project_config: ProjectConfig,
    #[bincode(with_serde)]
    pub source_config: SourceConfig,
    #[bincode(with_serde)]
    pub scene_data: Vec<SceneConfig>,
    #[bincode(with_serde)]
    pub scripts: HashMap<String, String>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash, Clone, SerializableComponent)]
pub struct Label(String);

impl Default for Label {
    fn default() -> Self {
        Self(String::from("No Label"))
    }
}

impl Label {
    /// Creates a new label component from any type that can be converted into a [`String`].
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// Returns the underlying string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Returns a mutable reference to the underlying [`String`].
    pub fn as_mut_string(&mut self) -> &mut String {
        &mut self.0
    }

    /// Replaces the underlying value with the provided one.
    pub fn set(&mut self, value: impl Into<String>) {
        self.0 = value.into();
    }

    /// Consumes the label and returns the owned [`String`].
    pub fn into_inner(self) -> String {
        self.0
    }

    /// Returns whether the underlying label is empty.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl Display for Label {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<String> for Label {
    fn from(value: String) -> Self {
        Label::new(value)
    }
}

impl From<&str> for Label {
    fn from(value: &str) -> Self {
        Label::new(value)
    }
}

impl AsRef<str> for Label {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl Borrow<str> for Label {
    fn borrow(&self) -> &str {
        self.as_str()
    }
}

impl Deref for Label {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.as_str()
    }
}

impl DerefMut for Label {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.as_mut_string()
    }
}

/// A [MeshRenderer] that is serialized into a file to be stored as a value for config.
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct SerializedMeshRenderer {
    pub handle: ResourceReference,
    pub material_override: Vec<MaterialOverride>,
}

#[typetag::serde]
impl SerializableComponent for SerializedMeshRenderer {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn clone_boxed(&self) -> Box<dyn SerializableComponent> {
        Box::new(self.clone())
    }

    fn type_name(&self) -> &'static str {
        "SerializedMeshRenderer"
    }

    fn display_name(&self) -> String {
        "MeshRenderer".to_string()
    }
}

impl SerializedMeshRenderer {
    /// Creates a new [SerializedMeshRenderer] from an existing [MeshRenderer] by cloning data.
    pub fn from_renderer(renderer: &MeshRenderer) -> Self {
        let handle = renderer.handle();
        Self {
            handle: handle.path.clone(),
            material_override: renderer.material_overrides.clone(),
        }
    }
}
