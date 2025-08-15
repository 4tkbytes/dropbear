# atan
Calculates the arctangent (inverse tangent) of a value.

## Parameters
 - `value` - An f64 value.

## Example
```rhai
atan(1.0); // returns PI/4
atan(0.0); // returns 0.0
```
---

# atan2
Calculates the arctangent of y/x using the signs of both arguments to determine the quadrant.

## Parameters
 - `y` - An f64 value representing the y-coordinate.
 - `x` - An f64 value representing the x-coordinate.

## Example
```rhai
atan2(1.0, 1.0); // returns PI/4
atan2(0.0, 1.0); // returns 0.0
```