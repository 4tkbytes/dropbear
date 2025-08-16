# Properties

## Type
Represents the different user definable properties by using a key-value pair. 

## Functions

### set_property
Sets a new entity-global property. 

#### Parameters
- `String` or `i64` or `f64` or `bool` or `Vector3` - Sets the property within the scope of the entity.

#### Example
```rhai
entity.set_property("speed", 0.5); // sets the property
entity.get_property("speed"); // returns 0.5 or () is not set
```

### get_property
Fetches a entity-global property. Search using a key and it will return that value, or a unit (`()`) if nothing exists. 

There are alternative functions such as `get_string`, `get_int`, `get_bool` or `get_vec3`, however this function is more dynamic and preferred. 

#### Parameters
- `key` - The key value that you are searching as a string value

#### Example
```rhai
entity.set_property("speed", 0.5); // sets the property
entity.get_property("speed"); // returns 0.5 or () is not set
entity.get_float("speed"); // same thing as above
```

### has_property
Checks if the entity has a property. Querying a key will result in either a true or false value.

#### Parameters
- `key` - The key value you are querying

#### Example
```rhai
entity.set_property("speed", 0.5);
entity.has_property("speed"); // returns true
entity.has_property("health"); // returns false
```