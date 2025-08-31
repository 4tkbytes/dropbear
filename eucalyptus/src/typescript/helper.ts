// Modules for the dropbear-engine scripting component

type TransformT = any;
type Vec3T = any;
type QuatT = any;

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

const Vec3 = {
    create: (x = 0, y = 0, z = 0): Vec3T => hostFn("createVec3")(x, y, z),
    zero: (): Vec3T => hostFn("createVec3")(0, 0, 0),
    one: (): Vec3T => hostFn("createVec3")(1, 1, 1),
};

const Quaternion = {
    identity: (): QuatT => hostFn("createQuatIdentity")(),
    fromEuler: (x: number, y: number, z: number): QuatT => hostFn("createQuatFromEuler")(x, y, z),
};

// make global
export { Transform, Vec3, Quaternion };

globalThis.Transform = Transform;
globalThis.Vec3 = Vec3;
globalThis.Quaternion = Quaternion;