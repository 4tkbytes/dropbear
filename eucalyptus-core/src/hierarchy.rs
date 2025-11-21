use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use dropbear_engine::entity::{EntityTransform, Transform};
use crate::states::Label;

/// A component that tracks all child entities of a parent entity
#[derive(Default, Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Children(Vec<hecs::Entity>);

impl Children {
    /// Creates a new children component with the provided child entities.
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

    /// Adds a new child entity to this component.
    pub fn push(&mut self, child: hecs::Entity) {
        if !self.0.contains(&child) {
            self.0.push(child);
        }
    }

    /// Removes a specific child entity.
    pub fn remove(&mut self, child: hecs::Entity) {
        self.0.retain(|&e| e != child);
    }

    /// Removes all children from this component.
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
pub struct Parent(hecs::Entity);

impl Parent {
    /// Creates a new parent component with the provided parent entity.
    pub fn new(parent: hecs::Entity) -> Self {
        Self(parent)
    }

    /// Returns the parent entity of this component.
    pub fn parent(&self) -> hecs::Entity {
        self.0
    }
}

/// Helper functions for managing entity hierarchies
pub struct Hierarchy;

impl Hierarchy {
    /// Set the parent of a child entity, updating both Parent and Children components
    pub fn set_parent(world: &mut hecs::World, child: hecs::Entity, parent: hecs::Entity) {
        // Remove old parent relationship if it exists
        if let Ok(old_parent) = world.get::<&Parent>(child) {
            let old_parent_entity = old_parent.parent();
            if let Ok(mut children) = world.get::<&mut Children>(old_parent_entity) {
                children.remove(child);
            }
        }

        // Add Parent component to child
        let _ = world.insert_one(child, Parent::new(parent));

        // Add child to parent's Children component
        if let Ok(mut children) = world.get::<&mut Children>(parent) {
            children.push(child);
        } else {
            let _ = world.insert_one(parent, Children::new(vec![child]));
        }
    }

    /// Remove parent relationship from a child entity
    pub fn remove_parent(world: &mut hecs::World, child: hecs::Entity) {
        let mut local_remove_signal = false;
        if let Ok(parent) = world.get::<&Parent>(child) {
            let parent_entity = parent.parent();

            local_remove_signal = true;

            if let Ok(mut children) = world.get::<&mut Children>(parent_entity) {
                children.remove(child);
            }
        }

        if local_remove_signal {
            let _ = world.remove_one::<Parent>(child);
        }
    }

    /// Get all children of an entity
    pub fn get_children(world: &hecs::World, entity: hecs::Entity) -> Vec<hecs::Entity> {
        world.get::<&Children>(entity)
            .map(|c| c.children().to_vec())
            .unwrap_or_default()
    }

    /// Get the parent of an entity
    pub fn get_parent(world: &hecs::World, entity: hecs::Entity) -> Option<hecs::Entity> {
        world.get::<&Parent>(entity)
            .ok()
            .map(|p| p.parent())
    }

    /// Get all ancestors of an entity (parent, grandparent, etc.)
    pub fn get_ancestors(world: &hecs::World, entity: hecs::Entity) -> Vec<hecs::Entity> {
        let mut ancestors = Vec::new();
        let mut current = entity;

        while let Some(parent) = Self::get_parent(world, current) {
            ancestors.push(parent);
            current = parent;
        }

        ancestors
    }

    /// Check if an entity is a descendant of another
    pub fn is_descendant_of(
        world: &hecs::World,
        entity: hecs::Entity,
        potential_ancestor: hecs::Entity,
    ) -> bool {
        let mut current = entity;

        while let Some(parent) = Self::get_parent(world, current) {
            if parent == potential_ancestor {
                return true;
            }
            current = parent;
        }

        false
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
        while let Ok(parent_comp) = world.get::<&Parent>(current) {
            let parent_entity = parent_comp.parent();

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

/// A serializable scene hierarchy based on entity labels
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

    /// Build hierarchy from world entities
    pub fn from_world(world: &hecs::World) -> Self {
        let mut hierarchy = Self::new();

        for (_entity, (label, parent)) in world.query::<(&Label, &Parent)>().iter() {
            if let Ok(parent_label) = world.get::<&Label>(parent.parent()) {
                hierarchy.set_parent(label.clone(), Label::new(parent_label.as_str()));
            }
        }

        hierarchy
    }

    /// Apply this hierarchy to a world
    pub fn apply_to_world(&self, world: &mut hecs::World, label_to_entity: &HashMap<Label, hecs::Entity>) {
        for (child_label, parent_label) in &self.parent_map {
            if let (Some(&child_entity), Some(&parent_entity)) = (
                label_to_entity.get(child_label),
                label_to_entity.get(parent_label),
            ) {
                Hierarchy::set_parent(world, child_entity, parent_entity);
            }
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