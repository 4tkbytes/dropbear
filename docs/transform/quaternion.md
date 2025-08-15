# Quaternion

## Type
Represents a quaternion rotation (DQuat).

## Functions

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