# Keyboard

## Type - InputState
The InputState is an uber-variable that contains all the functions of input. 

## Type - Key
This is a rhai module that is used for fetching the keycodes. Since the engine relys on the winit library, it uses keycodes. 

#### Examples
```rhai
key::W
key::ControlLeft
key::AltRight
```

## Functions

### is_pressed
Checks if a keyboard `key` is pressed. Returns true if presseed, false if not. 

#### Parameters
- `key` - A key value. Keycode is taken from winit.

#### Example
```rhai
if input.is_pressed(keys::W) {
    log("W is pressed!");
}
```