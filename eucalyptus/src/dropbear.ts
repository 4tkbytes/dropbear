// Modules for the dropbear-engine scripting component
// Made by 4tkbytes
// EDIT THIS IF YOU WISH, RECOMMENDED TO NOT TOUCH IT

/**
 * A class describing the position, scale and rotation to be able to manipulate
 * the entity's location. 
 */
export class Transform {
    /**
     * A {@link Vector3} describing the position of the entity
     */
    position: Vector3;
    /**
     * A {@link Quaternion} describing the rotation of the entity
     */
    rotation: Quaternion;
    /**
     * A {@link Vector3} describing the scale of the entity
     */
    scale: Vector3;

    public constructor(position?: Vector3, rotation?: Quaternion, scale?: Vector3) {
        this.position = position || Vector3.zero();
        this.rotation = rotation || Quaternion.identity();
        this.scale = scale || Vector3.one();
    }

    /**
     * Translates/Offsets the position of the entity by a {@link Vector3}
     * 
     * # Example
     * @example
     * ```ts
     * let transform = new Transform();
     * transform.translate(new Vector3(1.0, 1.0, 1.0));
     * ```
     * 
     * @param movement - A {@link Vector3} for position
     */
    public translate(movement: Vector3) {
        this.position.x += movement.x;
        this.position.y += movement.y;
        this.position.z += movement.z;
    }

    /**
     * Rotates the transformables rotation on the X axis
     * 
     * # Example
     * @example
     * ```ts
     * let transform = new Transform();
     * transform.rotateX(dbMath.degreesToRadians(180));
     * ```
     * 
     * @param angle - The angle in radians
     */
    public rotateX(angle: number) {
        const rotQuat = Quaternion.fromAxisAngle(new Vector3(1, 0, 0), angle);
        this.rotation = Quaternion.multiply(this.rotation, rotQuat);
        
    }

    /**
     * Rotates the transformables rotation on the Y axis
     * 
     * # Example
     * ```ts
     * let transform = new Transform();
     * transform.rotateY(dbMath.degreesToRadians(180))
     * ```
     * 
     * @param angle - The angle in radians
     */
    public rotateY(angle: number) {
        const rotQuat = Quaternion.fromAxisAngle(new Vector3(0, 1, 0), angle);
        this.rotation = Quaternion.multiply(this.rotation, rotQuat);        
    }

    /**
     * Rotates the transformables rotation on the Z axis
     * 
     * # Example
     * ```ts
     * let transform = new Transform();
     * transform.rotateZ(dbMath.degreesToRadians(180))
     * ```
     * 
     * @param angle - The angle in radians
     */
    public rotateZ(angle: number) {
        const rotQuat = Quaternion.fromAxisAngle(new Vector3(0, 0, 1), angle);
        this.rotation = Quaternion.multiply(this.rotation, rotQuat);
    }

    /**
     * Uniformly scales the entity by a multiplier. 
     * 
     * # Example
     * ```ts
     * let transform = new Transform();
     * transform.scaleUniform(2.0)
     * ```
     * 
     * @param scale - A number that the scale multiplies by
     */
    public scaleUniform(scale: number) {
        this.scale.x *= scale;
        this.scale.y *= scale;
        this.scale.z *= scale;
    }

    /**
     * Individually scales the entity by a multiplier by using a {@link Vector3}
     * 
     * # Example
     * ```ts
     * let transform = new Transform();
     * transform.scaleIndividual(new Vector3(1.0, 2.0, 1.5))
     * ```
     * 
     * @param scale - A Vector3 representing the scale.x, scale.y and scale.z values
     */
    public scaleIndividual(scale: Vector3) {
        this.scale.x *= scale.x;
        this.scale.y *= scale.y;
        this.scale.z *= scale.z;
    }
}

/**
 * Utilities for math functions that do not exist in the TypeScript Math module. 
 */
export const dbMath = {
    /**
     * Convert from degrees to radians
     * 
     * # Example
     * @example
     * ```ts
     * console.log(dbMath.degreesToRadians(180)) // expect Math.PI
     * ```
     * 
     * @param deg - The angle in degrees
     * @returns The angle in radians
     */
    degreesToRadians:(deg: number):number => {
        return deg * (Math.PI / 180.0);
    },

    /**
     * Convert from radians to degrees
     * 
     * # Example
     * @example
     * ```ts
     * console.log(dbMath.radiansToDegrees(Math.PI)) // expect 180
     * ```
     * 
     * @param rad - The angle in radians
     * @returns The angle in degrees
     */
    radiansToDegrees:(rad: number):number => {
        return (180 * rad) / Math.PI;
    },

    /**
     * Constrains a number to lie within a specified range. If value is less than min, returns min. 
     * If value is greater than max, returns max. Otherwise, returns value.
     * 
     * @example
     * ```ts
     * dropbear.dbMath.clamp(5, 0, 10)     // → 5
     * dropbear.dbMath.clamp(-3, 0, 10)    // → 0
     * dropbear.dbMath.clamp(15, 0, 10)    // → 10
     * ```
     * 
     * @param value - The input value to clamp
     * @param min - The lower bound of the range
     * @param max - The upper bound of the range
     * @returns - The clamped value
     */
    clamp: (value: number, min: number, max: number): number => {
        return Math.min(Math.max(value, min), max);
    }
}

/**
 * A Vector of 3 components: an X, Y and Z. 
 */
export class Vector3 {
    /**
     * The X value
     */
    public x: number;

    /**
     * The Y value
     */
    public y: number;
    /**
     * The Z value
     */
    public z: number;

    public constructor(x: number, y: number, z: number) {
        this.x = x;
        this.y = y;
        this.z = z;
    }

    /**
     * Converts the {@link Vector3} class to a primitive number array
     * 
     * # Example
     * @example
     * ```ts
     * let vec = new Vector3(1.0, 1.5, 2.0);
     * console.log(vec.as_array()) // expect [1.0, 1.5, 2.0]
     * ```
     * 
     * @returns A number array representing the x, y, z values
     */
    public as_array(): [number, number, number] {
        return [this.x, this.y, this.z];
    }

    /**
     * An alternative static constructor to create a {@link Vector3} with all values 
     * set to 0.0
     * 
     * # Example
     * ```ts
     * let vec = Vector3.zero();
     * console.log(vec); // expect [0.0, 0.0, 0.0]
     * ```
     * 
     * @returns A new Vector3 instance with all values set to 0.0
     */
    public static zero(): Vector3 {
        return new Vector3(0.0, 0.0, 0.0);
    }

    /**
     * An alternative static constructor to create a {@link Vector3} with all values 
     * set to 1.0
     * 
     * # Example
     * ```ts
     * let vec = Vector3.one();
     * console.log(vec); // expect [1.0, 1.0, 1.0]
     * ```
     * 
     * @returns A new Vector3 instance with all values set to 1.0
     */
    public static one(): Vector3 {
        return new Vector3(1.0, 1.0, 1.0);
    }

    /**
     * An alternative static constructor that creates a Vector3 from a number array.
     * 
     * # Example
     * @example
     * ```ts
     * let arr = [1.0, 1.5, 2.0];
     * let vec = Vector3.fromArray(arr);
     * console.log(vec.to_array() === arr) // expect to print 'true'
     * ```
     *  
     * @param arr - The number array to convert
     * @returns - A new Vector3 instance
     */
    public static fromArray(arr: [number, number, number]): Vector3 {
        return new Vector3(arr[0], arr[1], arr[2]);
    }

    /**
     * Add another Vector3 to this vector and return the result as a new Vector3.
     *
     * @param rhs - Right-hand side vector to add.
     * @returns A new Vector3 equal to (this + rhs).
     *
     * @example
     * const a = new Vector3(1, 2, 3);
     * const b = new Vector3(4, 5, 6);
     * const c = a.add(b); // Vector3(5,7,9)
     */
    public add(rhs: Vector3): Vector3 {
        return new Vector3(this.x + rhs.x, this.y + rhs.y, this.z + rhs.z);
    }

    /**
     * Subtract another Vector3 from this vector and return the result as a new Vector3.
     *
     * @param rhs - Right-hand side vector to subtract.
     * @returns A new Vector3 equal to (this - rhs).
     *
     * @example
     * const a = new Vector3(5, 7, 9);
     * const b = new Vector3(1, 2, 3);
     * const c = a.subtract(b); // Vector3(4,5,6)
     */
    public subtract(rhs: Vector3): Vector3 {
        return new Vector3(this.x - rhs.x, this.y - rhs.y, this.z - rhs.z);
    }

    /**
     * Multiply this vector by a scalar and return the result as a new Vector3.
     *
     * @param rhs - Scalar multiplier.
     * @returns A new Vector3 scaled by rhs.
     *
     * @example
     * const v = new Vector3(1, 2, 3);
     * const s = v.multiply(2); // Vector3(2,4,6)
     */
    public multiply(rhs: number): Vector3 {
        return new Vector3(this.x * rhs, this.y * rhs, this.z * rhs);
    }

    /**
     * Compute the Euclidean length (magnitude) of this vector.
     *
     * @returns The length sqrt(x*x + y*y + z*z).
     *
     * @example
     * const v = new Vector3(1, 2, 2);
     * console.log(v.length()); // 3
     */
    public length(): number {
        return Math.sqrt(this.x * this.x + this.y * this.y + this.z * this.z);
    }

    /**
     * Return a new Vector3 representing the normalized (unit) direction of this vector.
     * If the vector has zero length, returns Vector3.zero().
     *
     * @returns A normalized Vector3 or Vector3.zero() when length is 0.
     *
     * @example
     * const v = new Vector3(0, 3, 4);
     * const n = v.normalize(); // Vector3(0, 0.6, 0.8)
     */
    public normalize(): Vector3 {
        const len = this.length();
        if (len === 0) return Vector3.zero();
        return new Vector3(this.x / len, this.y / len, this.z / len);
    }
}

/**
 * Quaternion representing a rotation in 3D space.
 *
 * Components are stored as (x, y, z, w) where w is the scalar part.
 * Useful helpers are provided to construct quaternions from Euler angles,
 * axis/angle, multiply them, and convert to/from arrays.
 */
export class Quaternion {
    public x: number;
    public y: number;
    public z: number;
    public w: number;

    /**
     * Create a new quaternion.
     *
     * @param x - X component (default 0)
     * @param y - Y component (default 0)
     * @param z - Z component (default 0)
     * @param w - W (scalar) component (default 1)
     *
     * @example
     * const q = new Quaternion(); // identity
     * const q2 = new Quaternion(0.1, 0.2, 0.3, 0.9);
     */
    public constructor(x: number = 0, y: number = 0, z: number = 0, w: number = 1) {
        this.x = x;
        this.y = y;
        this.z = z;
        this.w = w;
    }

    /**
     * Construct a quaternion from Euler angles (radians).
     *
     * The parameters represent rotations about the X, Y and Z axes respectively.
     * Angles must be provided in radians.
     *
     * @param x - rotation about the X axis in radians
     * @param y - rotation about the Y axis in radians
     * @param z - rotation about the Z axis in radians
     * @returns A new Quaternion representing the composite rotation
     *
     * @example
     * const q = Quaternion.fromEuler(Math.PI/2, 0, 0); // 90° around X
     */
    public static fromEuler(x: number, y: number, z: number): Quaternion {
        // Convert euler angles to quaternion
        const cx = Math.cos(x * 0.5);
        const sx = Math.sin(x * 0.5);
        const cy = Math.cos(y * 0.5);
        const sy = Math.sin(y * 0.5);
        const cz = Math.cos(z * 0.5);
        const sz = Math.sin(z * 0.5);

        return new Quaternion(
            sx * cy * cz - cx * sy * sz,
            cx * sy * cz + sx * cy * sz,
            cx * cy * sz - sx * sy * cz,
            cx * cy * cz + sx * sy * sz
        );
    }

    /**
     * Construct a quaternion representing a rotation around an axis.
     *
     * The axis will be normalized internally. Angle is in radians.
     *
     * @param axis - Rotation axis as a Vector3
     * @param angle - Rotation angle in radians
     * @returns A new Quaternion representing the axis-angle rotation
     *
     * @example
     * const q = Quaternion.fromAxisAngle(new Vector3(0,1,0), Math.PI); // 180° around Y
     */
    public static fromAxisAngle(axis: Vector3, angle: number): Quaternion {
        const halfAngle = angle * 0.5;
        const sin = Math.sin(halfAngle);
        const normalizedAxis = axis.normalize();
        
        return new Quaternion(
            normalizedAxis.x * sin,
            normalizedAxis.y * sin,
            normalizedAxis.z * sin,
            Math.cos(halfAngle)
        );
    }

    /**
     * Multiply two quaternions.
     *
     * The result corresponds to the composition a * b (apply b, then a) using
     * the multiplication implemented here.
     *
     * @param a - Left quaternion
     * @param b - Right quaternion
     * @returns The product quaternion
     *
     * @example
     * const r = Quaternion.multiply(q1, q2);
     */
    public static multiply(a: Quaternion, b: Quaternion): Quaternion {
        return new Quaternion(
            a.w * b.x + a.x * b.w + a.y * b.z - a.z * b.y,
            a.w * b.y - a.x * b.z + a.y * b.w + a.z * b.x,
            a.w * b.z + a.x * b.y - a.y * b.x + a.z * b.w,
            a.w * b.w - a.x * b.x - a.y * b.y - a.z * b.z
        );
    }

    /**
     * Create a quaternion from a numeric array [x, y, z, w].
     *
     * @param arr - Array containing quaternion components in order [x, y, z, w]
     * @returns A new Quaternion with components taken from the array
     *
     * @example
     * const q = Quaternion.fromArray([0, 0, 0, 1]);
     */
    public static fromArray(arr: [number, number, number, number]): Quaternion {
        return new Quaternion(arr[0], arr[1], arr[2], arr[3]);
    }

    /**
     * Return the identity quaternion (no rotation).
     *
     * @returns Quaternion equal to (0, 0, 0, 1)
     *
     * @example
     * const id = Quaternion.identity();
     */
    public static identity(): Quaternion {
        return new Quaternion(0.0, 0.0, 0.0, 1.0);
    }

    /**
     * Convert this quaternion to a numeric array [x, y, z, w].
     *
     * @returns An array containing the quaternion components
     *
     * @example
     * const arr = q.as_array(); // [x, y, z, w]
     */
    public as_array(): [number, number, number, number] {
        return [this.x, this.y, this.z, this.w];
    }
}

/**
 * Values of the Key codes. 
 */
export const Keys = {
    KeyA: "KeyA", KeyB: "KeyB", KeyC: "KeyC", KeyD: "KeyD", KeyE: "KeyE", KeyF: "KeyF", 
    KeyG: "KeyG", KeyH: "KeyH", KeyI: "KeyI", KeyJ: "KeyJ", KeyK: "KeyK", KeyL: "KeyL", 
    KeyM: "KeyM", KeyN: "KeyN", KeyO: "KeyO", KeyP: "KeyP", KeyQ: "KeyQ", KeyR: "KeyR", 
    KeyS: "KeyS", KeyT: "KeyT", KeyU: "KeyU", KeyV: "KeyV", KeyW: "KeyW", KeyX: "KeyX", 
    KeyY: "KeyY", KeyZ: "KeyZ",
    Digit0: "Digit0", Digit1: "Digit1", Digit2: "Digit2", Digit3: "Digit3", Digit4: "Digit4",
    Digit5: "Digit5", Digit6: "Digit6", Digit7: "Digit7", Digit8: "Digit8", Digit9: "Digit9",
    Space: "Space",
    ShiftLeft: "ShiftLeft", ShiftRight: "ShiftRight",
    ControlLeft: "ControlLeft", ControlRight: "ControlRight",
    AltLeft: "AltLeft", AltRight: "AltRight",
    Escape: "Escape", Enter: "Enter", Tab: "Tab",
    ArrowUp: "ArrowUp", ArrowDown: "ArrowDown", ArrowLeft: "ArrowLeft", ArrowRight: "ArrowRight",
    F1: "F1", F2: "F2", F3: "F3", F4: "F4", F5: "F5", F6: "F6",
    F7: "F7", F8: "F8", F9: "F9", F10: "F10", F11: "F11", F12: "F12",
} as const;

export type KeyCode = typeof Keys[keyof typeof Keys];

/**
 * The properties of the entity. 
 * From the eucalyptus editor, you are able to make custom 
 * properties such as 'speed' or 'health'. This class allows 
 * you to create and edit new value
 * during the runtime of the script. 
 * 
 * Usage:
 * ```ts
 * props.setNumber("hp", 100);
 * const hp = props.getNumber("hp"); // 100
 * ```
 */
export class EntityProperties {
    private data: Record<string, any>;

    /**
     * Create an EntityProperties wrapper.
     *
     * @param data - Optional initial properties object. A shallow reference is kept.
     */
    public constructor(data?: Record<string, any>) {
        this.data = data || {};
    }

    /**
     * Get the raw underlying properties object.
     *
     * Use this to serialize properties back to the host or perform bulk operations.
     *
     * @returns The raw Record<string, any> used to store properties.
     *
     * @example
     * const raw = props.getRawProperties();
     */
    public getRawProperties(): Record<string, any> {
        return this.data;
    }

    /**
     * Set a string property.
     *
     * @param key - Property key.
     * @param value - String value to set.
     *
     * @example
     * props.setString("tag", "friendly");
     */
    public setString(key: string, value: string): void {
        this.data[key] = value;
    }

    /**
     * Set a numeric property.
     *
     * @param key - Property key.
     * @param value - Number value to set.
     *
     * @example
     * props.setNumber("speed", 4.2);
     */
    public setNumber(key: string, value: number): void {
        this.data[key] = value;
    }

    /**
     * Set a boolean property.
     *
     * @param key - Property key.
     * @param value - Boolean value to set.
     *
     * @example
     * props.setBool("isActive", true);
     */
    public setBool(key: string, value: boolean): void {
        this.data[key] = value;
    }

    /**
     * Get a string property.
     *
     * If the property is missing or falsy, an empty string ("") is returned.
     *
     * @param key - Property key.
     * @returns The string value or "" when missing.
     *
     * @example
     * const name = props.getString("name");
     */
    public getString(key: string): string {
        return this.data[key] || "";
    }

    /**
     * Get a numeric property.
     *
     * If the property is missing or falsy, 0 is returned.
     *
     * @param key - Property key.
     * @returns The number value or 0 when missing.
     *
     * @example
     * const speed = props.getNumber("speed");
     */
    public getNumber(key: string): number {
        return this.data[key] || 0;
    }

    /**
     * Get a boolean property.
     *
     * If the property is missing or falsy, false is returned.
     *
     * @param key - Property key.
     * @returns The boolean value or false when missing.
     *
     * @example
     * const active = props.getBool("isActive");
     */
    public getBool(key: string): boolean {
        return this.data[key] || false;
    }

    /**
     * Checks if the entity has a specific property as per a key. 
     * 
     * If the property exists, it will return true. If not, it will return false
     * @param key - Property key.
     * @returns - True is the value exists, false if not
     * 
     * @example
     * ```ts
     * if props.hasProperty("speed") {
     *      let speed = props.getNumber("speed");
     *      speed = speed+10;
     * } else {
     *      console.log("No value as speed exists");
     * }
     * ```
     */
    public hasProperty(key: string): boolean {
        return key in this.data;
    }
}

export class Entity {
    public label: string;
    public transform: Transform;
    public properties: EntityProperties;

    /**
     * Creates a new instance of entity
     * @param entityData - The entty data, typically parsed as an argument in the load or update functions
     */
    constructor(label: string, properties: EntityData, transform: TransformData) {
        this.label = label;
        this.transform = createTransformFromData(transform);
        this.properties = new EntityProperties(properties);
    }

    /**
     * Moves the player forward on the Z axis
     * @param distance - The distance (as a number) it moves forward by
     */
    moveForward(distance: number): void {
        const movement = new Vector3(0, 0, -distance);
        this.transform.translate(movement);
    }

    /**
     * Moves the player back on the Z axis
     * @param distance - The distance (as a number) it moves back by
     */
    moveBack(distance: number): void {
        const movement = new Vector3(0, 0, distance);
        this.transform.translate(movement);
    }

    /**
     * Moves the player left on the X axis
     * @param distance - The distance (as a number) it moves left by
     */
    moveLeft(distance: number): void {
        const movement = new Vector3(-distance, 0, 0);
        this.transform.translate(movement);
    }

    /**
     * Moves the player right on the X axis
     * @param distance - The distance (as a number) it moves right by
     */
    moveRight(distance: number): void {
        const movement = new Vector3(distance, 0, 0);
        this.transform.translate(movement);
    }

    /**
     * Moves the player up on the Y axis
     * @param distance - The distance (as a number) it moves up by
     */
    moveUp(distance: number): void {
        const movement = new Vector3(0, distance, 0);
        this.transform.translate(movement);
    }

    /**
     * Moves the player down on the Y axis
     * @param distance - The distance (as a number) it moves down by
     */
    moveDown(distance: number): void {
        const movement = new Vector3(0, -distance, 0);
        this.transform.translate(movement);
    }
}


/**
 * Helper function that creates a new {@link Transform} from 
 * a {@link TransformData}
 * @param data - The raw transformable ({@link TransformData})data
 * @returns - An instance of a {@link Transform}
 */
function createTransformFromData(data: TransformData): Transform {
    const position = Vector3.fromArray(data.position);
    const rotation = Quaternion.fromArray(data.rotation);
    const scale = Vector3.fromArray(data.scale);
    return new Transform(position, rotation, scale);
}

/**
 * Camera class for controlling and manipulating cameras in the scene.
 * Provides functionality for camera movement, switching, and property manipulation.
 */
export class Camera {
    public label: string;

    public eye: Vector3;
    public target: Vector3;
    public up: Vector3;
    public aspect: number;
    public fov: number;
    public near: number;
    public far: number;
    public yaw: number;
    public pitch: number;
    public speed: number;
    public sensitivity: number;
    public camera_type: string;


    /**
     * Create a new Camera instance.
     * 
     * @param data - Optional camera data to initialize from
     */
    constructor(label: string, data: CameraData) {
        this.eye = Vector3.fromArray(data.eye);
        this.target = Vector3.fromArray(data.target);
        this.up = Vector3.fromArray(data.up);
        this.aspect = data.aspect;
        this.fov = data.fov;
        this.near = data.near;
        this.far = data.far;
        this.yaw = data.yaw;
        this.pitch = data.pitch;
        this.speed = data.speed;
        this.sensitivity = data.sensitivity;
        this.camera_type = data.camera_type;
        this.label = label;
    }

    /**
     * Track mouse movement for camera look controls.
     * 
     * @param delta - Mouse delta X and Y
     * 
     * @example
     * ```ts
     * let delta = entity.getMouseDelta();
     * camera.track
     * ```
     */
    trackMouseDelta(delta: [number, number]): void {
        this.yaw += delta[0] * this.sensitivity;
        this.pitch += delta[1] * this.sensitivity;
        
        this.pitch = dbMath.clamp(this.pitch, dbMath.degreesToRadians(-89.0), dbMath.degreesToRadians(89.0));

        
        let direction = new Vector3(
            Math.cos(this.yaw) * Math.cos(this.pitch),
            Math.sin(this.pitch),
            Math.sin(this.yaw) * Math.cos(this.pitch),
        );
        this.target = this.eye.add(direction);
    }
}

export class Light {

}

/**
 * A raw format for storing the transform data, typically used as a
 * FFI by the engine
 */
export interface TransformData {
    position: [number, number, number];
    rotation: [number, number, number, number]; // quaternion [x, y, z, w]
    scale: [number, number, number];
}

/**
 * A raw format for storing the custom entity properties from the engines
 * raw FFI
 */
export interface EntityData {
    custom_properties: Record<string, any>;
}

/**
 * A raw format for storing the input data
 */
export interface InputData {
    mouse_pos: [number, number];
    pressed_keys: string[];
    mouse_delta: [number, number] | null;
    is_cursor_locked: boolean;
}

/**
 * A raw format for storing the camera data
 */
export interface CameraData {
    eye: [number, number, number];
    target: [number, number, number];
    up: [number, number, number];
    aspect: number;
    fov: number;
    near: number;
    far: number;
    yaw: number;
    pitch: number;
    speed: number;
    sensitivity: number;
    camera_type: string;
}

/**
 * A raw format for storing the light data
 */
export interface LightData {

}

/**
 * A raw format for storing the scene data
 */
export interface RawSceneData {
    entities: [{
        label: string,
        properties: EntityData, 
        transform: TransformData
    }];
    cameras: [{
        label: string,
        data: CameraData
    }];
    lights: [{
        label: string,
        data: LightData
    }];
    input: InputData;
}

/**
 * A wrapper class aimed to aid with input data
 */
export class Input {
    public inputData?: InputData

    // constructor(data: InputData) {
    //     this.inputData = data;
    // }

    // empty because global variable
    constructor() {

    }

    /**
     * Checks if a key is pressed
     * @param key - The specific keycode
     * @returns - True is pressed, false if not
     * 
     * @example
     * ```ts
     * if entity.isKeyPressed(Keys::KeyW) {
     *      console.log("The W key is pressed");
     * }
     * ```
     */
    isKeyPressed(key: KeyCode): boolean {
        if (!this.inputData) return false;
        return this.inputData.pressed_keys.indexOf(key) !== -1
    }

    /**
     * Fetches the mouse position
     * @returns - The x,y position of the mouse
     */
    getMousePosition(): [number, number] {
        return this.inputData?.mouse_pos || [0, 0];
    }

    /**
     * Fetches the change in the mouse position from the center (as it gets reset each frame)
     * @returns - The dx,dy position of the mouse
     */
    getMouseDelta(): [number, number] {
        return this.inputData?.mouse_delta || [0.0, 0.0];
    }
}

/**
 * The class containing all the different entities in this scene. 
 */
export class Scene {
    // ensure none of these are public
    current_entity?: string;
    entities?: Entity[];
    cameras?: Camera[];
    lights?: Light[];

    // Sets everything to default
    constructor() {
        this.cameras = [];
        this.entities = [];
        this.lights = [];
    }

    /**
     * Returns a **reference** to the camera in the scene
     * @param label - The label of the camera as set by you from the editor
     */
    public getCamera(label: string): Camera | undefined {
        return this.cameras?.find(c => c.label === label);
    }

    /**
     * Returns a **reference** to the light in the scene
     * @param label - The label of the light as set by you from the editor
     */
    public getLight(label: string): Light | undefined {
        return this.lights?.find(l => (l as any).label === label);
    }

    /**
     * Fetches an entity as per its label. Returns the actual Entity instance (mutable reference).
     * @param label - The label of the entity
     */
    public getEntity(label: string): Entity | undefined {
        return this.entities?.find(e => e.label === label);
    }

    /**
     * Fetches the current entity this script is attached to (returns mutable reference).
     */
    public getCurrentEntity(): Entity | undefined {
        if (!this.current_entity) {
            console.error("Unable to get entity: Have you added dropbear.start(s) yet?");
            return undefined;
        }
        const ent = this.getEntity(this.current_entity);
        if (!ent) {
            console.error(`Unable to get entity: no entity with label "${this.current_entity}" found`);
        }
        return ent;
    }
}

/**
 * Starts the specific function by filling the scene data. 
 * @param data 
 */
export function start(data: RawSceneData) {
    data.cameras.forEach(camera => {
        scene.cameras?.push(new Camera(camera.label, camera.data))
    });
    data.entities.forEach(entity => {
        scene.entities?.push(new Entity(entity.label, entity.properties, entity.transform))
    });
    data.lights.forEach(light => {
        scene.lights?.push(light)
    });
    input.inputData = data.input;
}

/**
 * Ends the scripting function by returning a Partial RawSceneData for the
 * rust client to take. 
 */
export function end(): Partial<RawSceneData> {
    const out: Partial<RawSceneData> = {};

    if (scene.entities && scene.entities.length) {
        out.entities = scene.entities.map(e => {
            return {
                label: e.label,
                properties: {
                    custom_properties: e.properties.getRawProperties()
                } as EntityData,
                transform: {
                    position: e.transform.position.as_array(),
                    rotation: e.transform.rotation.as_array(),
                    scale: e.transform.scale.as_array()
                } as TransformData
            };
        }) as any;
    }

    if (scene.cameras && scene.cameras.length) {
        out.cameras = scene.cameras.map(c => {
            return {
                label: c.label,
                data: {
                    eye: c.eye.as_array(),
                    target: c.target.as_array(),
                    up: c.up.as_array(),
                    aspect: c.aspect,
                    fov: c.fov,
                    near: c.near,
                    far: c.far,
                    yaw: c.yaw,
                    pitch: c.pitch,
                    speed: c.speed,
                    sensitivity: c.sensitivity,
                    camera_type: c.camera_type
                } as CameraData
            };
        }) as any;
    }

    if (scene.lights && scene.lights.length) {
        out.lights = scene.lights.map(l => {
            return {
                label: (l as any).label || "",
                data: {} as LightData
            };
        }) as any;
    }

    if (input.inputData) {
        out.input = input.inputData;
    }

    return out;
}

// global variables
/**
 * A global variable that contains all the information about the scene. 
 * 
 * To use the variable, you need to run {@link start()} at the start of 
 * the function and return {@link end()} at the end to send to the engine.  
 * 
 * @example
 * ```ts
 * export function onUpdate(s, dt: number) {
 *    dropbear.start(s);
 *    console.log("I'm being updated!");
 *    return dropbear.end();
 * }
 * ```
 */
export const scene = new Scene();
/**
 * A global variable that contains all the information about the inputs
 * such as the keyboard and the mouse.
 * 
 * To use the variable, you need to run {@link start()} at the start of 
 * the function and return {@link end()} at the end to send to the engine.  
 * 
 * @example
 * ```ts
 * export function onUpdate(s, dt: number) {
 *    dropbear.start(s);
 *    console.log("I'm being updated!");
 *    return dropbear.end();
 * }
 */
export const input = new Input();