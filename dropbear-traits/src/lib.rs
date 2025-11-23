pub mod registry;

use anyhow::{Result, anyhow};
use hecs::{Entity, EntityBuilder, World};
use std::any::{Any, TypeId};
use std::fmt::Debug;

/// A type of component that gets serialized and deserialized into a scene config file.
#[typetag::serde(tag = "type")]
pub trait SerializableComponent: Send + Sync + Debug {
    /// Converts a [SerializableComponent] to an [Any] type.
    fn as_any(&self) -> &dyn Any;
    /// Converts a [SerializableComponent] to a mutable [Any] type
    fn as_any_mut(&mut self) -> &mut dyn Any;
    /// Fetches the type name of that component
    fn type_name(&self) -> &'static str;
    /// Allows you to clone the dynamic object.
    fn clone_boxed(&self) -> Box<dyn SerializableComponent>;

    /// Returns the display name of the component.
    fn display_name(&self) -> String {
        let type_name = self.type_name();
        type_name.split("::").last().unwrap_or(type_name).to_string()
    }
}

impl Clone for Box<dyn SerializableComponent> {
    fn clone(&self) -> Self {
        self.clone_boxed()
    }

    fn clone_from(&mut self, source: &Self) {
        *self = source.clone_boxed();
    }
}

pub trait ComponentConverter: Send + Sync {
    fn type_id(&self) -> TypeId;
    fn type_name(&self) -> &'static str;
    fn serializable_type_id(&self) -> TypeId;

    fn extract_serializable(
        &self,
        world: &World,
        entity: Entity,
    ) -> Option<Box<dyn SerializableComponent>>;

    fn remove_component(&self, world: &mut World, entity: Entity);
}

pub trait ComponentDeserializer: Send + Sync {
    fn serializable_type_id(&self) -> TypeId;
    fn serializable_type_name(&self) -> &'static str;

    fn insert_into_builder(
        &self,
        component: &dyn SerializableComponent,
        builder: &mut EntityBuilder,
    ) -> Result<()>;
}

struct DirectConverter<T: SerializableComponent + hecs::Component + Clone> {
    _phantom: std::marker::PhantomData<T>,
}

impl<T: SerializableComponent + hecs::Component + Clone + 'static> DirectConverter<T> {
    fn new() -> Self {
        Self {
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<T: SerializableComponent + hecs::Component + Clone + 'static> ComponentConverter
    for DirectConverter<T>
{
    fn type_id(&self) -> TypeId {
        TypeId::of::<T>()
    }

    fn type_name(&self) -> &'static str {
        std::any::type_name::<T>()
    }

    fn serializable_type_id(&self) -> TypeId {
        TypeId::of::<T>()
    }

    fn extract_serializable(
        &self,
        world: &World,
        entity: Entity,
    ) -> Option<Box<dyn SerializableComponent>> {
        if let Ok(mut q) = world.query_one::<&T>(entity)
            && let Some(ty) = q.get()
        {
            return Some(ty.clone_boxed());
        }
        None
    }

    fn remove_component(&self, world: &mut World, entity: Entity) {
        let _ = world.remove_one::<T>(entity);
    }
}

/// Custom converter that has special logic for converting `T` to a [`SerializableComponent`]
struct CustomConverter<From, To, F>
where
    From: hecs::Component,
    To: SerializableComponent,
    F: Fn(&World, Entity, &From) -> Option<To> + Send + Sync,
{
    converter_fn: F,
    _phantom: std::marker::PhantomData<(From, To)>,
}

impl<From, To, F> CustomConverter<From, To, F>
where
    From: hecs::Component + 'static,
    To: SerializableComponent + 'static,
    F: Fn(&World, Entity, &From) -> Option<To> + Send + Sync,
{
    fn new(converter_fn: F) -> Self {
        Self {
            converter_fn,
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<From, To, F> ComponentConverter for CustomConverter<From, To, F>
where
    From: hecs::Component + 'static,
    To: SerializableComponent + 'static,
    F: Fn(&World, Entity, &From) -> Option<To> + Send + Sync,
{
    fn type_id(&self) -> TypeId {
        TypeId::of::<From>()
    }

    fn type_name(&self) -> &'static str {
        std::any::type_name::<From>()
    }

    fn serializable_type_id(&self) -> TypeId {
        TypeId::of::<To>()
    }

    fn extract_serializable(
        &self,
        world: &World,
        entity: Entity,
    ) -> Option<Box<dyn SerializableComponent>> {
        let component = world.get::<&From>(entity).ok()?;
        (self.converter_fn)(world, entity, &component)
            .map(|converted| Box::new(converted) as Box<dyn SerializableComponent>)
    }

    fn remove_component(&self, world: &mut World, entity: Entity) {
        let _ = world.remove_one::<From>(entity);
    }
}

struct DirectDeserializer<T: SerializableComponent + hecs::Component + Clone> {
    _phantom: std::marker::PhantomData<T>,
}

impl<T: SerializableComponent + hecs::Component + Clone + 'static> DirectDeserializer<T> {
    pub fn new() -> Self {
        Self {
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<T: SerializableComponent + hecs::Component + Clone + 'static> ComponentDeserializer
    for DirectDeserializer<T>
{
    fn serializable_type_id(&self) -> TypeId {
        TypeId::of::<T>()
    }

    fn serializable_type_name(&self) -> &'static str {
        std::any::type_name::<T>()
    }

    fn insert_into_builder(
        &self,
        component: &dyn SerializableComponent,
        builder: &mut EntityBuilder,
    ) -> Result<()> {
        let typed = component.as_any().downcast_ref::<T>().ok_or_else(|| {
            anyhow!(
                "Component '{}' does not match registered type '{}'",
                component.type_name(),
                std::any::type_name::<T>()
            )
        })?;
        builder.add(typed.clone());
        Ok(())
    }
}

struct CustomDeserializer<From, To, F>
where
    From: SerializableComponent,
    To: hecs::Component,
    F: Fn(&From) -> To + Send + Sync,
{
    converter_fn: F,
    _phantom: std::marker::PhantomData<(From, To)>,
}

impl<From, To, F> CustomDeserializer<From, To, F>
where
    From: SerializableComponent + 'static,
    To: hecs::Component + Clone + 'static,
    F: Fn(&From) -> To + Send + Sync + 'static,
{
    pub fn new(converter_fn: F) -> Self {
        Self {
            converter_fn,
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<From, To, F> ComponentDeserializer for CustomDeserializer<From, To, F>
where
    From: SerializableComponent + 'static,
    To: hecs::Component + Clone + 'static,
    F: Fn(&From) -> To + Send + Sync + 'static,
{
    fn serializable_type_id(&self) -> TypeId {
        TypeId::of::<From>()
    }

    fn serializable_type_name(&self) -> &'static str {
        std::any::type_name::<From>()
    }

    fn insert_into_builder(
        &self,
        component: &dyn SerializableComponent,
        builder: &mut EntityBuilder,
    ) -> Result<()> {
        let typed = component.as_any().downcast_ref::<From>().ok_or_else(|| {
            anyhow!(
                "Component '{}' cannot be deserialized by '{}'",
                component.type_name(),
                std::any::type_name::<From>()
            )
        })?;
        let rebuild = (self.converter_fn)(typed);
        builder.add(rebuild);
        Ok(())
    }
}
