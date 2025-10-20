package com.dropbear.input

/**
 * Enum representing the different gamepad buttons.
 *
 * Enum taken from the `gilrs` crate
 *
 * See the [Controller layout](https://gilrs-project.gitlab.io/gilrs/img/controller.svg) image
 * for more information
 */
enum class GamepadButton {
    Unknown,
    South,
    East,
    North,
    West,
    C,
    Z,
    LeftTrigger,
    RightTrigger,
    LeftTrigger2,
    RightTrigger2,
    Select,
    Start,
    Mode,
    LeftThumb,
    RightThumb,
    DPadUp,
    DPadDown,
    DPadLeft,
    DPadRight,
}