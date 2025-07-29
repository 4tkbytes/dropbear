use super::*;
use std::{
    collections::HashSet,
    fs,
    sync::{LazyLock, Mutex},
};

use dropbear_engine::{
    egui, egui_extras,
    graphics::NO_TEXTURE,
    hecs::{self},
    log,
};
use egui_dock_fork::TabViewer;
use serde::{Deserialize, Serialize};

use crate::{
    APP_INFO,
    states::{EntityNode, Node, RESOURCES, ResourceType},
};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum EditorTab {
    AssetViewer,       // bottom side,
    ResourceInspector, // left side,
    ModelEntityList,   // right side,
    Viewport,          // middle,
}

pub struct EditorTabViewer {
    pub view: egui::TextureId,
    pub nodes: Vec<EntityNode>,
}

pub const SELECTED: LazyLock<Mutex<Option<hecs::Entity>>> = LazyLock::new(|| Mutex::new(None));

static TABS_GLOBAL: LazyLock<Mutex<INeedABetterNameForThisStruct>> =
    LazyLock::new(|| Mutex::new(INeedABetterNameForThisStruct::default()));

#[derive(Debug, Default)]
pub(crate) struct INeedABetterNameForThisStruct {
    show_context_menu: bool,
    context_menu_pos: egui::Pos2,
    context_menu_tab: Option<EditorTab>,
}

impl INeedABetterNameForThisStruct {}

impl TabViewer for EditorTabViewer {
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
                        cfg.context_menu_tab = Some(tab.clone()); // <-- Store the tab
                    }
                }
            }
        });

        match tab {
            EditorTab::Viewport => {
                let size = ui.available_size();
                ui.image((self.view, size));
            }
            EditorTab::ModelEntityList => {
                ui.label("Model/Entity List");
                // TODO: deal with show_entity_tree and figure out how to convert hecs::World
                // to EntityNodes and to write it to file
                {
                    show_entity_tree(
                        ui,
                        &mut self.nodes,
                        &mut SELECTED.lock().unwrap(),
                        "Model Entity Asset List",
                    );
                }
            }
            EditorTab::AssetViewer => {
                egui_extras::install_image_loaders(ui.ctx());

                let mut assets: Vec<(egui::Image, String)> = Vec::new();
                {
                    let res = RESOURCES.read().unwrap();

                    fn recursive_search_nodes_and_attach_thumbnail(
                        res: &Vec<Node>,
                        assets: &mut Vec<(egui::Image, String)>,
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
                                                    ad_dir.join("cube_thumbnail.png");
                                                let file_name_osstr =
                                                    model_thumbnail.file_name().unwrap();
                                                let file_name =
                                                    file_name_osstr.to_str().unwrap().to_string();
                                                let image = egui::Image::from_bytes(
                                                    file_name.clone(),
                                                    fs::read(&model_thumbnail).unwrap(),
                                                );
                                                assets.push((image, file.name.clone()))
                                            }
                                            ResourceType::Texture => {
                                                let image = egui::Image::from_bytes(
                                                    file.name.clone(),
                                                    std::fs::read(&file.path)
                                                        .unwrap_or(NO_TEXTURE.to_vec()),
                                                );
                                                assets.push((image, file.name.clone()))
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
                                                let image = egui::Image::from_bytes(
                                                    file.name.clone(),
                                                    NO_TEXTURE,
                                                );
                                                assets.push((image, file.name.clone()))
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
                    let columns = 6;
                    let available_width = ui.available_width();
                    let min_spacing = 8.0;
                    let max_spacing = 30.0;
                    let label_height = 20.0;
                    let padding = 8.0; // check it out

                    let card_width = ((available_width - max_spacing * (columns as f32 - 1.0))
                        / columns as f32)
                        .max(32.0);
                    let image_size = card_width - label_height;
                    let spacing = ((available_width - columns as f32 * card_width)
                        / (columns as f32 - 1.0))
                        .clamp(min_spacing, max_spacing);
                    let card_height = image_size + label_height + padding;

                    egui::Grid::new("asset_grid")
                        .num_columns(columns)
                        .min_col_width(card_width)
                        .max_col_width(card_width)
                        .spacing([spacing, spacing])
                        .show(ui, |ui| {
                            for (i, (image, asset_name)) in assets.iter().enumerate() {
                                let card_size = egui::vec2(card_width, card_height);
                                let (rect, card_response) =
                                    ui.allocate_exact_size(card_size, egui::Sense::click());

                                let mut card_ui = ui.new_child(
                                    egui::UiBuilder::new()
                                        .max_rect(rect)
                                        .layout(egui::Layout::top_down(egui::Align::Center)),
                                );

                                let image_response = card_ui.add(
                                    egui::ImageButton::new(
                                        image.clone().max_size([image_size, image_size].into()),
                                    )
                                    .frame(false),
                                );

                                let is_hovered =
                                    card_response.hovered() || image_response.hovered();

                                if is_hovered {
                                    ui.painter().rect_filled(
                                        rect,
                                        6.0, // corner radius
                                        egui::Color32::from_rgb(60, 60, 80),
                                    );
                                }

                                card_ui.vertical_centered(|ui| {
                                    ui.label(
                                        egui::RichText::new(asset_name)
                                            .strong()
                                            .color(egui::Color32::WHITE),
                                    );
                                });

                                if (i + 1) % columns == 0 {
                                    ui.end_row();
                                }
                            }
                        });
                });
            }
            EditorTab::ResourceInspector => {
                ui.label("Resource Inspector");
            }
        }
    }
}
