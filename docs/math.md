# Math

In this page, you will see functions relating to calculations.

## sin
It's a sine function that calculates the sine of an angle.

### Parameters
 - `angle` - An f64 value of the angle in radians. Can be useful when paired with time.

### Example
```rhai
sin(time());
sin(PI / 2); // returns 1.0
sin(0); // returns 0.0
```

## cos
Calculates the cosine of an angle.

### Parameters
 - `angle` - An f64 value of the angle in radians.

### Example
```rhai
cos(time());
cos(0); // returns 1.0
cos(PI); // returns -1.0
```

## tan
Calculates the tangent of an angle.

### Parameters
 - `angle` - An f64 value of the angle in radians.

### Example
```rhai
tan(time());
tan(PI / 4); // returns 1.0
tan(0); // returns 0.0
```

## asin
Calculates the arcsine (inverse sine) of a value.

### Parameters
 - `value` - An f64 value between -1.0 and 1.0.

### Example
```rhai
asin(1.0); // returns PI/2
asin(0.5); // returns PI/6
```

## acos
Calculates the arccosine (inverse cosine) of a value.

### Parameters
 - `value` - An f64 value between -1.0 and 1.0.

### Example
```rhai
acos(1.0); // returns 0.0
acos(0.0); // returns PI/2
```

## atan
Calculates the arctangent (inverse tangent) of a value.

### Parameters
 - `value` - An f64 value.

### Example
```rhai
atan(1.0); // returns PI/4
atan(0.0); // returns 0.0
```

## atan2
Calculates the arctangent of y/x using the signs of both arguments to determine the quadrant.

### Parameters
 - `y` - An f64 value representing the y-coordinate.
 - `x` - An f64 value representing the x-coordinate.

### Example
```rhai
atan2(1.0, 1.0); // returns PI/4
atan2(0.0, 1.0); // returns 0.0
```

## abs
Returns the absolute value of a number.

### Parameters
 - `value` - An f64 value.

### Example
```rhai
abs(-5.0); // returns 5.0
abs(3.14); // returns 3.14
```

## sqrt
Calculates the square root of a number.

### Parameters
 - `value` - An f64 value (must be non-negative).

### Example
```rhai
sqrt(9.0); // returns 3.0
sqrt(2.0); // returns approximately 1.414
```

## pow
Raises a number to the power of another number.

### Parameters
 - `base` - An f64 value representing the base.
 - `exponent` - An f64 value representing the exponent.

### Example
```rhai
pow(2.0, 3.0); // returns 8.0
pow(9.0, 0.5); // returns 3.0 (square root)
```

## min
Returns the smaller of two values.

### Parameters
 - `a` - An f64 value.
 - `b` - An f64 value.

### Example
```rhai
min(5.0, 3.0); // returns 3.0
min(-1.0, 2.0); // returns -1.0
```

## max
Returns the larger of two values.

### Parameters
 - `a` - An f64 value.
 - `b` - An f64 value.

### Example
```rhai
max(5.0, 3.0); // returns 5.0
max(-1.0, 2.0); // returns 2.0
```

## clamp
Constrains a value between a minimum and maximum value.

### Parameters
 - `value` - An f64 value to clamp.
 - `min` - An f64 minimum value.
 - `max` - An f64 maximum value.

### Example
```rhai
clamp(5.0, 0.0, 10.0); // returns 5.0
clamp(-5.0, 0.0, 10.0); // returns 0.0
clamp(15.0, 0.0, 10.0); // returns 10.0
```

## exp
Calculates e raised to the power of a number.

### Parameters
 - `value` - An f64 value representing the exponent.

### Example
```rhai
exp(1.0); // returns E (approximately 2.718)
exp(0.0); // returns 1.0
```

## ln
Calculates the natural logarithm (base e) of a number.

### Parameters
 - `value` - An f64 value (must be positive).

### Example
```rhai
ln(E); // returns 1.0
ln(1.0); // returns 0.0
```

## log10
Calculates the base-10 logarithm of a number.

### Parameters
 - `value` - An f64 value (must be positive).

### Example
```rhai
log10(100.0); // returns 2.0
log10(10.0); // returns 1.0
```

## log2
Calculates the base-2 logarithm of a number.

### Parameters
 - `value` - An f64 value (must be positive).

### Example
```rhai
log2(8.0); // returns 3.0
log2(2.0); // returns 1.0
```

## floor
Returns the largest integer less than or equal to a number.

### Parameters
 - `value` - An f64 value.

### Example
```rhai
floor(3.7); // returns 3.0
floor(-2.1); // returns -3.0
```

## ceil
Returns the smallest integer greater than or equal to a number.

### Parameters
 - `value` - An f64 value.

### Example
```rhai
ceil(3.1); // returns 4.0
ceil(-2.9); // returns -2.0
```

## round
Rounds a number to the nearest integer.

### Parameters
 - `value` - An f64 value.

### Example
```rhai
round(3.7); // returns 4.0
round(3.2); // returns 3.0
round(-2.6); // returns -3.0
```

## trunc
Returns the integer part of a number by removing the fractional part.

### Parameters
 - `value` - An f64 value.

### Example
```rhai
trunc(3.7); // returns 3.0
trunc(-2.9); // returns -2.0
```

## fract
Returns the fractional part of a number.

### Parameters
 - `value` - An f64 value.

### Example
```rhai
fract(3.7); // returns 0.7
fract(-2.3); // returns 0.7
```

## to_radians
Converts degrees to radians.

### Parameters
 - `degrees` - An f64 value in degrees.

### Example
```rhai
to_radians(180.0); // returns PI
to_radians(90.0); // returns PI/2
```

## to_degrees
Converts radians to degrees.

### Parameters
 - `radians` - An f64 value in radians.

### Example
```rhai
to_degrees(PI); // returns 180.0
to_degrees(PI / 2); // returns 90.0
```

## lerp
Performs linear interpolation between two values.

### Parameters
 - `a` - An f64 start value.
 - `b` - An f64 end value.
 - `t` - An f64 interpolation factor (typically between 0.0 and 1.0).

### Example
```rhai
lerp(0.0, 10.0, 0.5); // returns 5.0
lerp(2.0, 8.0, 0.25); // returns 3.5
```

## smoothstep
Performs smooth Hermite interpolation between two edges.

### Parameters
 - `edge0` - An f64 lower edge value.
 - `edge1` - An f64 upper edge value.
 - `x` - An f64 input value.

### Example
```rhai
smoothstep(0.0, 1.0, 0.5); // returns 0.5
smoothstep(0.0, 1.0, 0.0); // returns 0.0
smoothstep(0.0, 1.0, 1.0); // returns 1.0
```

## sinh
Calculates the hyperbolic sine of a number.

### Parameters
 - `value` - An f64 value.

### Example
```rhai
sinh(0.0); // returns 0.0
sinh(1.0); // returns approximately 1.175
```

## cosh
Calculates the hyperbolic cosine of a number.

### Parameters
 - `value` - An f64 value.

### Example
```rhai
cosh(0.0); // returns 1.0
cosh(1.0); // returns approximately 1.543
```

## tanh
Calculates the hyperbolic tangent of a number.

### Parameters
 - `value` - An f64 value.

### Example
```rhai
tanh(0.0); // returns 0.0
tanh(1.0); // returns approximately 0.762
```

## Constants

### PI
Mathematical constant π (pi) ≈ 3.14159265359

### E
Mathematical constant e (Euler's number) ≈ 2.71828182846

### TAU
Mathematical constant τ (tau) = 2π ≈ 6.28318530718