//// A module for storing the properties of entities, as well as the manipulation of entities.

import gleam/dict
import types
import math

/// A standard type for an entity.
pub type Entity {
    Entity(
        /// The id/reference
        id: Int,
        /// The position, rotation and scale of the entity
        transform: math.Transform,
        /// The properties of the entity, stored in a gleam dictionary as a String key and
        /// Value type.
        properties: dict.Dict(String, types.Value),
        /// An internal value that checks if the entity is "dirty" and needs to be synced up.
        ///
        /// It can be manually synced up using the `dropbear.sync()` command, but in the case
        /// that it is not used, this flag pushes the changes at the end of the update function.
        dirty: Bool,
        before_dirty_entity: BeforeSyncedEntity,
    )
}

/// Creates a "dummy"/placeholder value for an Entity.
///
/// It's ID is always set to -1 so it cannot be used in any queries.
pub fn dummy() -> Entity {
    Entity(id: -1, transform: math.new_transform(), properties: dict.new(), dirty: False, before_dirty_entity: dummy_before())
}

/// A type used to checked the latest synced change. This is mainly internal, however you can use it
/// to check which values are dirty or not.
pub type BeforeSyncedEntity {
    BeforeSyncedEntity(
        /// The id/reference
        id: Int,
        /// The position, rotation and scale of the entity
        transform: math.Transform,
        /// The properties of the entity, stored in a gleam dictionary as a String key and
        /// Value type.
        properties: dict.Dict(String, types.Value),
    )
}

/// Creates a new dummy value for a BeforeSyncedEntity.
///
/// It's ID is always set to -1 so it cannot be used in any queries.
pub fn dummy_before() -> BeforeSyncedEntity {
    BeforeSyncedEntity(id: -1, transform: math.new_transform(), properties: dict.new())
}

/// A hella long name, creates a new BeforeSyncedEntity from an existing Entity.
///
/// It mainly exists as an internal helper, thats all...
pub fn new_before_synced_entity_from_existing_entity(entity: Entity) -> BeforeSyncedEntity {
    BeforeSyncedEntity(id: entity.id, transform: entity.transform, properties: entity.properties)
}

/// Sets the position of the entity
pub fn set_position(entity: Entity, position: math.Vector3(Float)) -> Entity {
    let transform = math.Transform(..entity.transform, position: position)
    Entity(..entity, transform:transform, dirty: True)
}