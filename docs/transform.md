# Transform

This page documents the functions and types related to position, rotation, and scale manipulation.

## Types

### Transform
Represents a 3D transformation, including position, rotation, and scale.

### Quaternion
Represents a quaternion rotation (DQuat).

### Vector3
Represents a 3D vector (DVec3).

## Functions

### new_transform
Creates a new `Transform` with default values.

#### Example
```rhai
let t = new_transform();
```

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

### new_quat
Creates a new `Quaternion` with given angle and axis values, or returns an identity Quaternion.

#### Parameters

- `axis` - A Vector3 axis value.
- `angle` - The angle of rotation in radians.

#### Example
```rhai
let q = new_quat(new_vec3(), degrees_to_radians(90));
let q = new_quat();
```

## Properties

### Transform

#### position
Gets or sets the position of the transform as a `Vector3`.

#### Example
```rhai
let t = new_transform();
t.position = new_vec3(1.0, 2.0, 3.0);
let pos = t.position;
```

#### rotation
Gets or sets the rotation of the transform as a `Quaternion`.

#### Example
```rhai
let t = new_transform();
t.rotation = some_quaternion;
let rot = t.rotation;
```

#### scale
Gets or sets the scale of the transform as a `Vector3`.

#### Example
```rhai
let t = new_transform();
t.scale = new_vec3(2.0, 2.0, 2.0);
let s = t.scale;
```

### Vector3

#### x
Gets or sets the x component.

#### Example
```rhai
let v = new_vec3(1.0, 2.0, 3.0);
v.x = 5.0;
let x = v.x;
```

#### y
Gets or sets the y component.

#### Example
```rhai
let v = new_vec3(1.0, 2.0, 3.0);
v.y = 6.0;
let y = v.y;
```

#### z
Gets or sets the z component.

#### Example
```rhai
let v = new_vec3(1.0, 2.0, 3.0);
v.z = 7.0;
```