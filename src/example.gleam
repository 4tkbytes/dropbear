//// A dummy file, do not use. I mean, use it if you want, its public for a reason...

import gleam/io
import gleam/string
import gleam/option.{Some}
import math
import entity
import dropbear

pub fn main() {
  load()
  update(0.016)
}

pub fn load() -> Nil {
  let _ = case dropbear.query_current_entity() {
    Some(entity) -> {
        entity.set_position(entity, math.Vector3(1.0, 1.0, 1.0))
        |> dropbear.sync()
    }
    option.None -> {
      entity.dummy()
    }
  }

  io.println("Loaded!")
}

pub fn update(dt: Float) -> Nil {
  io.println("Updating...")
  let _ = case dropbear.query_current_entity() {
    Some(entity) -> {
      io.println("Successfully queried entity!")
      entity.set_position(entity, math.Vector3(1.0, 1.0, 1.0))
      |> dropbear.sync()
    }
    option.None -> {
      io.println("Could not query entity, creating a dummy entity")
      entity.dummy()
    }
  }
  io.println("Deltatime is " <> string.inspect(dt))
  update(dt)
}