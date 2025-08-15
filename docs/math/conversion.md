# to_radians
Converts degrees to radians.

## Parameters
 - `degrees` - An f64 value in degrees.

## Example
```rhai
to_radians(180.0); // returns PI
to_radians(90.0); // returns PI/2
```

---

# to_degrees
Converts radians to degrees.

## Parameters
 - `radians` - An f64 value in radians.

## Example
```rhai
to_degrees(PI); // returns 180.0
to_degrees(PI / 2); // returns 90.0
```