use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use dropbear_engine::entity::{EntityTransform, Transform};
use crate::states::Label;

/// A component that is added to all entities to show all child entities
#[derive(Default, Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Parent(Vec<hecs::Entity>);

impl Parent {
    /// Creates a new parent component with the provided child entities.
    pub fn new(children: Vec<hecs::Entity>) -> Self {
        Self(children)
    }

    /// Returns an immutable view into the stored child entities.
    pub fn children(&self) -> &[hecs::Entity] {
        &self.0
    }

    /// Returns a mutable view into the stored child entities.
    pub fn children_mut(&mut self) -> &mut Vec<hecs::Entity> {
        &mut self.0
    }

    /// Adds a new child entity to this parent component.
    pub fn push(&mut self, child: hecs::Entity) {
        self.0.push(child);
    }

    /// Removes all children from this parent component.
    pub fn clear(&mut self) {
        self.0.clear();
    }

    /// Returns whether this parent does not track any child entities.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

/// A component that points to the parent entity of an entity.
#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct Child(hecs::Entity);

impl Child {
    /// Creates a new child component with the provided parent entity.
    pub fn new(parent: hecs::Entity) -> Self {
        Self(parent)
    }

    /// Returns the parent entity of this child component.
    pub fn parent(&self) -> hecs::Entity {
        self.0
    }
}

/// An extension trait for [EntityTransform] that allows for propagation of entities into a target transform.
pub trait EntityTransformExt {
    /// Walks up the [`hecs::World`] and calculates the final [Transform] for the entity based off its parents.
    fn propagate(&self, world: &hecs::World, target_entity: hecs::Entity) -> Transform;
}

impl EntityTransformExt for EntityTransform {
    fn propagate(&self, world: &hecs::World, target_entity: hecs::Entity) -> Transform {
        let mut result = self.local().clone();

        let mut current = target_entity;
        while let Ok(child) = world.get::<&Child>(current) {
            let parent_entity = child.parent();

            if let Ok(parent_transform) = world.get::<&EntityTransform>(parent_entity) {
                let parent_world = parent_transform.world();

                result = Transform {
                    position: parent_world.position + parent_world.rotation * (result.position * parent_world.scale),
                    rotation: parent_world.rotation * result.rotation,
                    scale: parent_world.scale * result.scale,
                };
            }

            current = parent_entity;
        }

        result
    }
}

#[derive(Default, Serialize, Deserialize, Clone, Debug)]
pub struct SceneHierarchy {
    /// Maps entity labels to their parent label
    parent_map: HashMap<Label, Label>,
    /// Maps entity labels to their children labels
    children_map: HashMap<Label, Vec<Label>>,
}

impl SceneHierarchy {
    pub fn new() -> Self {
        Self {
            parent_map: HashMap::new(),
            children_map: HashMap::new(),
        }
    }

    /// Set the parent of an entity
    pub fn set_parent(&mut self, child: Label, parent: Label) {
        if let Some(old_parent) = self.parent_map.get(&child) {
            if let Some(children) = self.children_map.get_mut(old_parent) {
                children.retain(|c| c != &child);
            }
        }

        self.parent_map.insert(child.clone(), parent.clone());

        self.children_map
            .entry(parent)
            .or_insert_with(Vec::new)
            .push(child);
    }

    /// Remove parent relationship
    pub fn remove_parent(&mut self, child: &Label) {
        if let Some(parent) = self.parent_map.remove(child) {
            if let Some(children) = self.children_map.get_mut(&parent) {
                children.retain(|c| c != child);
            }
        }
    }

    /// Get the parent of an entity
    pub fn get_parent(&self, child: &Label) -> Option<&Label> {
        self.parent_map.get(child)
    }

    /// Get the children of an entity
    pub fn get_children(&self, parent: &Label) -> &[Label] {
        self.children_map
            .get(parent)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    /// Get all ancestors of an entity (parent, grandparent, etc.)
    pub fn get_ancestors(&self, entity: &Label) -> Vec<Label> {
        let mut ancestors = Vec::new();
        let mut current = entity.clone();

        while let Some(parent) = self.parent_map.get(&current) {
            ancestors.push(parent.clone());
            current = parent.clone();
        }

        ancestors
    }

    /// Check if an entity is a descendant of another
    pub fn is_descendant_of(&self, entity: &Label, potential_ancestor: &Label) -> bool {
        let mut current = entity.clone();

        while let Some(parent) = self.parent_map.get(&current) {
            if parent == potential_ancestor {
                return true;
            }
            current = parent.clone();
        }

        false
    }
}