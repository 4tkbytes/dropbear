//// The bridge used for connecting between the dropbear rust game engine and the
//// gleam WASM interface.

/// The id returned during a query, which allows you to query for other features
pub type QueryId {
  QueryId(id: Int)
}