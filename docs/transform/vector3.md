# Vector3

## Type
Represents a 3D vector (DVec3).

## Functions

### new_vec3
Creates a new `Vector3` with the given x, y, z values, or returns a vector with all components set to 1.0.

#### Parameters
- `x` - An f64 value for the x component.
- `y` - An f64 value for the y component.
- `z` - An f64 value for the z component.

#### Example
```rhai
let v = new_vec3(1.0, 2.0, 3.0);
let v = new_vec3(); // returns Vector3(1.0, 1.0, 1.0)
```

## Accessible Properties

### x
Gets or sets the x component.

#### Example
```rhai
let v = new_vec3(1.0, 2.0, 3.0);
v.x = 5.0;
let x = v.x;
```

### y
Gets or sets the y component.

#### Example
```rhai
let v = new_vec3(1.0, 2.0, 3.0);
v.y = 6.0;
let y = v.y;
```

### z
Gets or sets the z component.

#### Example
```rhai
let v = new_vec3(1.0, 2.0, 3.0);
v.z = 7.0;
```