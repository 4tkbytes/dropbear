# clamp
Constrains a value between a minimum and maximum value.

## Parameters
 - `value` - An f64 value to clamp.
 - `min` - An f64 minimum value.
 - `max` - An f64 maximum value.

## Example
```rhai
clamp(5.0, 0.0, 10.0); // returns 5.0
clamp(-5.0, 0.0, 10.0); // returns 0.0
clamp(15.0, 0.0, 10.0); // returns 10.0
```