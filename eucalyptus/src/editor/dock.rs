use super::*;
use std::{
    collections::HashSet,
    sync::{LazyLock, Mutex},
};

use dropbear_engine::entity::{Transform};
use egui::{self};
use egui_dock_fork::TabViewer;
use egui_extras;
use egui_toast_fork::{Toast, ToastKind};
use log;
use serde::{Deserialize, Serialize};
use transform_gizmo_egui::{
    Gizmo, GizmoConfig, GizmoExt, GizmoMode,
    math::{DMat4, DVec3},
};

use crate::{
    APP_INFO,
    editor::scene::PENDING_SPAWNS,
    states::{EntityNode, Node, RESOURCES, ResourceType},
    utils::PendingSpawn,
};

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
    pub camera: &'a mut Camera,
    pub resize_signal: &'a mut (bool, u32, u32),
    pub world: &'a mut hecs::World,
    pub selected_entity: &'a mut Option<hecs::Entity>,
    pub viewport_mode: &'a mut ViewportMode,
}

impl<'a> EditorTabViewer<'a> {
    fn spawn_entity_at_pos(
        &mut self,
        asset: &DraggedAsset,
        position: glam::DVec3,
    ) -> anyhow::Result<()> {
        let mut transform = Transform::default();
        transform.position = position;

        if let Ok(mut pending_spawns) = PENDING_SPAWNS.lock() {
            pending_spawns.push(PendingSpawn {
                asset_path: asset.path.clone(),
                asset_name: asset.name.clone(),
                transform,
            });
            Ok(())
        } else {
            return Err(anyhow::anyhow!(
                "Failed to retrieve the lock from the PENDING_SPAWNS const"
            ));
        }
    }

    #[allow(dead_code)]
    // purely for debug, nothing else...
    fn debug_camera_state(&self) {
        log::debug!("Camera state:");
        log::debug!("  Eye: {:?}", self.camera.eye);
        log::debug!("  Target: {:?}", self.camera.target);
        log::debug!("  Up: {:?}", self.camera.up);
        log::debug!("  FOV Y: {}", self.camera.fov_y);
        log::debug!("  Aspect: {}", self.camera.aspect);
        log::debug!("  Z Near: {}", self.camera.znear);
        log::debug!("  Proj Mat finite: {}", self.camera.proj_mat.is_finite());
        log::debug!("  View Mat finite: {}", self.camera.view_mat.is_finite());
    }
}

#[derive(Clone, Debug)]
pub struct DraggedAsset {
    pub name: String,
    pub path: PathBuf,
    pub asset_type: ResourceType,
}

pub static TABS_GLOBAL: LazyLock<Mutex<INeedABetterNameForThisStruct>> =
    LazyLock::new(|| Mutex::new(INeedABetterNameForThisStruct::default()));

pub static DRAGGED_ASSET: LazyLock<Mutex<Option<DraggedAsset>>> =
    LazyLock::new(|| Mutex::new(None));

pub(crate) struct INeedABetterNameForThisStruct {
    show_context_menu: bool,
    context_menu_pos: egui::Pos2,
    context_menu_tab: Option<EditorTab>,
}

impl Default for INeedABetterNameForThisStruct {
    fn default() -> Self {
        Self {
            show_context_menu: Default::default(),
            context_menu_pos: Default::default(),
            context_menu_tab: Default::default(),
        }
    }
}

impl INeedABetterNameForThisStruct {}

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
        let mut cfg = TABS_GLOBAL.lock().unwrap();

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
                let size = ui.available_size();
                let new_tex_width = size.x.max(1.0) as u32;
                let new_tex_height = size.y.max(1.0) as u32;

                if self.tex_size.width != new_tex_width || self.tex_size.height != new_tex_height {
                    *self.resize_signal = (true, new_tex_width, new_tex_height);

                    self.tex_size.width = new_tex_width;
                    self.tex_size.height = new_tex_height;

                    let new_aspect = new_tex_width as f64 / new_tex_height as f64;
                    self.camera.aspect = new_aspect;
                }

                let image_response = ui.add(
                    egui::Image::new((
                        self.view,
                        [self.tex_size.width as f32, self.tex_size.height as f32].into(),
                    ))
                    .sense(egui::Sense::click_and_drag()),
                );

                if image_response.hovered() {
                    if let Ok(mut dragged_asset) = DRAGGED_ASSET.lock() {
                        if let Some(asset) = dragged_asset.take() {
                            if matches!(asset.asset_type, ResourceType::Model) {
                                if let Some(drop_pos) = ui.ctx().input(|i| i.pointer.interact_pos())
                                {
                                    let (ray_og, ray_dir) = crate::utils::screen_to_world_coords(
                                        self.camera,
                                        drop_pos,
                                        image_response.rect,
                                    );

                                    let spawn_distance = 5.0;
                                    let spawn_position = ray_og + ray_dir * spawn_distance;

                                    match self.spawn_entity_at_pos(&asset, spawn_position) {
                                        Ok(()) => {
                                            log::info!(
                                                "Queued spawn for {} at position {:?}",
                                                asset.name,
                                                spawn_position
                                            );

                                            if let Ok(mut toasts) = GLOBAL_TOASTS.lock() {
                                                toasts.add(Toast {
                                                    kind: ToastKind::Success,
                                                    text: format!("Spawning {}", asset.name).into(), // Changed text to indicate queuing
                                                    options: ToastOptions::default()
                                                        .duration_in_seconds(2.0),
                                                    ..Default::default()
                                                });
                                            }
                                        }
                                        Err(e) => {
                                            log::error!(
                                                "Failed to queue spawn for {}: {}",
                                                asset.name,
                                                e
                                            );

                                            if let Ok(mut toasts) = GLOBAL_TOASTS.lock() {
                                                toasts.add(Toast {
                                                    kind: ToastKind::Error,
                                                    text: format!(
                                                        "Failed to queue spawn for {}: {}",
                                                        asset.name, e
                                                    )
                                                    .into(),
                                                    options: ToastOptions::default()
                                                        .duration_in_seconds(3.0),
                                                    ..Default::default()
                                                });
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                if image_response.clicked() {
                    if let Some(click_pos) = ui.ctx().input(|i| i.pointer.interact_pos()) {
                        // self.debug_camera_state();

                        let (ray_origin, ray_dir) = crate::utils::screen_to_world_coords(
                            self.camera,
                            click_pos,
                            image_response.rect,
                        );
                        log::debug!(
                            "Click pos: {:?}, viewport rect: {:?}",
                            click_pos,
                            image_response.rect
                        );
                        log::debug!("Ray origin: {:?}, direction: {:?}", ray_origin, ray_dir);
                    }
                }

                let snapping = ui.input(|input| input.modifiers.shift);

                // Note to self: fuck you >:(
                // Note to self: ok wow thats pretty rude im trying my best ＞﹏＜
                // Note to self: finally holy shit i got it working
                self.gizmo.update_config(GizmoConfig {
                    view_matrix: DMat4::look_at_lh(
                        DVec3::new(
                            self.camera.eye.x as f64,
                            self.camera.eye.y as f64,
                            self.camera.eye.z as f64,
                        ),
                        DVec3::new(
                            self.camera.target.x as f64,
                            self.camera.target.y as f64,
                            self.camera.target.z as f64,
                        ),
                        DVec3::new(
                            self.camera.up.x as f64,
                            self.camera.up.y as f64,
                            self.camera.up.z as f64,
                        ),
                    )
                    .into(),
                    projection_matrix: DMat4::perspective_infinite_reverse_lh(
                        self.camera.fov_y as f64,
                        self.camera.aspect as f64,
                        self.camera.znear as f64,
                    )
                    .into(),
                    viewport: image_response.rect,
                    modes: GizmoMode::all(),
                    orientation: transform_gizmo_egui::GizmoOrientation::Global,
                    snapping,
                    ..Default::default()
                });

                if matches!(self.viewport_mode, crate::utils::ViewportMode::Gizmo) {
                    if let Some(entity_id) = self.selected_entity {
                        if let Ok(transform) =
                            self.world.query_one_mut::<&mut Transform>(*entity_id)
                        {
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
                        }
                    }
                }
            }
            EditorTab::ModelEntityList => {
                ui.label("Model/Entity List");
                // TODO: deal with show_entity_tree and figure out how to convert hecs::World
                // to EntityNodes and to write it to file

                // Note: Technically i have already done that, but further testing required. 
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
                                                // let image = egui::Image::from_uri(format!(
                                                //     "file://{}",
                                                //     image_uri
                                                // ));

                                                assets.push((
                                                    format!("file://{}", image_uri),
                                                    file.name.clone(),
                                                    file.path.clone(),
                                                    res_type.clone(),
                                                ))
                                            }
                                            ResourceType::Texture => {
                                                // let image = egui::Image::from_bytes(
                                                //     file.name.clone(),
                                                //     std::fs::read(&file.path)
                                                //         .unwrap_or(NO_TEXTURE.to_vec()),
                                                // );
                                                assets.push((
                                                    file.path.to_string_lossy().to_string(),
                                                    file.name.clone(),
                                                    file.path.clone(),
                                                    res_type.clone(),
                                                ))
                                            }
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
                                                // let image = egui::Image::from_bytes(
                                                //     file.name.clone(),
                                                //     NO_TEXTURE,
                                                // );
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

                    let mut logged = LOGGED.lock().unwrap();
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

                                            let is_hovered = card_response.hovered()
                                                || image_response.hovered()
                                                || state.dragged;
                                            
                                            let is_d_clicked = card_response.double_clicked() || image_response.double_clicked();

                                            if is_d_clicked {
                                                if matches!(asset_type, ResourceType::Model) {
                                                    let spawn_position = self.camera.eye;
                                                    let asset = DraggedAsset {
                                                        name: asset_name.clone(),
                                                        path: asset_path.clone(),
                                                        asset_type: asset_type.clone(),
                                                    };

                                                    match self.spawn_entity_at_pos(&asset, spawn_position) {
                                                        Ok(()) => {
                                                            log::debug!("db click spawned {} at camera pos {:?}",
                                                                asset.name, spawn_position
                                                            );

                                                            if let Ok(mut toasts) = GLOBAL_TOASTS.lock() {
                                                                toasts.add(Toast {
                                                                    kind: ToastKind::Success,
                                                                    text: format!("Spawned {} at camera", asset.name).into(),
                                                                    options: ToastOptions::default()
                                                                        .duration_in_seconds(2.0),
                                                                    ..Default::default()
                                                                });
                                                            }
                                                        }
                                                        Err(e) => {
                                                            log::error!(
                                                            "Failed to spawn {} at camera: {}",
                                                            asset.name,
                                                            e);

                                                            if let Ok(mut toasts) = GLOBAL_TOASTS.lock() {
                                                                toasts.add(Toast {
                                                                    kind: ToastKind::Error,
                                                                    text: format!(
                                                                        "Failed to spawn {}: {}",
                                                                        asset.name, e
                                                                    ).into(),
                                                                    options: ToastOptions::default()
                                                                        .duration_in_seconds(3.0),
                                                                    ..Default::default()
                                                                });
                                                            }
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
                ui.label("Resource Inspector");
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

            // We'll store the popup rect here
            let mut popup_rect = None;

            area.show(ui.ctx(), |ui| {
                egui::Frame::popup(ui.style()).show(ui, |ui| {
                    // Save the rect of the popup for later hit-testing
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
                            if ui.selectable_label(false, "Inspect Resource").clicked() {
                                menu_action = Some(EditorTabMenuAction::InspectResource);
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
                                    if let Ok(mut toasts) = GLOBAL_TOASTS.lock() {
                                        toasts.add(Toast {
                                            kind: ToastKind::Success,
                                            text: "Resource(s) imported successfully!".into(),
                                            options: ToastOptions::default()
                                                .duration_in_seconds(3.0),
                                            ..Default::default()
                                        });
                                    }
                                }
                                Err(e) => {
                                    if let Ok(mut toasts) = GLOBAL_TOASTS.lock() {
                                        toasts.add(Toast {
                                            kind: ToastKind::Error,
                                            text: format!("Failed to import resource(s): {e}")
                                                .into(),
                                            options: ToastOptions::default()
                                                .duration_in_seconds(5.0),
                                            ..Default::default()
                                        });
                                    }
                                }
                            }
                            cfg.show_context_menu = false;
                            cfg.context_menu_tab = None;
                            return;
                        }
                        EditorTabMenuAction::RefreshAssets => {
                            log::debug!("Refresh assets clicked");
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
                            cfg.show_context_menu = false;
                            cfg.context_menu_tab = None;
                            return;
                        }
                        EditorTabMenuAction::InspectResource => {
                            log::debug!("Inspect Resource clicked");
                            cfg.show_context_menu = false;
                            cfg.context_menu_tab = None;
                            return;
                        }
                        EditorTabMenuAction::ViewportOption => {
                            log::debug!("Viewport Option clicked");
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
    InspectResource,
    ViewportOption,
}
