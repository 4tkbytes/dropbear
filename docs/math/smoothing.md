# lerp
Performs linear interpolation between two values.

## Parameters
 - `a` - An f64 start value.
 - `b` - An f64 end value.
 - `t` - An f64 interpolation factor (typically between 0.0 and 1.0).

## Example
```rhai
lerp(0.0, 10.0, 0.5); // returns 5.0
lerp(2.0, 8.0, 0.25); // returns 3.5
```

---

# smoothstep
Performs smooth Hermite interpolation between two edges.

## Parameters
 - `edge0` - An f64 lower edge value.
 - `edge1` - An f64 upper edge value.
 - `x` - An f64 input value.

## Example
```rhai
smoothstep(0.0, 1.0, 0.5); // returns 0.5
smoothstep(0.0, 1.0, 0.0); // returns 0.0
smoothstep(0.0, 1.0, 1.0); // returns 1.0
```