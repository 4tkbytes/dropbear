use crate::{
    ComponentConverter, ComponentDeserializer, CustomConverter, CustomDeserializer,
    DirectConverter, DirectDeserializer, SerializableComponent,
};
use anyhow::Result;
use hecs::{Entity, EntityBuilder, World};
use std::any::TypeId;
use std::collections::HashMap;

pub struct ComponentRegistry {
    converters: HashMap<TypeId, Box<dyn ComponentConverter>>,
    deserializers: HashMap<TypeId, Box<dyn ComponentDeserializer>>,
    serializable_ids: HashMap<TypeId, u64>,
    id_to_serializable: HashMap<u64, TypeId>,
    next_component_id: u64,
}

impl ComponentRegistry {
    pub fn new() -> Self {
        Self {
            converters: HashMap::new(),
            deserializers: HashMap::new(),
            serializable_ids: HashMap::new(),
            id_to_serializable: HashMap::new(),
            next_component_id: 1,
        }
    }

    // Register a component that's already SerializableComponent
    pub fn register<T>(&mut self)
    where
        T: SerializableComponent + hecs::Component + Clone + 'static,
    {
        let type_id = TypeId::of::<T>();
        self.ensure_serializable_id(type_id);
        self.converters
            .insert(type_id, Box::new(DirectConverter::<T>::new()));
        self.deserializers
            .insert(type_id, Box::new(DirectDeserializer::<T>::new()));
    }

    // Register a custom converter for special cases
    pub fn register_converter<From, To, F>(&mut self, converter_fn: F)
    where
        From: hecs::Component + 'static,
        To: SerializableComponent + 'static,
        F: Fn(&World, Entity, &From) -> Option<To> + Send + Sync + 'static,
    {
        let type_id = TypeId::of::<From>();
        // converter output is To, so track its serializable id
        self.ensure_serializable_id(TypeId::of::<To>());
        self.converters
            .insert(type_id, Box::new(CustomConverter::new(converter_fn)));
    }

    pub fn register_deserializer<From, To, F>(&mut self, converter_fn: F)
    where
        From: SerializableComponent + 'static,
        To: hecs::Component + Clone + 'static,
        F: Fn(&From) -> To + Send + Sync + 'static,
    {
        let type_id = TypeId::of::<From>();
        self.ensure_serializable_id(type_id);
        self.deserializers
            .insert(type_id, Box::new(CustomDeserializer::new(converter_fn)));
    }

    // Extract all serializable components from an entity
    pub fn extract_all_components(
        &self,
        world: &World,
        entity: Entity,
    ) -> Vec<Box<dyn SerializableComponent>> {
        let mut vec = vec![];
        for converter in self.converters.values() {
            if let Some(component) = converter.extract_serializable(world, entity) {
                vec.push(component);
            }
        }
        return vec;
    }

    fn ensure_serializable_id(&mut self, type_id: TypeId) -> u64 {
        if let Some(id) = self.serializable_ids.get(&type_id) {
            *id
        } else {
            let id = self.next_component_id;
            self.next_component_id = self.next_component_id.wrapping_add(1).max(1);
            self.serializable_ids.insert(type_id, id);
            self.id_to_serializable.insert(id, type_id);
            id
        }
    }

    /// Returns the numeric identifier that was assigned to the provided
    /// [`SerializableComponent`] type when it was registered.
    pub fn id_for_component(&self, component: &dyn SerializableComponent) -> Option<u64> {
        let type_id = component.as_any().type_id();
        self.serializable_ids.get(&type_id).copied()
    }

    /// Returns the numeric identifier for `T` if it has been registered.
    pub fn id_for_type<T>(&self) -> Option<u64>
    where
        T: SerializableComponent + 'static,
    {
        self.serializable_ids.get(&TypeId::of::<T>()).copied()
    }

    fn serializable_type_from_numeric(&self, component_id: u64) -> Option<TypeId> {
        self.id_to_serializable.get(&component_id).copied()
    }

    /// Attempts to extract a specific component instance from an entity using
    /// its registry-assigned numeric identifier.
    pub fn extract_component_by_numeric_id(
        &self,
        world: &World,
        entity: Entity,
        component_id: u64,
    ) -> Option<Box<dyn SerializableComponent>> {
        let expected_type = self.serializable_type_from_numeric(component_id)?;

        for converter in self.converters.values() {
            if let Some(component) = converter.extract_serializable(world, entity) {
                if component.as_any().type_id() == expected_type {
                    return Some(component);
                }
            }
        }

        None
    }

    /// Iterates every entity in the world and clones any components whose
    /// numeric identifier matches `component_id`.
    pub fn find_components_by_numeric_id(
        &self,
        world: &World,
        component_id: u64,
    ) -> Vec<(Entity, Box<dyn SerializableComponent>)> {
        let mut matches = Vec::new();
        for (entity, ()) in world.query::<()>().iter() {
            if let Some(component) =
                self.extract_component_by_numeric_id(world, entity, component_id)
            {
                matches.push((entity, component));
            }
        }
        matches
    }

    /// Attempts to deserialize a [`SerializableComponent`] back into an
    /// ECS component and insert it into the provided [`EntityBuilder`].
    /// Returns `Ok(true)` if the component was handled, `Ok(false)` if no
    /// deserializer was registered, and `Err` if deserialization failed.
    pub fn deserialize_into_builder(
        &self,
        component: &dyn SerializableComponent,
        builder: &mut EntityBuilder,
    ) -> Result<bool> {
        let type_id = component.as_any().type_id();
        if let Some(deserializer) = self.deserializers.get(&type_id) {
            deserializer.insert_into_builder(component, builder)?;
            Ok(true)
        } else {
            Ok(false)
        }
    }
}

impl Default for ComponentRegistry {
    fn default() -> Self {
        Self::new()
    }
}
