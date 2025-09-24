//// The bridge used for connecting between the dropbear-engine and the
//// gleam WASM interface.
////
//// This can be ran on it's own, but it is pretty useless without attaching to an entity.
////
//// # Compile Pipeline
//// In the rust interface, it compiles Gleam using a Gleam -> JavaScript -> WASM (using javy). Despite using javy
//// creating large amounts of overhead, it is best choice for compiling JavaScript.
////
//// # Targets
//// Currently, Erlang targets do not do anything (unless you can compile to WASM, which I'm pretty sure you can't.
//// There is also no library available for a Gleam <-> Rust connection, nor is there a Erlang <-> Rust **practical**
//// connection. Also, using Erlang for a game engine is extremely impractical and stupid (and a pain to setup).
////
//// My other thoughts were to use OCaml, but OCaml has a high learning curve (as if Gleam doesn't already), and is
//// hard to setup up, including dealing with FFI.
////
//// Gleam just seemed like the best language: niche, rising to popularity, can be compiled and integrates with Rust,
//// and most importantly: Type safety.

import gleam/dict
import math
import gleam/option
import entity

/// The id returned during a query, which allows you to query for other features
/// 
/// A very primitive type. 
pub type QueryId {
  QueryId(id: Int)
}

/// Queries the current entity the script is attached to.
///
/// Returns None is the script is not attached to anything (i.e. a library), or Some if it is attached,
/// along with the entity information as shown in `dropbear/entity.Entity`.
pub fn query_current_entity() -> option.Option(entity.Entity) {
  option.Some(entity.Entity(0, transform: math.new_transform(), properties: dict.new(), dirty: False, before_dirty_entity: entity.BeforeSyncedEntity(id: 0, transform: math.new_transform(), properties: dict.new())))

}

/// A command that syncs/pushes the data back to the game engine interface.
///
/// It is typically automatically triggered at the end of a function (such as at the end of a move function), but can
/// be triggered manually by running the command.
///
/// The sync command takes an input of entity (so perfect for chaining), and then returning an entity. It also updates
/// the BeforeSyncedEntity to its current iteration.
pub fn sync(entity: entity.Entity) -> entity.Entity {
  case entity.dirty {
    True -> {
      // mock sync/push
      entity.Entity(..entity, before_dirty_entity: entity.new_before_synced_entity_from_existing_entity(entity))
    }
    False -> {
      // just return back the entity
      entity
    }
  }
}