// Modules for the dropbear-engine scripting component
// Made by 4tkbytes
// EDIT THIS IF YOU WISH, MOST LIKELY 

type TransformT = any;
type Vec3T = any;
type QuatT = any;
type InputStateT = any;

/**
 * A placeholder function that returns the value of the host function
 */
function hostFn(name: string) {
    const fn = (globalThis as any)[name];
    if (typeof fn !== "function") {
        throw new Error(`Host function ${name}() is not available. Make sure the runtime exposes it.`);
    }
    return fn;
}

const Transform = {
    create: (): TransformT => hostFn("createTransform")(),
    translate: (transform: TransformT, translation: [number, number, number] | number[]): TransformT =>
        hostFn("transformTranslate")(transform, translation),
    rotateX: (transform: TransformT, angle: number): TransformT =>
        hostFn("transformRotateX")(transform, angle),
    rotateY: (transform: TransformT, angle: number): TransformT =>
        hostFn("transformRotateY")(transform, angle),
    rotateZ: (transform: TransformT, angle: number): TransformT =>
        hostFn("transformRotateZ")(transform, angle),
    scale: (transform: TransformT, scale: number | [number, number, number]): TransformT =>
        hostFn("transformScale")(transform, scale),
    matrix: (transform: TransformT): TransformT => hostFn("transformMatrix")(transform),
};

/**
 * A Vector of 3 components: an X, Y and Z. 
 */
const Vec3 = {
    new: (x = 0, y = 0, z = 0): Vec3T => hostFn("createVec3")(x, y, z),
    zero: (): Vec3T => hostFn("createVec3")(0, 0, 0),
    one: (): Vec3T => hostFn("createVec3")(1, 1, 1),
};

const Quaternion = {
    identity: (): QuatT => hostFn("createQuatIdentity")(),
    fromEuler: (x: number, y: number, z: number): QuatT => hostFn("createQuatFromEuler")(x, y, z),
};

export const Keys = {
    // Letters
    KeyA: "KeyA", KeyB: "KeyB", KeyC: "KeyC", KeyD: "KeyD", KeyE: "KeyE", KeyF: "KeyF", 
    KeyG: "KeyG", KeyH: "KeyH", KeyI: "KeyI", KeyJ: "KeyJ", KeyK: "KeyK", KeyL: "KeyL", 
    KeyM: "KeyM", KeyN: "KeyN", KeyO: "KeyO", KeyP: "KeyP", KeyQ: "KeyQ", KeyR: "KeyR", 
    KeyS: "KeyS", KeyT: "KeyT", KeyU: "KeyU", KeyV: "KeyV", KeyW: "KeyW", KeyX: "KeyX", 
    KeyY: "KeyY", KeyZ: "KeyZ",
    
    // Numbers
    Digit0: "Digit0", Digit1: "Digit1", Digit2: "Digit2", Digit3: "Digit3", Digit4: "Digit4",
    Digit5: "Digit5", Digit6: "Digit6", Digit7: "Digit7", Digit8: "Digit8", Digit9: "Digit9",
    
    // Special keys
    Space: "Space",
    ShiftLeft: "ShiftLeft",
    ShiftRight: "ShiftRight",
    ControlLeft: "ControlLeft",
    ControlRight: "ControlRight",
    AltLeft: "AltLeft",
    AltRight: "AltRight",
    Escape: "Escape",
    Enter: "Enter",
    Tab: "Tab",
    ArrowUp: "ArrowUp",
    ArrowDown: "ArrowDown",
    ArrowLeft: "ArrowLeft",
    ArrowRight: "ArrowRight",
    
    // Function keys
    F1: "F1", F2: "F2", F3: "F3", F4: "F4", F5: "F5", F6: "F6",
    F7: "F7", F8: "F8", F9: "F9", F10: "F10", F11: "F11", F12: "F12",
} as const;

export type KeyCode = typeof Keys[keyof typeof Keys];

export const Input = {
    /**
     * Check if a key is currently pressed
     * @param inputState The current input state
     * @param key The key to check
     * @returns true if the key is pressed, false otherwise
     */
    isKeyPressed: (inputState: InputStateT, key: KeyCode): boolean => 
        hostFn("isKeyPressed")(inputState, key),

    /**
     * Get the current mouse X position
     * @param inputState The current input state
     * @returns The mouse X coordinate
     */
    getMouseX: (inputState: InputStateT): number => 
        hostFn("getMouseX")(inputState),

    /**
     * Get the current mouse Y position
     * @param inputState The current input state
     * @returns The mouse Y coordinate
     */
    getMouseY: (inputState: InputStateT): number => 
        hostFn("getMouseY")(inputState),

    /**
     * Get the mouse delta X (movement since last frame)
     * @param inputState The current input state
     * @returns The mouse delta X
     */
    getMouseDeltaX: (inputState: InputStateT): number => 
        hostFn("getMouseDeltaX")(inputState),

    /**
     * Get the mouse delta Y (movement since last frame)
     * @param inputState The current input state
     * @returns The mouse delta Y
     */
    getMouseDeltaY: (inputState: InputStateT): number => 
        hostFn("getMouseDeltaY")(inputState),

    /**
     * Lock or unlock the cursor
     * @param locked Whether to lock the cursor
     */
    lockCursor: (locked: boolean): void => 
        hostFn("lockCursor")(locked),
};

export const EntityProperties = {
    /**
     * Get a property value (generic)
     * @param properties The entity properties object
     * @param key The property key
     * @returns The property value or null if not found
     */
    getProperty: (properties: any, key: string): any =>
        hostFn("getProperty")(properties, key),

    /**
     * Set a string property
     * @param properties The entity properties object
     * @param key The property key
     * @param value The string value
     * @returns Updated properties object
     */
    setPropertyString: (properties: any, key: string, value: string): any =>
        hostFn("setPropertyString")(properties, key, value),

    /**
     * Set an integer property
     * @param properties The entity properties object
     * @param key The property key
     * @param value The integer value
     * @returns Updated properties object
     */
    setPropertyInt: (properties: any, key: string, value: number): any =>
        hostFn("setPropertyInt")(properties, key, value),

    /**
     * Set a float property
     * @param properties The entity properties object
     * @param key The property key
     * @param value The float value
     * @returns Updated properties object
     */
    setPropertyFloat: (properties: any, key: string, value: number): any =>
        hostFn("setPropertyFloat")(properties, key, value),

    /**
     * Set a boolean property
     * @param properties The entity properties object
     * @param key The property key
     * @param value The boolean value
     * @returns Updated properties object
     */
    setPropertyBool: (properties: any, key: string, value: boolean): any =>
        hostFn("setPropertyBool")(properties, key, value),

    /**
     * Set a Vec3 property
     * @param properties The entity properties object
     * @param key The property key
     * @param value The Vec3 value as [x, y, z] array
     * @returns Updated properties object
     */
    setPropertyVec3: (properties: any, key: string, value: [number, number, number]): any =>
        hostFn("setPropertyVec3")(properties, key, value),

    /**
     * Get a string property
     * @param properties The entity properties object
     * @param key The property key
     * @returns The string value or empty string if not found
     */
    getString: (properties: any, key: string): string =>
        hostFn("getString")(properties, key),

    /**
     * Get an integer property
     * @param properties The entity properties object
     * @param key The property key
     * @returns The integer value or 0 if not found
     */
    getInt: (properties: any, key: string): number =>
        hostFn("getInt")(properties, key),

    /**
     * Get a float property
     * @param properties The entity properties object
     * @param key The property key
     * @returns The float value or 0.0 if not found
     */
    getFloat: (properties: any, key: string): number =>
        hostFn("getFloat")(properties, key),

    /**
     * Get a boolean property
     * @param properties The entity properties object
     * @param key The property key
     * @returns The boolean value or false if not found
     */
    getBool: (properties: any, key: string): boolean =>
        hostFn("getBool")(properties, key),

    /**
     * Get a Vec3 property
     * @param properties The entity properties object
     * @param key The property key
     * @returns The Vec3 value as [x, y, z] array or [0, 0, 0] if not found
     */
    getVec3: (properties: any, key: string): [number, number, number] =>
        hostFn("getVec3")(properties, key),

    /**
     * Check if a property exists
     * @param properties The entity properties object
     * @param key The property key
     * @returns True if the property exists, false otherwise
     */
    hasProperty: (properties: any, key: string): boolean =>
        hostFn("hasProperty")(properties, key),
};

export { Transform, Vec3, Quaternion };

globalThis.Transform = Transform;
globalThis.Vec3 = Vec3;
globalThis.Quaternion = Quaternion;
globalThis.Input = Input;
globalThis.Keys = Keys;
globalThis.EntityProperties = EntityProperties;