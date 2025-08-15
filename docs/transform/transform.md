# Transform

## Type
Represents a 3D transformation, including position (Vector3), rotation (Quaternion), and scale.

## Functions

### new_transform
Creates a new `Transform` with default values.

#### Example
```rhai
let t = new_transform();
```

## Accessible Properties

### position
Gets or sets the position of the transform as a `Vector3`.

#### Example
```rhai
let t = new_transform();
t.position = new_vec3(1.0, 2.0, 3.0);
let pos = t.position;
```

### rotation
Gets or sets the rotation of the transform as a `Quaternion`.

#### Example
```rhai
let t = new_transform();
t.rotation = some_quaternion;
let rot = t.rotation;
```

### scale
Gets or sets the scale of the transform as a `Vector3`.

#### Example
```rhai
let t = new_transform();
t.scale = new_vec3(2.0, 2.0, 2.0);
let s = t.scale;
```
