use super::*;
use std::{collections::HashSet, sync::LazyLock};

use dropbear_engine::{entity::Transform, lighting::{Light, LightComponent}};
use egui::{self, CollapsingHeader};
use egui_dock_fork::TabViewer;
use egui_extras;
use log;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use transform_gizmo_egui::{
    math::{DMat4, DVec3}, EnumSet, Gizmo, GizmoConfig, GizmoExt, GizmoMode
};

use crate::{
    APP_INFO,
    camera::CameraAction,
    editor::scene::PENDING_SPAWNS,
    states::{EntityNode, Node, RESOURCES, ResourceType},
    utils::PendingSpawn,
};
use crate::editor::component::Component;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum EditorTab {
    AssetViewer,       // bottom side,
    ResourceInspector, // left side,
    ModelEntityList,   // right side,
    Viewport,          // middle,
}

pub struct EditorTabViewer<'a> {
    pub view: egui::TextureId,
    pub nodes: Vec<EntityNode>,
    pub tex_size: Extent3d,
    pub gizmo: &'a mut Gizmo,
    pub world: &'a mut hecs::World,
    pub selected_entity: &'a mut Option<hecs::Entity>,
    pub viewport_mode: &'a mut ViewportMode,
    pub undo_stack: &'a mut Vec<UndoableAction>,
    pub signal: &'a mut Signal,
    pub camera_manager: &'a mut CameraManager,
    pub gizmo_mode: &'a mut EnumSet<GizmoMode>,
    pub editor_mode: &'a mut EditorState,
}

impl<'a> EditorTabViewer<'a> {
    fn spawn_entity_at_pos(
        &mut self,
        asset: &DraggedAsset,
        position: glam::DVec3,
        properties: Option<ModelProperties>,
    ) -> anyhow::Result<()> {
        let mut transform = Transform::default();
        transform.position = position;
        {
            let mut pending_spawns = PENDING_SPAWNS.lock();
            if let Some(props) = properties {
                pending_spawns.push(PendingSpawn {
                    asset_path: asset.path.clone(),
                    asset_name: asset.name.clone(),
                    transform,
                    properties: props,
                });
            } else {
                pending_spawns.push(PendingSpawn {
                    asset_path: asset.path.clone(),
                    asset_name: asset.name.clone(),
                    transform,
                    properties: ModelProperties::default(),
                });
            }
            Ok(())
        }
    }

    #[allow(dead_code)]
    // purely for debug, nothing else...
    fn debug_camera_state(&self) {
        let camera = self.camera_manager.get_active().unwrap();
        log::debug!("Camera state:");
        log::debug!("  Eye: {:?}", camera.eye);
        log::debug!("  Target: {:?}", camera.target);
        log::debug!("  Up: {:?}", camera.up);
        log::debug!("  FOV Y: {}", camera.fov_y);
        log::debug!("  Aspect: {}", camera.aspect);
        log::debug!("  Z Near: {}", camera.znear);
        log::debug!("  Proj Mat finite: {}", camera.proj_mat.is_finite());
        log::debug!("  View Mat finite: {}", camera.view_mat.is_finite());
    }
}

#[derive(Clone, Debug)]
pub struct DraggedAsset {
    pub name: String,
    pub path: PathBuf,
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
}

impl StaticallyKept {}

impl<'a> TabViewer for EditorTabViewer<'a> {
    type Tab = EditorTab;

    fn title(&mut self, tab: &mut Self::Tab) -> egui::WidgetText {
        match tab {
            EditorTab::Viewport => "Viewport".into(),
            EditorTab::ModelEntityList => "Model/Entity List".into(),
            EditorTab::AssetViewer => "Asset Viewer".into(),
            EditorTab::ResourceInspector => "Resource Inspector".into(),
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, tab: &mut Self::Tab) {
        let mut cfg = TABS_GLOBAL.lock();
        
        ui.ctx().input(|i| {
            if i.pointer.button_pressed(egui::PointerButton::Secondary) {
                if let Some(pos) = i.pointer.hover_pos() {
                    if ui.available_rect_before_wrap().contains(pos) {
                        cfg.show_context_menu = true;
                        cfg.context_menu_pos = pos;
                        cfg.context_menu_tab = Some(tab.clone());
                    }
                }
            }
        });

        match tab {
            EditorTab::Viewport => {
                let camera = self.camera_manager.get_active().unwrap();

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
                    egui::vec2(display_width, display_height)
                );

                let (_rect, _response) = ui.allocate_exact_size(
                    available_size,
                    egui::Sense::click_and_drag()
                );

                let _image_response = ui.allocate_rect(image_rect, egui::Sense::click_and_drag());

                ui.scope_builder(egui::UiBuilder::new().max_rect(image_rect), |ui| {
                    ui.add_sized(
                        [display_width, display_height],
                        egui::Image::new((
                            self.view,
                            [display_width, display_height].into(),
                        ))
                        .fit_to_exact_size([display_width, display_height].into())
                    )
                });

                let image_response = ui.interact(image_rect, ui.id().with("viewport_image"), egui::Sense::click_and_drag());

                if image_response.clicked() {
                    if let Some(click_pos) = ui.ctx().input(|i| i.pointer.interact_pos()) {
                        let viewport_rect = image_response.rect;
                        let local_pos = click_pos - viewport_rect.min;

                        let ndc_x = (2.0 * local_pos.x / viewport_rect.width()) - 1.0;
                        let ndc_y = 1.0 - (2.0 * local_pos.y / viewport_rect.height());

                        let view_matrix =
                            glam::DMat4::look_at_lh(camera.eye, camera.target, camera.up);

                        let proj_matrix = glam::DMat4::perspective_lh(
                            camera.fov_y.to_radians(),
                            camera.aspect,
                            camera.znear,
                            camera.zfar,
                        );

                        if !view_matrix.is_finite() {
                            log::error!("Invalid view matrix");
                            return;
                        }
                        if !proj_matrix.is_finite() {
                            log::error!("Invalid projection matrix");
                            return;
                        }

                        let view_proj = proj_matrix * view_matrix;
                        let inv_view_proj = view_proj.inverse();

                        if !inv_view_proj.is_finite() {
                            log::error!("Cannot invert view-projection matrix");
                            return;
                        }

                        let ray_start_ndc = glam::DVec4::new(ndc_x as f64, ndc_y as f64, 0.0, 1.0);
                        let ray_end_ndc = glam::DVec4::new(ndc_x as f64, ndc_y as f64, 1.0, 1.0);

                        let ray_start_world = inv_view_proj * ray_start_ndc;
                        let ray_end_world = inv_view_proj * ray_end_ndc;

                        if ray_start_world.w == 0.0 || ray_end_world.w == 0.0 {
                            log::error!("Invalid homogeneous coordinates");
                            return;
                        }

                        let ray_start = ray_start_world.truncate() / ray_start_world.w;
                        let ray_end = ray_end_world.truncate() / ray_end_world.w;

                        if !ray_start.is_finite() || !ray_end.is_finite() {
                            log::error!(
                                "Invalid ray points - start: {:?}, end: {:?}",
                                ray_start,
                                ray_end
                            );
                            return;
                        }

                        let ray_direction = (ray_end - ray_start).normalize();

                        if !ray_direction.is_finite() {
                            log::error!("Invalid ray direction: {:?}", ray_direction);
                            return;
                        }

                        let mut closest_distance = f64::INFINITY;
                        let mut selected_entity_id: Option<hecs::Entity> = None;
                        let mut entity_count = 0;

                        for (entity_id, (transform, _)) in
                            self.world.query::<(&Transform, &AdoptedEntity)>().iter()
                        {
                            entity_count += 1;
                            let entity_pos = transform.position;
                            let sphere_radius = transform.scale.max_element() * 1.5;

                            let to_sphere = entity_pos - ray_start;
                            let projection = to_sphere.dot(ray_direction);

                            if projection > 0.0 {
                                let closest_point = ray_start + ray_direction * projection;
                                let distance_to_sphere = (closest_point - entity_pos).length();

                                if distance_to_sphere <= sphere_radius {
                                    let discriminant = sphere_radius * sphere_radius
                                        - distance_to_sphere * distance_to_sphere;
                                    if discriminant >= 0.0 {
                                        let intersection_distance =
                                            projection - discriminant.sqrt();

                                        if intersection_distance < closest_distance {
                                            closest_distance = intersection_distance;
                                            selected_entity_id = Some(entity_id);
                                        }
                                    }
                                }
                            }
                        }

                        log::debug!("Total entities checked: {}", entity_count);

                        if !matches!(self.editor_mode, EditorState::Playing) {
                            if let Some(entity_id) = selected_entity_id {
                                *self.selected_entity = Some(entity_id);
                                log::debug!("Selected entity: {:?}", entity_id);
                            } else {
                                // *self.selected_entity = None;
                                if entity_count == 0 {
                                    log::debug!("No entities in world to select");
                                } else {
                                    log::debug!(
                                        "No entity hit by ray (checked {} entities)",
                                        entity_count
                                    );
                                }
                            }
                        }
                    }
                }

                let snapping = ui.input(|input| input.modifiers.shift);

                // Note to self: fuck you >:(
                // Note to self: ok wow thats pretty rude im trying my best ＞﹏＜
                // Note to self: finally holy shit i got it working
                self.gizmo.update_config(GizmoConfig {
                    view_matrix: DMat4::look_at_lh(
                        DVec3::new(
                            camera.eye.x as f64,
                            camera.eye.y as f64,
                            camera.eye.z as f64,
                        ),
                        DVec3::new(
                            camera.target.x as f64,
                            camera.target.y as f64,
                            camera.target.z as f64,
                        ),
                        DVec3::new(camera.up.x as f64, camera.up.y as f64, camera.up.z as f64),
                    )
                    .into(),
                    projection_matrix: DMat4::perspective_infinite_reverse_lh(
                        camera.fov_y as f64,
                        display_width as f64 / display_height as f64,
                        camera.znear as f64,
                    )
                    .into(),
                    viewport: image_rect,
                    modes: *self.gizmo_mode,
                    orientation: transform_gizmo_egui::GizmoOrientation::Global,
                    snapping,
                    ..Default::default()
                });

                if !matches!(self.viewport_mode, crate::utils::ViewportMode::None) {
                    if let Some(entity_id) = self.selected_entity {
                        if let Ok(transform) =
                            self.world.query_one_mut::<&mut Transform>(*entity_id)
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
                            {
                                if let Some(new_transform) = new_transforms.first() {
                                    transform.position = new_transform.translation.into();
                                    transform.rotation = new_transform.rotation.into();
                                    transform.scale = new_transform.scale.into();
                                }
                            }

                            if was_focused && !cfg.is_focused {
                                let transform_changed = cfg.old_pos.position != transform.position
                                    || cfg.old_pos.rotation != transform.rotation
                                    || cfg.old_pos.scale != transform.scale;

                                if transform_changed {
                                    UndoableAction::push_to_undo(
                                        &mut self.undo_stack,
                                        UndoableAction::Transform(
                                            entity_id.clone(),
                                            cfg.old_pos.clone(),
                                        ),
                                    );
                                    log::debug!("Pushed transform action to stack");
                                }
                            }
                        }
                    }
                }
            }
            EditorTab::ModelEntityList => {
                ui.label("Model/Entity List");
                show_entity_tree(
                    ui,
                    &mut self.nodes,
                    &mut self.selected_entity,
                    "Model Entity Asset List",
                );
            }
            EditorTab::AssetViewer => {
                egui_extras::install_image_loaders(ui.ctx());

                let mut assets: Vec<(String, String, PathBuf, ResourceType)> = Vec::new();
                {
                    let res = RESOURCES.read().unwrap();
                    egui_extras::install_image_loaders(ui.ctx());

                    fn recursive_search_nodes_and_attach_thumbnail(
                        res: &Vec<Node>,
                        assets: &mut Vec<(String, String, PathBuf, ResourceType)>,
                        logged: &mut HashSet<String>,
                    ) {
                        for node in res {
                            match node {
                                Node::File(file) => {
                                    if !logged.contains(&file.name) {
                                        logged.insert(file.name.clone());
                                        log::debug!(
                                            "Adding image for {} of type {}",
                                            file.name,
                                            file.resource_type.as_ref().unwrap()
                                        );
                                    }
                                    if let Some(ref res_type) = file.resource_type {
                                        match res_type {
                                            crate::states::ResourceType::Model => {
                                                let ad_dir = app_dirs2::get_app_root(
                                                    app_dirs2::AppDataType::UserData,
                                                    &APP_INFO,
                                                )
                                                .unwrap();

                                                let model_thumbnail =
                                                    ad_dir.join(format!("{}.png", file.name));

                                                if !model_thumbnail.exists() {
                                                    // gen image
                                                    log::debug!(
                                                        "Model thumbnail [{}] does not exist, generating one now",
                                                        file.name
                                                    );
                                                    let mut model = match model_to_image::ModelToImageBuilder::new(&file.path)
                                                    .with_size((600, 600))
                                                    .build() {
                                                        Ok(v) => v,
                                                        Err(e) => panic!("Error occurred while loading file from path: {}", e),
                                                    };
                                                    if let Err(e) =
                                                        model.render().unwrap().write_to(Some(
                                                            &ad_dir
                                                                .join(format!("{}.png", file.name)),
                                                        ))
                                                    {
                                                        log::error!(
                                                            "Failed to write model thumbnail for {}: {}",
                                                            file.name,
                                                            e
                                                        );
                                                    }
                                                }

                                                let image_uri =
                                                    model_thumbnail.to_string_lossy().to_string();

                                                assets.push((
                                                    format!("file://{}", image_uri),
                                                    file.name.clone(),
                                                    file.path.clone(),
                                                    res_type.clone(),
                                                ))
                                            }
                                            ResourceType::Texture => assets.push((
                                                format!(
                                                    "file://{}",
                                                    file.path.to_string_lossy().to_string()
                                                ),
                                                file.name.clone(),
                                                file.path.clone(),
                                                res_type.clone(),
                                            )),
                                            _ => {
                                                if file
                                                    .path
                                                    .clone()
                                                    .extension()
                                                    .unwrap()
                                                    .to_str()
                                                    .unwrap()
                                                    .contains("euc")
                                                {
                                                    continue;
                                                }
                                                assets.push((
                                                    "NO_TEXTURE".into(),
                                                    file.name.clone(),
                                                    file.path.clone(),
                                                    res_type.clone(),
                                                ))
                                            }
                                        }
                                    }
                                }
                                Node::Folder(folder) => {
                                    recursive_search_nodes_and_attach_thumbnail(
                                        &folder.nodes,
                                        assets,
                                        logged,
                                    )
                                }
                            }
                        }
                    }

                    let mut logged = LOGGED.lock();
                    recursive_search_nodes_and_attach_thumbnail(
                        &res.nodes,
                        &mut assets,
                        &mut logged,
                    );
                }

                egui::ScrollArea::vertical().show(ui, |ui| {
                    let max_columns = 6;
                    let available_width = ui.clip_rect().width() - ui.spacing().indent;
                    let margin = 16.0;
                    let usable_width = available_width - margin;
                    let label_height = 20.0;
                    let padding = 8.0;
                    let min_card_width = 60.0;

                    let mut columns = max_columns;
                    for test_columns in (1..=max_columns).rev() {
                        let card_width = usable_width / test_columns as f32;
                        if card_width >= min_card_width {
                            columns = test_columns;
                            break;
                        }
                    }

                    let card_width = usable_width / columns as f32;
                    let image_size = (card_width - label_height - padding).max(32.0);
                    let card_height = image_size + label_height + padding;

                    for row_start in (0..assets.len()).step_by(columns) {
                        let row_end = (row_start + columns).min(assets.len());
                        let row_items = &mut assets[row_start..row_end];

                        ui.horizontal(|ui| {
                            ui.set_max_width(usable_width);

                            egui_dnd::dnd(ui, format!("asset_row_{}", row_start / columns))
                                .show_vec(
                                    row_items,
                                    |ui, (image, asset_name, asset_path, asset_type), handle, state| {
                                        let card_size = egui::vec2(card_width, card_height);
                                        handle.ui(ui, |ui| {
                                            let (rect, card_response) = ui.allocate_exact_size(
                                                card_size,
                                                egui::Sense::click(),
                                            );

                                            let mut card_ui = ui.new_child(
                                                egui::UiBuilder::new().max_rect(rect).layout(
                                                    egui::Layout::top_down(egui::Align::Center),
                                                ),
                                            );

                                            let image_response = card_ui.add_sized(
                                                [image_size, image_size],
                                                egui::ImageButton::new(image.clone()).frame(false),
                                            );

                                            let is_hovered = card_response.hovered() || image_response.hovered() || state.dragged;
                                            let is_d_clicked = card_response.double_clicked() || image_response.double_clicked();

                                            if is_d_clicked {
                                                if matches!(asset_type, ResourceType::Model) {
                                                    let spawn_position = self.camera_manager.get_active().unwrap().eye;
                                                    let asset = DraggedAsset {
                                                        name: asset_name.clone(),
                                                        path: asset_path.clone(),
                                                        // asset_type: asset_type.clone(),
                                                    };

                                                    match self.spawn_entity_at_pos(&asset, spawn_position, None) {
                                                        Ok(()) => {
                                                            log::debug!("double click spawned {} at camera pos {:?}",
                                                                asset.name, spawn_position
                                                            );

                                                            crate::success!("Spawned {} at camera", asset.name);
                                                        }
                                                        Err(e) => {
                                                            log::error!(
                                                            "Failed to spawn {} at camera: {}",
                                                            asset.name,
                                                            e);

                                                            crate::fatal!("Failed to spawn {}: {}",
                                                                        asset.name, e);
                                                        }
                                                    }
                                                }
                                            }

                                            if is_hovered || state.dragged {
                                                ui.painter().rect_filled(
                                                    rect,
                                                    6.0,
                                                    if state.dragged {
                                                        egui::Color32::from_rgb(80, 80, 100)
                                                    } else {
                                                        egui::Color32::from_rgb(60, 60, 80)
                                                    },
                                                );
                                            }

                                            card_ui.vertical_centered(|ui| {
                                                ui.label(
                                                    egui::RichText::new(asset_name.clone())
                                                        .strong()
                                                        .color(egui::Color32::WHITE),
                                                );
                                            });
                                        });
                                    },
                                );
                        });
                        ui.add_space(8.0);
                    }
                });
            }
            EditorTab::ResourceInspector => {
                if let Some(entity) = self.selected_entity {
                    if let Ok((e, transform, _props, script)) = self
                    .world
                    .query_one_mut::<(&mut AdoptedEntity, &mut Transform, &ModelProperties, Option<&mut ScriptComponent>)>(*entity) {
                        let label = e.label().clone();

                        e.inspect(entity, &mut cfg, ui, self.undo_stack, self.signal, &mut String::new());
                        transform.inspect(entity, &mut cfg, ui, self.undo_stack, self.signal, e.label_mut());

                        if let Some(script) = script {
                            script.inspect(entity, &mut cfg, ui, self.undo_stack, self.signal, e.label_mut());
                        }

                        // todo: convert camera into component
                        let entity_id_copy = *entity;
                        let entity_label = label.clone();
                        let entity_position = transform.position;
                        let camera_manager = &self.camera_manager;
                        let signal = &mut *self.signal;
                        let get_player_camera_target =
                            camera_manager.get_player_camera_target();
                        let get_player_camera_offset =
                            camera_manager.get_player_camera_offset();
                        let get_active_type = camera_manager.get_active_type();
                        let get_active_eye = camera_manager.get_active().unwrap().eye;

                        let followed_entity_label = if let Some(target_entity) =
                            get_player_camera_target
                        {
                            if target_entity != entity_id_copy {
                                if let Ok((followed_entity, _, _)) = self.world
                                    .query_one_mut::<(&AdoptedEntity, &Transform, &ModelProperties)>(target_entity)
                                {
                                    Some(followed_entity.label().to_string())
                                } else {
                                    None
                                }
                            } else {
                                None
                            }
                        } else {
                            None
                        };

                        ui.group(|ui| {
                            CollapsingHeader::new("Camera")
                            .default_open(true)
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    if ui.button("Capture Camera Position relative to Entity").clicked() {
                                        let current_camera_pos = get_active_eye;
                                        let calculated_offset = current_camera_pos - entity_position;
                                        log::debug!("Capturing camera offset: entity at {:?}, camera at {:?}, offset: {:?}",
                                            entity_position, current_camera_pos, calculated_offset);
                                        *signal = Signal::CameraAction(CameraAction::SetPlayerTarget {
                                            entity: entity_id_copy,
                                            offset: calculated_offset,
                                        });
                                        crate::success_without_console!("Camera successfully attached to {}", entity_label);
                                    }
                                });
                                ui.separator();
                                ui.horizontal(|ui| {
                                    ui.label("Status:");
                                    let status_text = match get_active_type {
                                        crate::camera::CameraType::Debug => {
                                            egui::RichText::new("Debug Camera (Free)")
                                                .color(egui::Color32::LIGHT_BLUE)
                                        },
                                        crate::camera::CameraType::Player => {
                                            if let Some(target_entity) = get_player_camera_target {
                                                if target_entity == entity_id_copy {
                                                    egui::RichText::new("Following THIS Entity")
                                                        .color(egui::Color32::LIGHT_GREEN)
                                                        .strong()
                                                } else {
                                                    if let Some(followed_label) = &followed_entity_label {
                                                        egui::RichText::new(format!("Following: {}", followed_label))
                                                            .color(egui::Color32::YELLOW)
                                                    } else {
                                                        egui::RichText::new("Following: Unknown Entity")
                                                            .color(egui::Color32::RED)
                                                    }
                                                }
                                            } else {
                                                egui::RichText::new("Player Camera (Free)")
                                                    .color(egui::Color32::LIGHT_GRAY)
                                            }
                                        }
                                    };
                                    ui.label(status_text);
                                });

                                ui.separator();

                                if let Some(target_entity) = get_player_camera_target {
                                    if target_entity == entity_id_copy {
                                        ui.horizontal(|ui| {
                                            ui.label("Camera Offset:");
                                            if let Some(offset) = get_player_camera_offset {
                                                ui.label(format!("({:.2}, {:.2}, {:.2})", offset.x, offset.y, offset.z));
                                            } else {
                                                ui.label("Unknown");
                                            }
                                        });
                                        ui.horizontal(|ui| {
                                            ui.label("Distance:");
                                            let camera_pos = get_active_eye;
                                            let distance = (camera_pos - entity_position).length();
                                            ui.label(format!("{:.2} units", distance));
                                        });
                                    }
                                }

                                ui.horizontal(|ui| {
                                    if ui.button("Clear Camera Target").clicked() {
                                        *signal = Signal::CameraAction(CameraAction::ClearPlayerTarget);
                                    }
                                });
                            });
                        });

                        if let Some(t) = cfg.label_last_edit {
                            if t.elapsed() >= Duration::from_millis(500) {
                                if let Some(ent) = cfg.old_label_entity.take() {
                                    if let Some(orig) = cfg.label_original.take() {
                                        UndoableAction::push_to_undo(
                                            &mut self.undo_stack,
                                            UndoableAction::Label(ent, orig, EntityType::Entity),
                                        );
                                        log::debug!("Pushed label change to undo stack after 500ms debounce period");
                                    }
                                }
                                cfg.label_last_edit = None;
                            }
                        }
                    } else {
                        log_once::debug_once!("Unable to query entity inside resource inspector");
                    }
                    
                    if let Ok((light, transform, _props)) = self.world.query_one_mut::<(&mut Light, &mut Transform, &mut LightComponent)>(*entity) {
                            light.inspect(entity, &mut cfg, ui, self.undo_stack, self.signal, &mut String::new());
                            transform.inspect(entity, &mut cfg, ui, self.undo_stack, self.signal, &mut light.label);
                            if let Some(t) = cfg.label_last_edit {
                                if t.elapsed() >= Duration::from_millis(500) {
                                    if let Some(ent) = cfg.old_label_entity.take() {
                                        if let Some(orig) = cfg.label_original.take() {
                                            UndoableAction::push_to_undo(
                                                &mut self.undo_stack,
                                                UndoableAction::Label(ent, orig, EntityType::Light),
                                            );
                                            log::debug!("Pushed label change to undo stack after 500ms debounce period");
                                        }
                                    }
                                    cfg.label_last_edit = None;
                                }
                            }
                    }
                } else {
                    ui.label("No entity selected, therefore no info to provide. Go on, what are you waiting for? Click an entity!");
                }
            }
        }

        let mut menu_action: Option<EditorTabMenuAction> = None;
        let area = egui::Area::new("context_menu".into())
            .fixed_pos(cfg.context_menu_pos)
            .order(egui::Order::Foreground);

        if cfg.show_context_menu {
            let menu_tab = cfg
                .context_menu_tab
                .clone()
                .unwrap_or(EditorTab::ModelEntityList);

            let mut popup_rect = None;

            area.show(ui.ctx(), |ui| {
                egui::Frame::popup(ui.style()).show(ui, |ui| {
                    popup_rect.replace(ui.max_rect());

                    match menu_tab {
                        EditorTab::AssetViewer => {
                            ui.set_min_width(150.0);
                            if ui.selectable_label(false, "Import resource").clicked() {
                                menu_action = Some(EditorTabMenuAction::ImportResource);
                            }
                            if ui.selectable_label(false, "Refresh assets").clicked() {
                                menu_action = Some(EditorTabMenuAction::RefreshAssets);
                            }
                        }
                        EditorTab::ModelEntityList => {
                            ui.set_min_width(150.0);
                            if ui.selectable_label(false, "Add Entity").clicked() {
                                menu_action = Some(EditorTabMenuAction::AddEntity);
                            }
                            if ui.selectable_label(false, "Delete Entity").clicked() {
                                menu_action = Some(EditorTabMenuAction::DeleteEntity);
                            }
                        }
                        EditorTab::ResourceInspector => {
                            ui.set_min_width(150.0);
                            if ui.selectable_label(false, "Add Component").clicked() {
                                menu_action = Some(EditorTabMenuAction::AddComponent);
                            }
                        }
                        EditorTab::Viewport => {
                            ui.set_min_width(150.0);
                            if ui.selectable_label(false, "Viewport Option").clicked() {
                                menu_action = Some(EditorTabMenuAction::ViewportOption);
                            }
                        }
                    }
                })
            });

            if let Some(action) = menu_action {
                if Some(tab.clone()) == cfg.context_menu_tab {
                    match action {
                        EditorTabMenuAction::ImportResource => {
                            log::debug!("Import Resource clicked");

                            match import_object() {
                                Ok(_) => {
                                    crate::success!("Resource(s) imported successfully!");
                                }
                                Err(e) => {
                                    crate::warn!("Failed to import resource(s): {e}");
                                }
                            }
                            cfg.show_context_menu = false;
                            cfg.context_menu_tab = None;
                            return;
                        }
                        EditorTabMenuAction::RefreshAssets => {
                            log::debug!("Refresh assets clicked");
                            if let Ok(mut res) = RESOURCES.write() {
                                match res.update_mem() {
                                    Ok(res_cfg) => {
                                        *res = res_cfg;
                                        crate::success!("Assets refreshed successfully!");
                                    }
                                    Err(e) => {
                                        crate::fatal!("Failed to refresh assets: {}", e);
                                    }
                                }
                            }
                            cfg.show_context_menu = false;
                            cfg.context_menu_tab = None;
                            return;
                        }
                        EditorTabMenuAction::AddEntity => {
                            log::debug!("Add Entity clicked");
                            cfg.show_context_menu = false;
                            cfg.context_menu_tab = None;
                            return;
                        }
                        EditorTabMenuAction::DeleteEntity => {
                            log::debug!("Delete Entity clicked");
                            *self.signal = Signal::Delete;
                            cfg.show_context_menu = false;
                            cfg.context_menu_tab = None;
                            return;
                        }
                        EditorTabMenuAction::AddComponent => {
                            log::debug!("Add Component clicked");
                            if let Some(entity) = self.selected_entity {
                                if let Ok(..) = self.world.query_one_mut::<&AdoptedEntity>(*entity) {
                                    log::debug!("Queried selected entity, it is an entity");
                                    *self.signal = Signal::AddComponent(*entity, EntityType::Entity);
                                }

                                if let Ok(..) = self.world.query_one_mut::<&Light>(*entity) {
                                    log::debug!("Queried selected entity, it is a light");
                                    *self.signal = Signal::AddComponent(*entity, EntityType::Entity);
                                }
                            } else {
                                crate::warn!("What are you adding a component to? Theres no entity selected...");
                            }

                            cfg.show_context_menu = false;
                            cfg.context_menu_tab = None;
                            return;
                        }
                        EditorTabMenuAction::ViewportOption => {
                            log::debug!("Viewport Option clicked");
                            cfg.show_context_menu = false;
                            cfg.context_menu_tab = None;
                            return;
                        },
                        EditorTabMenuAction::RemoveComponent => {
                            log::debug!("Remove Component clicked");
                            if let Some(entity) = self.selected_entity {
                                if let Ok(script) = self.world.query_one_mut::<&ScriptComponent>(*entity) {
                                    log::debug!("Queried selected entity, it has a script component");
                                    *self.signal = Signal::RemoveComponent(*entity, ComponentType::Script(script.clone()));
                                } else {
                                    crate::warn!("Selected entity does not have a script component to remove");
                                }
                            } else {
                                panic!("Paradoxical error: Cannot remove a component when its not selected...");
                            }

                            cfg.show_context_menu = false;
                            cfg.context_menu_tab = None;
                            return;
                        }
                    }
                }
            }

            if let Some(rect) = popup_rect {
                if cfg.show_context_menu && Some(tab.clone()) == cfg.context_menu_tab {
                    if ui
                        .ctx()
                        .input(|i| i.pointer.button_clicked(egui::PointerButton::Primary))
                    {
                        if let Some(pos) = ui.ctx().input(|i| i.pointer.interact_pos()) {
                            if !rect.contains(pos) {
                                cfg.show_context_menu = false;
                                cfg.context_menu_tab = None;
                            }
                        }
                    }
                }
            }
        }
    }
}


pub(crate) fn import_object() -> anyhow::Result<()> {
    let model_ext = vec!["glb", "fbx", "obj"];
    let texture_ext = vec!["png"];

    let files = rfd::FileDialog::new()
        .add_filter("All Files", &["*"])
        .add_filter("Model", &model_ext)
        .add_filter("Texture", &texture_ext)
        .pick_files();
    if let Some(files) = files {
        for file in files {
            let ext = file.extension().unwrap().to_str().unwrap();
            let mut copied = false;
            for mde in model_ext.iter() {
                if ext.contains(mde) {
                    // copy over to models folder
                    if let Some(project) = crate::states::PROJECT.read().ok() {
                        let models_dir = PathBuf::from(project.project_path.clone())
                            .join("resources")
                            .join("models");
                        if !models_dir.exists() {
                            std::fs::create_dir_all(&models_dir)?;
                        }
                        let dest = models_dir.join(file.file_name().unwrap());
                        std::fs::copy(&file, &dest)?;
                        log::info!("Copied model file to {:?}", dest);
                        copied = true;
                    }
                }
            }
            for tex in texture_ext.iter() {
                if ext.contains(tex) {
                    // copy over to textures folder
                    if let Some(project) = crate::states::PROJECT.read().ok() {
                        let textures_dir = PathBuf::from(project.project_path.clone())
                            .join("resources")
                            .join("textures");
                        if !textures_dir.exists() {
                            std::fs::create_dir_all(&textures_dir)?;
                        }
                        let dest = textures_dir.join(file.file_name().unwrap());
                        std::fs::copy(&file, &dest)?;
                        log::info!("Copied texture file to {:?}", dest);
                        copied = true;
                    }
                }
            }

            if !copied {
                if let Some(project) = crate::states::PROJECT.read().ok() {
                    // everything else copies over to resources root dir
                    let resources_dir =
                        PathBuf::from(project.project_path.clone()).join("resources");
                    if !resources_dir.exists() {
                        std::fs::create_dir_all(&resources_dir)?;
                    }
                    let dest = resources_dir.join(file.file_name().unwrap());
                    std::fs::copy(&file, &dest)?;
                    log::info!("Copied other resource file to {:?}", dest);
                }
            }
        }
        // save it all to ensure the eucc recognises it
        let mut proj = PROJECT.write().unwrap();
        proj.write_to_all()?;
        Ok(())
    } else {
        return Err(anyhow::anyhow!("File dialogue returned None"));
    }
}

#[derive(Debug, Clone, Copy)]
pub enum EditorTabMenuAction {
    ImportResource,
    RefreshAssets,
    AddEntity,
    DeleteEntity,
    AddComponent,
    RemoveComponent,
    ViewportOption,
}
