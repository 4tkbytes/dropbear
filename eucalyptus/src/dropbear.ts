// Modules for the dropbear-engine scripting component
// Made by 4tkbytes
// EDIT THIS IF YOU WISH, RECOMMENDED TO NOT TOUCH IT

type InputStateT = any;

export class Transform {
    position: Vector3;
    rotation: Quaternion;
    scale: Vector3;

    public constructor(position?: Vector3, rotation?: Quaternion, scale?: Vector3) {
        this.position = position || Vector3.zero();
        this.rotation = rotation || Quaternion.identity();
        this.scale = scale || Vector3.one();
    }

    public translate(movement: Vector3) {
        // Simple direct math - no host functions needed
        this.position.x += movement.x;
        this.position.y += movement.y;
        this.position.z += movement.z;
    }

    public rotateX(angle: number) {
        // Create rotation quaternion and multiply
        const rotQuat = Quaternion.fromAxisAngle(new Vector3(1, 0, 0), angle);
        this.rotation = Quaternion.multiply(this.rotation, rotQuat);
    }

    public rotateY(angle: number) {
        const rotQuat = Quaternion.fromAxisAngle(new Vector3(0, 1, 0), angle);
        this.rotation = Quaternion.multiply(this.rotation, rotQuat);
    }

    public rotateZ(angle: number) {
        const rotQuat = Quaternion.fromAxisAngle(new Vector3(0, 0, 1), angle);
        this.rotation = Quaternion.multiply(this.rotation, rotQuat);
    }

    public scaleUniform(scale: number) {
        this.scale.x *= scale;
        this.scale.y *= scale;
        this.scale.z *= scale;
    }

    public scaleIndividual(scale: [number, number, number]) {
        this.scale.x *= scale[0];
        this.scale.y *= scale[1];
        this.scale.z *= scale[2];
    }
}

/**
 * A Vector of 3 components: an X, Y and Z. 
 */
export class Vector3 {
    public x: number;
    public y: number;
    public z: number;

    public constructor(x: number, y: number, z: number) {
        this.x = x;
        this.y = y;
        this.z = z;
    }

    public as_array(): [number, number, number] {
        return [this.x, this.y, this.z];
    }

    public static zero(): Vector3 {
        return new Vector3(0.0, 0.0, 0.0);
    }

    public static one(): Vector3 {
        return new Vector3(1.0, 1.0, 1.0);
    }

    public static fromArray(arr: [number, number, number]): Vector3 {
        return new Vector3(arr[0], arr[1], arr[2]);
    }

    // Useful vector operations
    public add(other: Vector3): Vector3 {
        return new Vector3(this.x + other.x, this.y + other.y, this.z + other.z);
    }

    public subtract(other: Vector3): Vector3 {
        return new Vector3(this.x - other.x, this.y - other.y, this.z - other.z);
    }

    public multiply(scalar: number): Vector3 {
        return new Vector3(this.x * scalar, this.y * scalar, this.z * scalar);
    }

    public length(): number {
        return Math.sqrt(this.x * this.x + this.y * this.y + this.z * this.z);
    }

    public normalize(): Vector3 {
        const len = this.length();
        if (len === 0) return Vector3.zero();
        return new Vector3(this.x / len, this.y / len, this.z / len);
    }
}

export class Quaternion {
    public x: number;
    public y: number;
    public z: number;
    public w: number;

    public constructor(x: number = 0, y: number = 0, z: number = 0, w: number = 1) {
        this.x = x;
        this.y = y;
        this.z = z;
        this.w = w;
    }

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

    public static multiply(a: Quaternion, b: Quaternion): Quaternion {
        return new Quaternion(
            a.w * b.x + a.x * b.w + a.y * b.z - a.z * b.y,
            a.w * b.y - a.x * b.z + a.y * b.w + a.z * b.x,
            a.w * b.z + a.x * b.y - a.y * b.x + a.z * b.w,
            a.w * b.w - a.x * b.x - a.y * b.y - a.z * b.z
        );
    }

    public static fromArray(arr: [number, number, number, number]): Quaternion {
        return new Quaternion(arr[0], arr[1], arr[2], arr[3]);
    }

    public static identity(): Quaternion {
        return new Quaternion(0.0, 0.0, 0.0, 1.0);
    }

    public as_array(): [number, number, number, number] {
        return [this.x, this.y, this.z, this.w];
    }
}

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

export class EntityProperties {
    private data: Record<string, any>;

    public constructor(data?: Record<string, any>) {
        this.data = data || {};
    }

    public getRawProperties(): Record<string, any> {
        return this.data;
    }

    public setString(key: string, value: string): void {
        this.data[key] = value;
    }

    public setNumber(key: string, value: number): void {
        this.data[key] = value;
    }

    public setBool(key: string, value: boolean): void {
        this.data[key] = value;
    }

    public getString(key: string): string {
        return this.data[key] || "";
    }

    public getNumber(key: string): number {
        return this.data[key] || 0;
    }

    public getBool(key: string): boolean {
        return this.data[key] || false;
    }

    public hasProperty(key: string): boolean {
        return key in this.data;
    }
}

export class Entity {
    public transform: Transform;
    public properties: EntityProperties;
    private inputData: InputData | null = null;

    constructor(entityData?: ScriptEntityData) {
        if (entityData) {
            this.transform = createTransformFromData(entityData.transform);
            this.properties = new EntityProperties(entityData.entity.custom_properties);
            this.inputData = entityData.input;
        } else {
            this.transform = new Transform();
            this.properties = new EntityProperties({});
        }
    }

    // Input helpers
    isKeyPressed(key: KeyCode): boolean {
        if (!this.inputData) return false;
        return this.inputData.pressed_keys.indexOf(key) !== -1
    }

    getMousePosition(): [number, number] {
        return this.inputData?.mouse_pos || [0, 0];
    }

    getMouseDelta(): [number, number] | null {
        return this.inputData?.mouse_delta || null;
    }

    // Movement helpers
    moveForward(distance: number): void {
        const movement = new Vector3(0, 0, -distance);
        this.transform.translate(movement);
    }

    moveRight(distance: number): void {
        const movement = new Vector3(distance, 0, 0);
        this.transform.translate(movement);
    }

    moveUp(distance: number): void {
        const movement = new Vector3(0, distance, 0);
        this.transform.translate(movement);
    }

    // Convert back to data format for Rust
    toEntityData(): Partial<ScriptEntityData> {
        return {
            transform: {
                position: this.transform.position.as_array(),
                rotation: this.transform.rotation.as_array(),
                scale: this.transform.scale.as_array()
            },
            entity: {
                custom_properties: this.properties.getRawProperties()
            }
        };
    }
}

// Type definitions
export interface TransformData {
    position: [number, number, number];
    rotation: [number, number, number, number]; // quaternion [x, y, z, w]
    scale: [number, number, number];
}

export interface EntityData {
    custom_properties: Record<string, any>;
}

export interface InputData {
    mouse_pos: [number, number];
    pressed_keys: string[];
    mouse_delta: [number, number] | null;
    is_cursor_locked: boolean;
}

export interface ScriptEntityData {
    transform: TransformData;
    entity: EntityData;
    input: InputData;
}

export function createTransformFromData(data: TransformData): Transform {
    const position = Vector3.fromArray(data.position);
    const rotation = Quaternion.fromArray(data.rotation);
    const scale = Vector3.fromArray(data.scale);
    return new Transform(position, rotation, scale);
}

// Global exports
globalThis.Transform = Transform;
globalThis.Vector3 = Vector3;
globalThis.Quaternion = Quaternion;
globalThis.Keys = Keys;
globalThis.EntityProperties = EntityProperties;
globalThis.Entity = Entity;