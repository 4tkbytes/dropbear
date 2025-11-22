use super::*;
use crate::editor::{
    ViewportMode,
    console_error::{ConsoleItem, ErrorLevel},
};
use std::{
    cmp::Ordering,
    collections::{HashMap, hash_map::DefaultHasher},
    fs,
    hash::{Hash, Hasher},
    io,
    path::{Path, PathBuf},
    sync::LazyLock,
};

use crate::editor::component::InspectableComponent;
use crate::plugin::PluginRegistry;
use dropbear_engine::graphics::NO_TEXTURE;
use dropbear_engine::utils::ResourceReference;
use dropbear_engine::{
    entity::{MeshRenderer, Transform},
    lighting::{Light, LightComponent},
};
use egui::{self, Margin, RichText};
use egui_dock::TabViewer;
use egui_ltreeview::{NodeBuilder, TreeViewBuilder};
use eucalyptus_core::states::{Label, PROJECT};
use eucalyptus_core::traits::registry::ComponentRegistry;
use eucalyptus_core::hierarchy::{Children, Hierarchy, Parent};
use hecs::{Entity, EntityBuilder, World};
use indexmap::Equivalent;
use log;
use parking_lot::Mutex;
use transform_gizmo_egui::{EnumSet, Gizmo, GizmoConfig, GizmoExt, GizmoMode};

pub struct EditorTabViewer<'a> {
    pub view: egui::TextureId,
    pub tex_size: Extent3d,
    pub gizmo: &'a mut Gizmo,
    pub world: &'a mut World,
    pub selected_entity: &'a mut Option<Entity>,
    pub viewport_mode: &'a mut ViewportMode,
    pub undo_stack: &'a mut Vec<UndoableAction>,
    pub signal: &'a mut Signal,
    pub gizmo_mode: &'a mut EnumSet<GizmoMode>,
    pub editor_mode: &'a mut EditorState,
    pub active_camera: &'a mut Arc<Mutex<Option<Entity>>>,
    pub plugin_registry: &'a mut PluginRegistry,
    pub component_registry: &'a ComponentRegistry,
    pub build_logs: &'a mut Vec<String>,

    // "wah wah its unsafe, its using raw pointers" shut the fuck up if it breaks i will know
    pub editor: *mut Editor,
}

#[derive(Clone, Debug)]
pub struct DraggedAsset {
    pub name: String,
    pub path: ResourceReference,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ComponentNodeSelection {
    pub node_id: u64,
    entity_bits: u64,
    pub component_type_id: u64,
}

impl ComponentNodeSelection {
    pub fn entity(&self) -> Option<Entity> {
        Entity::from_bits(self.entity_bits)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct ComponentNodeKey {
    entity_bits: u64,
    component_type_id: u64,
}

impl ComponentNodeKey {
    fn new(entity: Entity, component_type_id: u64) -> Self {
        Self {
            entity_bits: entity.to_bits().get(),
            component_type_id,
        }
    }

    fn as_selection(&self, node_id: u64) -> ComponentNodeSelection {
        ComponentNodeSelection {
            node_id,
            entity_bits: self.entity_bits,
            component_type_id: self.component_type_id,
        }
    }
}

pub static TABS_GLOBAL: LazyLock<Mutex<StaticallyKept>> =
    LazyLock::new(|| Mutex::new(StaticallyKept::default()));

/// Variables kept statically.
///
/// The entire module (including the tab viewer) due to it
/// being part of an update/render function, therefore this is used to ensure
/// progress is not lost.
#[derive(Default)]
pub struct StaticallyKept {
    show_context_menu: bool,
    context_menu_pos: egui::Pos2,
    context_menu_tab: Option<EditorTab>,
    is_focused: bool,
    old_pos: Transform,
    pub(crate) scale_locked: bool,

    pub(crate) old_label_entity: Option<hecs::Entity>,
    pub(crate) label_original: Option<String>,
    pub(crate) label_last_edit: Option<Instant>,

    pub(crate) transform_old_entity: Option<hecs::Entity>,
    pub(crate) transform_original_transform: Option<Transform>,

    pub(crate) transform_in_progress: bool,
    pub(crate) transform_rotation_cache: HashMap<Entity, glam::DVec3>,

    component_node_ids: HashMap<ComponentNodeKey, u64>,
    component_node_lookup: HashMap<u64, ComponentNodeKey>,
    next_component_node_id: u64,
    pub(crate) last_component_lookup: Option<ComponentNodeSelection>,
    pub(crate) pending_component_drag: Option<ComponentNodeSelection>,
}

impl StaticallyKept {
    fn next_component_node_id(&mut self) -> u64 {
        if self.next_component_node_id == 0 {
            self.next_component_node_id = 1;
        }
        let id = self.next_component_node_id;
        self.next_component_node_id = self.next_component_node_id.wrapping_add(1);
        if self.next_component_node_id == 0 {
            self.next_component_node_id = 1;
        }
        id
    }

    fn component_node_id(&mut self, entity: Entity, component_type_id: u64) -> u64 {
        let key = ComponentNodeKey::new(entity, component_type_id);
        if let Some(id) = self.component_node_ids.get(&key) {
            *id
        } else {
            let id = self.next_component_node_id();
            self.component_node_ids.insert(key, id);
            self.component_node_lookup.insert(id, key);
            id
        }
    }

    fn component_selection(&self, node_id: u64) -> Option<ComponentNodeSelection> {
        self.component_node_lookup
            .get(&node_id)
            .map(|key| key.as_selection(node_id))
    }

    fn remember_component_lookup(&mut self, selection: ComponentNodeSelection) {
        self.last_component_lookup = Some(selection);
    }
}

impl<'a> TabViewer for EditorTabViewer<'a> {
    type Tab = EditorTab;

    fn title(&mut self, tab: &mut Self::Tab) -> egui::WidgetText {
        match tab {
            EditorTab::Viewport => "Viewport".into(),
            EditorTab::ModelEntityList => "Model/Entity List".into(),
            EditorTab::AssetViewer => "Asset Viewer".into(),
            EditorTab::ResourceInspector => "Resource Inspector".into(),
            EditorTab::Plugin(dock_index) => {
                if let Some((_, plugin)) = self.plugin_registry.plugins.get_index_mut(*dock_index) {
                    plugin.display_name().into()
                } else {
                    "Unknown Plugin Name".into()
                }
            }
            EditorTab::ErrorConsole => "Error Console".into(),
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, tab: &mut Self::Tab) {
        let mut cfg = TABS_GLOBAL.lock();

        ui.ctx().input(|i| {
            if i.pointer.button_pressed(egui::PointerButton::Secondary)
                && let Some(pos) = i.pointer.hover_pos()
                && ui.available_rect_before_wrap().contains(pos)
            {
                cfg.show_context_menu = true;
                cfg.context_menu_pos = pos;
                cfg.context_menu_tab = Some(tab.clone());
            }
        });

        match tab {
            EditorTab::Viewport => {
                log_once::debug_once!("Viewport focused");
                // ------------------- Template for querying active camera -----------------
                // if let Some(active_camera) = self.active_camera {
                //     if let Ok(mut q) = self.world.query_one::<(&Camera, &CameraComponent, Option<&CameraFollowTarget>)>(*active_camera) {
                //         if let Some((camera, component, follow_target)) = q.get() {

                //         } else {
                //             log_once::warn_once!("Unable to fetch the query result of camera: {:?}", active_camera)
                //         }
                //     } else {
                //         log_once::warn_once!("Unable to query camera, component and option<camerafollowtarget> for active camera: {:?}", active_camera);
                //     }
                // } else {
                //     log_once::warn_once!("No active camera found");
                // }
                // -------------------------------------------------------------------------

                let available_rect = ui.available_rect_before_wrap();
                let available_size = available_rect.size();

                let tex_aspect = self.tex_size.width as f32 / self.tex_size.height as f32;
                let available_aspect = available_size.x / available_size.y;

                let (display_width, display_height) = if available_aspect > tex_aspect {
                    let height = available_size.y * 0.95;
                    let width = height * tex_aspect;
                    (width, height)
                } else {
                    let width = available_size.x * 0.95;
                    let height = width / tex_aspect;
                    (width, height)
                };

                let center_x = available_rect.center().x;
                let center_y = available_rect.center().y;

                let image_rect = egui::Rect::from_center_size(
                    egui::pos2(center_x, center_y),
                    egui::vec2(display_width, display_height),
                );

                let (_rect, _response) =
                    ui.allocate_exact_size(available_size, egui::Sense::click_and_drag());

                let _image_response = ui.allocate_rect(image_rect, egui::Sense::click_and_drag());

                ui.scope_builder(egui::UiBuilder::new().max_rect(image_rect), |ui| {
                    ui.add_sized(
                        [display_width, display_height],
                        egui::Image::new((self.view, [display_width, display_height].into()))
                            .fit_to_exact_size([display_width, display_height].into()),
                    )
                });

                let snapping = ui.input(|input| input.modifiers.shift);

                // Note to self: fuck you >:(
                // Note to self: ok wow thats pretty rude im trying my best ＞﹏＜
                // Note to self: finally holy shit i got it working
                let active_cam = self.active_camera.lock();
                if let Some(active_camera) = *active_cam {
                    let camera_data = {
                        if let Ok(mut q) = self
                            .world
                            .query_one::<(&Camera, &CameraComponent)>(active_camera)
                        {
                            let val = q.get();
                            if let Some(val) = val {
                                Some(val.0.clone())
                            } else {
                                log::warn!("Queried camera but unable to get value");

                                None
                            }
                        } else {
                            None
                        }
                    };

                    if let Some(camera) = camera_data {
                        self.gizmo.update_config(GizmoConfig {
                            view_matrix: camera.view_mat.into(),
                            projection_matrix: camera.proj_mat.into(),
                            viewport: image_rect,
                            modes: *self.gizmo_mode,
                            orientation: transform_gizmo_egui::GizmoOrientation::Global,
                            snapping,
                            snap_distance: 1.0,
                            ..Default::default()
                        });
                    }
                }
                if !matches!(self.viewport_mode, ViewportMode::None)
                    && let Some(entity_id) = self.selected_entity
                {
                    {
                        if let Ok(mut q) = self.world.query_one::<&mut Transform>(*entity_id)
                            && let Some(transform) = q.get()
                        {
                            let was_focused = cfg.is_focused;
                            cfg.is_focused = self.gizmo.is_focused();

                            if cfg.is_focused && !was_focused {
                                cfg.old_pos = *transform;
                            }

                            let gizmo_transform =
                                        transform_gizmo_egui::math::Transform::from_scale_rotation_translation(
                                            transform.scale,
                                            transform.rotation,
                                            transform.position,
                                        );

                            if let Some((_result, new_transforms)) =
                                self.gizmo.interact(ui, &[gizmo_transform])
                                && let Some(new_transform) = new_transforms.first()
                            {
                                transform.position = new_transform.translation.into();
                                transform.rotation = new_transform.rotation.into();
                                transform.scale = new_transform.scale.into();
                            }

                            if was_focused && !cfg.is_focused {
                                let transform_changed = cfg.old_pos.position != transform.position
                                    || cfg.old_pos.rotation != transform.rotation
                                    || cfg.old_pos.scale != transform.scale;

                                if transform_changed {
                                    UndoableAction::push_to_undo(
                                        self.undo_stack,
                                        UndoableAction::Transform(*entity_id, cfg.old_pos),
                                    );
                                    log::debug!("Pushed transform action to stack");
                                }
                            }
                        }
                    }
                }
            }
            EditorTab::ModelEntityList => {
                let (_response, action) = egui_ltreeview::TreeView::new(egui::Id::new(
                    "model_entity_list",
                ))
                .show(ui, |builder| {
                    let current_scene_name = {
                        PROJECT
                            .read()
                            .last_opened_scene
                            .clone()
                            .unwrap_or("Scene".to_string())
                    };
                    builder.node(
                        NodeBuilder::dir(u64::MAX)
                            .label(format!("Scene: {}", current_scene_name))
                            .context_menu(|ui| {
                                if ui.button("New Empty Entity").clicked() {
                                    self.world.spawn((Label::new("Blank Entity"),));
                                    ui.close();
                                }
                            }),
                    );
                    // the root scene must be the biggest number possible to remove any ambiguity

                    fn add_entity_to_tree(
                        builder: &mut TreeViewBuilder<u64>,
                        entity: Entity,
                        world: &mut World,
                        registry: &ComponentRegistry,
                        cfg: &mut StaticallyKept,
                    ) -> anyhow::Result<()> {
                        let entity_id = entity.to_bits().get();
                        let label = if let Ok(mut q) = world.query_one::<&Label>(entity)
                            && let Some(label) = q.get()
                        {
                            label.clone()
                        } else {
                            anyhow::bail!(
                                "This entity [{}] is expected to contain Label",
                                entity_id
                            );
                        };

                        builder.node(
                            NodeBuilder::dir(entity_id)
                                .label(label.as_str())
                                .context_menu(|ui| {
                                    ui.menu_button("New", |ui| {
                                        if ui.button("Child").clicked() {
                                            let child = world.spawn((Label::new("New Entity"),));
                                            Hierarchy::set_parent(world, child, entity);
                                            ui.close();
                                        }
                                    });
                                    ui.menu_button("Add Component", |ui| {
                                        for (id, name) in registry.iter_available_components() {
                                            let short_name = name.split("::").last().unwrap_or(name);
                                            if ui.button(short_name).clicked() {
                                                if let Some(comp) = registry.create_default_component(id) {
                                                    let mut builder = EntityBuilder::new();
                                                    if let Ok(_) = registry.deserialize_into_builder(comp.as_ref(), &mut builder) {
                                                        let _ = world.insert(entity, builder.build());
                                                    }
                                                }
                                                ui.close();
                                            }
                                        }
                                    });
                                }),
                        );

                        let components = registry.extract_all_components(world, entity);

                        for component in components.iter() {
                            let Some(component_type_id) =
                                registry.id_for_component(component.as_ref())
                            else {
                                log_once::warn_once!(
                                    "Component '{}' missing registry id, skipping tree entry",
                                    component.type_name()
                                );
                                continue;
                            };
                            let component_node_id =
                                cfg.component_node_id(entity, component_type_id);
                            let component_name = component.type_name();
                            let short_name = component_name.split("::").last().unwrap_or(component_name);
                            let display = format!("{} (id #{component_type_id})", short_name);

                            builder.node(
                                NodeBuilder::leaf(component_node_id)
                                    .label(display)
                                    .context_menu(|ui| {
                                        if ui.button("Remove Component").clicked() {
                                            registry.remove_component_by_id(world, entity, component_type_id);
                                            ui.close();
                                        }
                                    }),
                            );
                        }

                        let children_entities = if let Ok(children) = world.get::<&Children>(entity) {
                            children.children().to_vec()
                        } else {
                            Vec::new()
                        };

                        for child in children_entities {
                            if let Err(e) =
                                add_entity_to_tree(builder, child, world, registry, cfg)
                            {
                                log_once::error_once!(
                                    "Failed to add child entity to tree, skipping: {}",
                                    e
                                );
                                continue;
                            }
                        }

                        builder.close_dir();
                        Ok(())
                    }

                    let root_entities: Vec<Entity> = self.world.query::<()>().without::<&Parent>().iter().map(|(e, _)| e).collect();

                    for entity in root_entities {
                        if let Err(e) = add_entity_to_tree(
                            builder,
                            entity,
                            &mut self.world,
                            &self.component_registry,
                            &mut cfg,
                        ) {
                            log_once::error_once!(
                                "Failed to add child entity to tree, skipping: {}",
                                e
                            );
                        }
                    }

                    builder.close_dir();
                });

                for i in action {
                    match i {
                        egui_ltreeview::Action::SetSelected(items) => {
                            self.handle_tree_selection(&mut cfg, &items);
                        }
                        egui_ltreeview::Action::Move(drag_and_drop) => {
                            self.handle_tree_move(&mut cfg, &drag_and_drop);
                        }
                        egui_ltreeview::Action::Drag(drag_and_drop) => {
                            self.handle_tree_drag(&mut cfg, &drag_and_drop);
                        }
                        egui_ltreeview::Action::Activate(activate) => {
                            self.handle_tree_activate(&mut cfg, &activate);
                        }
                        egui_ltreeview::Action::DragExternal(_drag_and_drop_external) => {}
                        egui_ltreeview::Action::MoveExternal(_drag_and_drop_external) => {}
                    }
                }
            }
            EditorTab::AssetViewer => {
                self.show_asset_viewer(ui);
            }
            EditorTab::ResourceInspector => {
                if let Some(entity) = self.selected_entity {
                    let mut local_set_initial_camera = false;
                    if let Ok(mut q) = self.world.query_one::<(&mut Label,)>(*entity) {
                        if let Some((label,)) = q.get() {
                            label.inspect(
                                entity,
                                &mut cfg,
                                ui,
                                self.undo_stack,
                                self.signal,
                                &mut String::new(),
                            );

                            ui.label(format!("Entity ID: {}", entity.id()));

                            if let Ok(mut q) = self.world.query_one::<&mut MeshRenderer>(*entity)
                                && let Some(e) = q.get()
                            {
                                // entity
                                e.inspect(
                                    entity,
                                    &mut cfg,
                                    ui,
                                    self.undo_stack,
                                    self.signal,
                                    &mut String::new(),
                                );
                            }

                            if let Ok(mut q) = self.world.query_one::<&mut EntityTransform>(*entity)
                                && let Some(t) = q.get()
                            {
                                // transform
                                t.inspect(
                                    entity,
                                    &mut cfg,
                                    ui,
                                    self.undo_stack,
                                    self.signal,
                                    &mut String::new(),
                                );
                            }

                            if let Ok(mut q) = self.world.query_one::<&mut ModelProperties>(*entity)
                                && let Some(props) = q.get()
                            {
                                // properties
                                props.inspect(
                                    entity,
                                    &mut cfg,
                                    ui,
                                    self.undo_stack,
                                    self.signal,
                                    label.as_mut_string(),
                                );
                            }

                            if let Ok(mut q) = self
                                .world
                                .query_one::<(&mut Camera, &mut CameraComponent)>(*entity)
                                && let Some((camera, camera_component)) = q.get()
                            {
                                camera.inspect(
                                    entity,
                                    &mut cfg,
                                    ui,
                                    self.undo_stack,
                                    self.signal,
                                    &mut String::new(),
                                );

                                camera_component.inspect(
                                    entity,
                                    &mut cfg,
                                    ui,
                                    self.undo_stack,
                                    self.signal,
                                    &mut camera.label.clone(),
                                );

                                ui.separator();

                                // camera controller
                                ui.label("Camera Controls:");
                                let mut active_camera = self.active_camera.lock();

                                if active_camera.equivalent(&Some(*entity)) {
                                    ui.label("Status: Currently viewing through camera");
                                } else {
                                    ui.label("Status: Not viewing through this camera");
                                }

                                if ui.button("Set as active camera").clicked() {
                                    *active_camera = Some(*entity);
                                    log::info!(
                                        "Currently viewing from camera angle '{}'",
                                        camera.label
                                    );
                                }

                                if camera_component.starting_camera {
                                    if ui.button("Stop making camera initial").clicked() {
                                        log::debug!("'Stop making camera initial' button clicked");
                                        camera_component.starting_camera = false;
                                        success!("Removed {} from starting camera", camera.label);
                                    }
                                } else if ui.button("Set as initial camera").clicked() {
                                    log::debug!("'Set as initial camera' button clicked");
                                    if matches!(camera_component.camera_type, CameraType::Debug) {
                                        warn!(
                                            "Cannot set any cameras of type 'Debug' to initial camera"
                                        );
                                    } else {
                                        local_set_initial_camera = true
                                    }
                                }
                            }

                            if let Ok(mut q) = self
                                .world
                                .query_one::<(&mut Light, &mut LightComponent)>(*entity)
                                && let Some((_light, comp)) = q.get()
                            {
                                comp.inspect(
                                    entity,
                                    &mut cfg,
                                    ui,
                                    self.undo_stack,
                                    self.signal,
                                    &mut String::new(),
                                );
                            }

                            if let Ok(mut q) = self.world.query_one::<&mut ScriptComponent>(*entity)
                                && let Some(script) = q.get()
                            {
                                script.inspect(
                                    entity,
                                    &mut cfg,
                                    ui,
                                    self.undo_stack,
                                    self.signal,
                                    label.as_mut_string(),
                                );
                            }

                            if let Some(t) = cfg.label_last_edit
                                && t.elapsed() >= Duration::from_millis(500)
                            {
                                if let Some(ent) = cfg.old_label_entity.take()
                                    && let Some(orig) = cfg.label_original.take()
                                {
                                    UndoableAction::push_to_undo(
                                        self.undo_stack,
                                        UndoableAction::Label(ent, orig),
                                    );
                                    log::debug!(
                                        "Pushed label change to undo stack after 500ms debounce period"
                                    );
                                }
                                cfg.label_last_edit = None;
                            }
                        }
                    } else {
                        log_once::debug_once!("Unable to query entity inside resource inspector");
                    }

                    if local_set_initial_camera {
                        for (id, comp) in self.world.query::<&mut CameraComponent>().iter() {
                            comp.starting_camera = false;
                            self.undo_stack
                                .push(UndoableAction::RemoveStartingCamera(id))
                        }

                        if let Ok(comp) = self.world.query_one_mut::<&mut CameraComponent>(*entity)
                        {
                            success!("This camera is currently set as the initial camera");
                            comp.starting_camera = true;
                        }
                    }
                } else {
                    ui.label("No entity selected, therefore no info to provide. Go on, what are you waiting for? Click an entity!");
                }
            }
            EditorTab::Plugin(dock_info) => {
                if self.editor.is_null() {
                    panic!("Editor pointer is null, unexpected behaviour");
                }
                let editor = unsafe { &mut *self.editor };
                if let Some((_, plugin)) = self.plugin_registry.plugins.get_index_mut(*dock_info) {
                    plugin.ui(ui, editor);
                } else {
                    ui.colored_label(
                        egui::Color32::RED,
                        format!("Plugin at index '{}' not found", *dock_info),
                    );
                }
            }
            EditorTab::ErrorConsole => {
                fn analyse_error(log: &Vec<String>) -> Vec<ConsoleItem> {
                    fn parse_compiler_location(
                        line: &str,
                    ) -> Option<(ErrorLevel, PathBuf, String)> {
                        let trimmed = line.trim_start();
                        let (error_level, rest) =
                            if let Some(r) = trimmed.strip_prefix("e: file:///") {
                                (ErrorLevel::Error, r)
                            } else if let Some(r) = trimmed.strip_prefix("w: file:///") {
                                (ErrorLevel::Warn, r)
                            } else {
                                return None;
                            };

                        let location = rest.split_whitespace().next()?;

                        let mut segments = location.rsplitn(3, ':');
                        let column = segments.next()?;
                        let row = segments.next()?;
                        let path = segments.next()?;

                        Some((error_level, PathBuf::from(path), format!("{row}:{column}")))
                    }

                    let mut list: Vec<ConsoleItem> = Vec::new();
                    let index = 0;
                    for line in log {
                        if line.contains("The required library") {
                            list.push(ConsoleItem {
                                error_level: ErrorLevel::Error,
                                msg: line.clone(),
                                file_location: None,
                                line_ref: None,
                                id: index + 1,
                            });
                        }

                        if let Some((error_level, path, loc)) = parse_compiler_location(line) {
                            list.push(ConsoleItem {
                                error_level,
                                msg: line.clone(),
                                file_location: Some(path),
                                line_ref: Some(loc),
                                id: index + 1,
                            });
                        }

                        // thats it for now
                    }
                    list
                }

                let logs = analyse_error(&self.build_logs);

                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        if logs.is_empty() {
                            ui.label("Build output will appear here once available.");
                            return;
                        }

                        for item in &logs {
                            let (bg_color, text_color) = match item.error_level {
                                ErrorLevel::Error => (
                                    egui::Color32::from_rgb(60, 20, 20),
                                    egui::Color32::from_rgb(255, 200, 200),
                                ),
                                ErrorLevel::Warn => (
                                    egui::Color32::from_rgb(40, 40, 10),
                                    egui::Color32::from_rgb(255, 255, 200),
                                ),
                            };

                            let available_width = ui.available_width();
                            let frame = egui::Frame::new()
                                .inner_margin(Margin::symmetric(8, 6))
                                .fill(bg_color)
                                .stroke(egui::Stroke::new(1.0, text_color));

                            let response = frame
                                .show(ui, |ui| {
                                    ui.set_width(available_width - 10.0);
                                    ui.horizontal(|ui| {
                                        ui.label(RichText::new(&item.msg).color(text_color));
                                    });
                                })
                                .response;

                            if response.clicked() {
                                log::debug!("Log item clicked: {}", &item.id);
                                if let (Some(path), Some(loc)) =
                                    (&item.file_location, &item.line_ref)
                                {
                                    let location_arg = format!("{}:{}", path.display(), loc);

                                    match std::process::Command::new("code")
                                        .args(["-g", &location_arg])
                                        .spawn()
                                        .map(|_| ())
                                    {
                                        Ok(()) => {
                                            log::info!(
                                                "Launched Visual Studio Code at the error: {}",
                                                &location_arg
                                            );
                                        }
                                        Err(e) => {
                                            warn!(
                                                "Failed to open '{}' in VS Code: {}",
                                                location_arg, e
                                            );
                                        }
                                    }
                                }
                            }

                            ui.add_space(4.0);
                        }
                    });
            }
        }
    }
}

impl<'a> EditorTabViewer<'a> {
    fn show_asset_viewer(&mut self, ui: &mut egui::Ui) {
        let project_root = {
            let project = PROJECT.read();
            if project.project_path.as_os_str().is_empty() {
                ui.label("Open a project to browse assets.");
                return;
            }
            project.project_path.clone()
        };

        egui_ltreeview::TreeView::new(egui::Id::new("asset_viewer")).show(ui, |builder| {
            builder.node(Self::dir_node("euca://"));
            Self::build_resource_branch(builder, &project_root);
            Self::build_scripts_branch(builder, &project_root);
            Self::build_scene_branch(builder, &project_root);
            builder.close_dir();
        });
    }

    fn build_resource_branch(builder: &mut TreeViewBuilder<u64>, project_root: &Path) {
        let label = "euca://resources";
        builder.node(Self::dir_node_labeled(label, "resources"));
        let resources_root = project_root.join("resources");
        if resources_root.exists() {
            Self::walk_resource_directory(builder, &resources_root, &resources_root);
        } else {
            Self::add_placeholder_leaf(builder, "euca://resources/missing", "missing");
        }
        builder.close_dir();
    }

    fn walk_resource_directory(
        builder: &mut TreeViewBuilder<u64>,
        base_path: &Path,
        current_path: &Path,
    ) {
        let entries = match Self::sorted_entries(current_path) {
            Ok(entries) => entries,
            Err(err) => {
                log::warn!(
                    "Unable to enumerate resources at '{}': {}",
                    current_path.display(),
                    err
                );
                return;
            }
        };

        for entry in entries {
            let full_label = Self::resource_label(base_path, &entry.path);
            if entry.is_dir {
                builder.node(Self::dir_node_labeled(&full_label, &entry.name));
                Self::walk_resource_directory(builder, base_path, &entry.path);
                builder.close_dir();
            } else {
                if entry.name.eq_ignore_ascii_case("resources.eucc") {
                    continue;
                }
                builder.node(Self::leaf_node_labeled(&full_label, &entry.name));
            }
        }
    }

    fn resource_label(base_path: &Path, path: &Path) -> String {
        let relative = path
            .strip_prefix(base_path)
            .map(|rel| rel.to_string_lossy().replace('\\', "/"))
            .unwrap_or_else(|_| path.to_string_lossy().replace('\\', "/"));
        if relative.is_empty() {
            "euca://resources".to_string()
        } else {
            format!("euca://resources/{}", relative)
        }
    }

    fn build_scripts_branch(builder: &mut TreeViewBuilder<u64>, project_root: &Path) {
        let label = "euca://scripts";
        builder.node(Self::dir_node_labeled(label, "scripts"));
        let scripts_root = project_root.join("src");
        if !scripts_root.exists() {
            Self::add_placeholder_leaf(builder, "euca://scripts/missing", "missing");
            builder.close_dir();
            return;
        }

        let entries = match Self::sorted_entries(&scripts_root) {
            Ok(entries) => entries,
            Err(err) => {
                log::warn!(
                    "Unable to enumerate scripts at '{}': {}",
                    scripts_root.display(),
                    err
                );
                builder.close_dir();
                return;
            }
        };

        let mut had_content = false;
        for entry in entries {
            if entry.is_dir {
                let source_label = format!("{}/{}", label, entry.name);
                builder.node(Self::dir_node_labeled(&source_label, &entry.name));
                if Self::build_script_source_set(builder, &entry.path, &source_label) {
                    had_content = true;
                }
                builder.close_dir();
            } else if !entry.name.eq_ignore_ascii_case("source.eucc") {
                let file_label = format!("{}/{}", label, entry.name);
                builder.node(Self::leaf_node_labeled(&file_label, &entry.name));
                had_content = true;
            }
        }

        if !had_content {
            Self::add_placeholder_leaf(builder, "euca://scripts/empty", "empty");
        }

        builder.close_dir();
    }

    fn build_script_source_set(
        builder: &mut TreeViewBuilder<u64>,
        source_path: &Path,
        source_label: &str,
    ) -> bool {
        let entries = match Self::sorted_entries(source_path) {
            Ok(entries) => entries,
            Err(err) => {
                log::warn!(
                    "Unable to enumerate source set at '{}': {}",
                    source_path.display(),
                    err
                );
                Self::add_placeholder_leaf(builder, &format!("{source_label}/unreadable"), "unreadable");
                return true;
            }
        };

        let mut had_content = false;
        for entry in entries {
            if entry.is_dir {
                if entry.name.eq_ignore_ascii_case("kotlin") {
                    if Self::build_kotlin_tree(builder, &entry.path, source_label) {
                        had_content = true;
                    }
                } else {
                    let child_label = format!("{}/{}", source_label, entry.name);
                    builder.node(Self::dir_node_labeled(&child_label, &entry.name));
                    Self::build_plain_directory(builder, &entry.path, &child_label);
                    builder.close_dir();
                    had_content = true;
                }
            } else if !entry.name.eq_ignore_ascii_case("source.eucc") {
                let file_label = format!("{}/{}", source_label, entry.name);
                builder.node(Self::leaf_node_labeled(&file_label, &entry.name));
                had_content = true;
            }
        }

        if !had_content {
            Self::add_placeholder_leaf(builder, &format!("{source_label}/empty"), "empty");
            had_content = true;
        }

        had_content
    }

    fn build_plain_directory(
        builder: &mut TreeViewBuilder<u64>,
        dir_path: &Path,
        parent_label: &str,
    ) {
        let entries = match Self::sorted_entries(dir_path) {
            Ok(entries) => entries,
            Err(err) => {
                log::warn!(
                    "Unable to enumerate directory '{}': {}",
                    dir_path.display(),
                    err
                );
                Self::add_placeholder_leaf(builder, &format!("{parent_label}/unreadable"), "unreadable");
                return;
            }
        };

        if entries.is_empty() {
            Self::add_placeholder_leaf(builder, &format!("{parent_label}/empty"), "empty");
            return;
        }

        for entry in entries {
            let child_label = format!("{}/{}", parent_label, entry.name);
            if entry.is_dir {
                builder.node(Self::dir_node_labeled(&child_label, &entry.name));
                Self::build_plain_directory(builder, &entry.path, &child_label);
                builder.close_dir();
            } else {
                builder.node(Self::leaf_node_labeled(&child_label, &entry.name));
            }
        }
    }

    fn build_kotlin_tree(
        builder: &mut TreeViewBuilder<u64>,
        kotlin_path: &Path,
        source_label: &str,
    ) -> bool {
        let entries = match Self::sorted_entries(kotlin_path) {
            Ok(entries) => entries,
            Err(err) => {
                log::warn!(
                    "Unable to enumerate kotlin sources at '{}': {}",
                    kotlin_path.display(),
                    err
                );
                Self::add_placeholder_leaf(builder, &format!("{source_label}/unreadable"), "unreadable");
                return true;
            }
        };

        if entries.is_empty() {
            Self::add_placeholder_leaf(builder, &format!("{source_label}/no_kotlin_files"), "no kotlin files");
            return true;
        }

        let mut had_entries = false;
        for entry in entries {
            if entry.is_dir {
                Self::build_kotlin_package_collapsed(
                    builder,
                    &entry.path,
                    source_label,
                    vec![entry.name.clone()],
                );
                had_entries = true;
            } else {
                let file_id = format!("{}/{}", source_label, entry.name);
                builder.node(Self::leaf_node_labeled(&file_id, &entry.name));
                had_entries = true;
            }
        }

        had_entries
    }

    fn build_kotlin_package_collapsed(
        builder: &mut TreeViewBuilder<u64>,
        dir_path: &Path,
        parent_path_str: &str,
        accumulated_parts: Vec<String>,
    ) {
        let entries = match Self::sorted_entries(dir_path) {
            Ok(entries) => entries,
            Err(err) => {
                let package_suffix = accumulated_parts.join(".");
                let full_path_str = format!("{}/{}", parent_path_str, package_suffix);
                log::warn!(
                    "Unable to enumerate package '{}' ({}): {}",
                    package_suffix,
                    dir_path.display(),
                    err
                );
                Self::add_placeholder_leaf(builder, &format!("{full_path_str}/unreadable"), "unreadable");
                return;
            }
        };

        let subdirs: Vec<&FsEntry> = entries.iter().filter(|e| e.is_dir).collect();
        let files: Vec<&FsEntry> = entries.iter().filter(|e| !e.is_dir).collect();

        if files.is_empty() && subdirs.len() == 1 {
            let subdir = subdirs[0];
            let mut new_parts = accumulated_parts;
            new_parts.push(subdir.name.clone());
            Self::build_kotlin_package_collapsed(builder, &subdir.path, parent_path_str, new_parts);
        } else {
            let package_suffix = accumulated_parts.join(".");
            let full_path_str = format!("{}/{}", parent_path_str, package_suffix);

            builder.node(Self::dir_node_labeled(&full_path_str, &package_suffix));

            for file in files {
                let file_id = format!("{}/{}", full_path_str, file.name);
                builder.node(Self::leaf_node_labeled(&file_id, &file.name));
            }

            for subdir in subdirs {
                Self::build_kotlin_package_collapsed(
                    builder,
                    &subdir.path,
                    &full_path_str,
                    vec![subdir.name.clone()],
                );
            }

            builder.close_dir();
        }
    }

    fn build_scene_branch(builder: &mut TreeViewBuilder<u64>, project_root: &Path) {
        let label = "euca://scenes";
        builder.node(Self::dir_node_labeled(label, "scenes"));
        let scenes_root = project_root.join("scenes");
        if !scenes_root.exists() {
            Self::add_placeholder_leaf(builder, "euca://scenes/missing", "missing");
            builder.close_dir();
            return;
        }

        let entries = match Self::sorted_entries(&scenes_root) {
            Ok(entries) => entries,
            Err(err) => {
                log::warn!(
                    "Unable to enumerate scenes at '{}': {}",
                    scenes_root.display(),
                    err
                );
                Self::add_placeholder_leaf(builder, "euca://scenes/unreadable", "unreadable");
                builder.close_dir();
                return;
            }
        };

        let mut had_entries = false;
        for entry in entries {
            if entry.is_dir {
                let child_label = format!("{}/{}", label, entry.name);
                builder.node(Self::dir_node_labeled(&child_label, &entry.name));
                Self::build_plain_directory(builder, &entry.path, &child_label);
                builder.close_dir();
                had_entries = true;
            } else if entry
                .path
                .extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| ext.eq_ignore_ascii_case("eucs"))
                .unwrap_or(false)
            {
                let file_label = format!("{}/{}", label, entry.name);
                builder.node(Self::leaf_node_labeled(&file_label, &entry.name));
                had_entries = true;
            }
        }

        if !had_entries {
            Self::add_placeholder_leaf(builder, "euca://scenes/no_scenes", "no scenes");
        }

        builder.close_dir();
    }

    fn sorted_entries(path: &Path) -> io::Result<Vec<FsEntry>> {
        let mut entries = Vec::new();
        for entry in fs::read_dir(path)? {
            let entry = entry?;
            let file_type = entry.file_type()?;
            let name = entry.file_name().to_string_lossy().to_string();
            entries.push(FsEntry {
                path: entry.path(),
                name_lower: name.to_lowercase(),
                name,
                is_dir: file_type.is_dir(),
            });
        }

        entries.sort_by(|a, b| match b.is_dir.cmp(&a.is_dir) {
            Ordering::Equal => a.name_lower.cmp(&b.name_lower),
            other => other,
        });

        Ok(entries)
    }

    fn asset_node_id(label: &str) -> u64 {
        let mut hasher = DefaultHasher::new();
        label.hash(&mut hasher);
        let mut id = hasher.finish();
        if id == 0 {
            id = 1;
        }
        id
    }

    fn dir_node<'ui>(label: &str) -> NodeBuilder<'ui, u64> {
        Self::with_icon(NodeBuilder::dir(Self::asset_node_id(label)).label(label.to_string()))
    }

    fn dir_node_labeled<'ui>(id_source: &str, label: &str) -> NodeBuilder<'ui, u64> {
        Self::with_icon(NodeBuilder::dir(Self::asset_node_id(id_source)).label(label.to_string()))
    }

    fn leaf_node_labeled<'ui>(id_source: &str, label: &str) -> NodeBuilder<'ui, u64> {
        Self::with_icon(NodeBuilder::leaf(Self::asset_node_id(id_source)).label(label.to_string()))
    }

    fn with_icon<'ui>(builder: NodeBuilder<'ui, u64>) -> NodeBuilder<'ui, u64> {
        builder.icon(|ui| {egui_extras::install_image_loaders(ui.ctx()); Self::draw_asset_icon(ui)})
    }

    fn draw_asset_icon(ui: &mut egui::Ui) {
        let image = egui::Image::from_bytes("bytes://asset-viewer-icon", NO_TEXTURE)
            .max_size(egui::vec2(14.0, 14.0));
        ui.add(image);
    }

    fn add_placeholder_leaf(builder: &mut TreeViewBuilder<u64>, id_source: &str, label: &str) {
        builder.node(Self::leaf_node_labeled(id_source, label));
    }

    fn handle_tree_selection(&mut self, cfg: &mut StaticallyKept, items: &[u64]) {
        for node_id in items {
            self.resolve_tree_node(cfg, *node_id);
        }
    }

    fn handle_tree_activate(
        &mut self,
        cfg: &mut StaticallyKept,
        activate: &egui_ltreeview::Activate<u64>,
    ) {
        self.handle_tree_selection(cfg, &activate.selected);
    }

    fn handle_tree_drag(
        &mut self,
        cfg: &mut StaticallyKept,
        drag: &egui_ltreeview::DragAndDrop<u64>,
    ) {
        if let Some(&node_id) = drag.source.first() {
            if let Some(selection) = cfg.component_selection(node_id) {
                cfg.pending_component_drag = Some(selection);
                self.inspect_component_selection(cfg, selection);
            }
        }
    }

    fn handle_tree_move(
        &mut self,
        cfg: &mut StaticallyKept,
        drag: &egui_ltreeview::DragAndDrop<u64>,
    ) {
        let selection = cfg.pending_component_drag.take().or_else(|| {
            drag.source
                .first()
                .and_then(|node_id| cfg.component_selection(*node_id))
        });

        if let Some(selection) = selection {
            self.inspect_component_selection(cfg, selection);
            if let Some(target_entity) = Self::entity_from_node_id(drag.target) {
                log::info!(
                    "Component id #{} ready to drop onto entity {:?}",
                    selection.component_type_id,
                    target_entity
                );
            }
        }
    }

    fn resolve_tree_node(&mut self, cfg: &mut StaticallyKept, node_id: u64) {
        if let Some(selection) = cfg.component_selection(node_id) {
            self.inspect_component_selection(cfg, selection);
        } else if let Some(entity) = Self::entity_from_node_id(node_id) {
            *self.selected_entity = Some(entity);
        }
    }

    fn inspect_component_selection(
        &mut self,
        cfg: &mut StaticallyKept,
        selection: ComponentNodeSelection,
    ) {
        cfg.remember_component_lookup(selection);
        let component_id = selection.component_type_id;
        let matches = self
            .component_registry
            .find_components_by_numeric_id(&*self.world, component_id);

        if matches.is_empty() {
            log::warn!("Component id #{} not found in world", component_id);
            return;
        }

        for (entity, component) in matches {
            log::debug!(
                "Serializable component '{}' (id #{}) attached to entity {:?}",
                component.type_name(),
                component_id,
                entity
            );
        }
    }

    fn entity_from_node_id(node_id: u64) -> Option<Entity> {
        if node_id == u64::MAX {
            None
        } else {
            Entity::from_bits(node_id)
        }
    }
}

#[derive(Clone)]
struct FsEntry {
    path: PathBuf,
    name: String,
    name_lower: String,
    is_dir: bool,
}

#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
// todo: provide a purpose to RemoveComponent
pub enum EditorTabMenuAction {
    ImportResource,
    RefreshAssets,
    AddEntity,
    DeleteEntity,
    AddComponent,
    RemoveComponent,
    ViewportOption,
}
