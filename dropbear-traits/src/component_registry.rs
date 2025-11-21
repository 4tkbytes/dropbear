use std::any::TypeId;
use std::collections::HashMap;
use anyhow::Result;
use hecs::{Entity, EntityBuilder, World};
use crate::{
    ComponentConverter,
    ComponentDeserializer,
    CustomConverter,
    CustomDeserializer,
    DirectConverter,
    DirectDeserializer,
    SerializableComponent,
};

#[derive(Default)]
pub struct ComponentRegistry {
    converters: HashMap<TypeId, Box<dyn ComponentConverter>>,
    deserializers: HashMap<TypeId, Box<dyn ComponentDeserializer>>,
}

impl ComponentRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    // Register a component that's already SerializableComponent
    pub fn register<T>(&mut self)
    where
        T: SerializableComponent + hecs::Component + Clone + 'static,
    {
        let type_id = TypeId::of::<T>();
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
        F: Fn(&From) -> To + Send + Sync + 'static,
    {
        let type_id = TypeId::of::<From>();
        self.converters.insert(
            type_id,
            Box::new(CustomConverter::new(converter_fn))
        );
    }

    pub fn register_deserializer<From, To, F>(&mut self, converter_fn: F)
    where
        From: SerializableComponent + 'static,
        To: hecs::Component + Clone + 'static,
        F: Fn(&From) -> To + Send + Sync + 'static,
    {
        let type_id = TypeId::of::<From>();
        self.deserializers.insert(
            type_id,
            Box::new(CustomDeserializer::new(converter_fn)),
        );
    }

    // Extract all serializable components from an entity
    pub fn extract_all_components(
        &self,
        world: &World,
        entity: Entity,
    ) -> Vec<Box<dyn SerializableComponent>> {
        let mut vec = vec![];
        for (k, v) in &self.converters
        {
            match v.extract_serializable(world, entity) {
                Some(v) => vec.push(v),
                None => {log::error!("Unable to extract the serializable value for entity [typeid: {:?}]", k); continue;}
            }
        }
        return vec;
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