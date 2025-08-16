# Mouse

## Type - InputState
The InputState is an uber-variable that contains all the functions of input. 

## Functions

### lock_cursor
Locks the cursor to the center. Useful for tracking the camera. Camera tracking is already managed, you just need to toggle it such as if a user goes into an inventory. 

#### Parameters
- `bool` - Value passed on to locking or unlocking. 

#### Example
```rhai
if input.is_pressed(key::E) {
    input.lock_cursor(false);
} else {
    input.lock_cursor(true);
}
```

## Accessible Properties

### mouse_x
Fetches the x value of the mouse relative to the window
#### Example
```rhai
input.mouse_x(); // returns the mouse x val
```

### mouse_y
Fetches the y value of the mouse relative to the window
#### Example
```rhai
input.mouse_y(); // returns the mouse y val
```

### mouse_dx
Fetches the change of mouse position since the last frame value of the mouse relative to the window (like deltaX)
#### Example
```rhai
input.mouse_dx(); // returns the mouse dx val
```

### mouse_dy
Fetches the change of mouse position since the last frame value of the mouse relative to the window (like deltaY)
#### Example
```rhai
input.mouse_dy(); // returns the mouse dy val
```