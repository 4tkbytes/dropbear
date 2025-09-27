import Foundation

/// 3D Transform representation matching the Rust Transform structure
public struct TransformData: Codable, Sendable {
    public var position: SimpleVector3
    public var rotation: SimpleVector3  // Euler angles in degrees
    public var scale: SimpleVector3
    
    public init(position: SimpleVector3 = SimpleVector3.zero, rotation: SimpleVector3 = SimpleVector3.zero, scale: SimpleVector3 = SimpleVector3.one) {
        self.position = position
        self.rotation = rotation
        self.scale = scale
    }
    
    /// Create identity transform
    public static var identity: TransformData {
        return TransformData(position: .zero, rotation: .zero, scale: .one)
    }
}

/// Simple 3D Vector representation for socket communication
public struct SimpleVector3: Codable, Sendable {
    public var x: Float
    public var y: Float
    public var z: Float
    
    public init(x: Float, y: Float, z: Float) {
        self.x = x
        self.y = y
        self.z = z
    }
    
    public init(_ x: Float, _ y: Float, _ z: Float) {
        self.x = x
        self.y = y
        self.z = z
    }
    
    // Common vectors
    public static let zero = SimpleVector3(0, 0, 0)
    public static let one = SimpleVector3(1, 1, 1)
    public static let up = SimpleVector3(0, 1, 0)
    public static let down = SimpleVector3(0, -1, 0)
    public static let left = SimpleVector3(-1, 0, 0)
    public static let right = SimpleVector3(1, 0, 0)
    public static let forward = SimpleVector3(0, 0, 1)
    public static let back = SimpleVector3(0, 0, -1)
    
    // Vector operations
    public static func + (lhs: SimpleVector3, rhs: SimpleVector3) -> SimpleVector3 {
        return SimpleVector3(lhs.x + rhs.x, lhs.y + rhs.y, lhs.z + rhs.z)
    }
    
    public static func - (lhs: SimpleVector3, rhs: SimpleVector3) -> SimpleVector3 {
        return SimpleVector3(lhs.x - rhs.x, lhs.y - rhs.y, lhs.z - rhs.z)
    }
    
    public static func * (lhs: SimpleVector3, scalar: Float) -> SimpleVector3 {
        return SimpleVector3(lhs.x * scalar, lhs.y * scalar, lhs.z * scalar)
    }
    
    public static func / (lhs: SimpleVector3, scalar: Float) -> SimpleVector3 {
        return SimpleVector3(lhs.x / scalar, lhs.y / scalar, lhs.z / scalar)
    }
    
    public var magnitude: Float {
        return sqrt(x * x + y * y + z * z)
    }
    
    public var normalized: SimpleVector3 {
        let mag = magnitude
        guard mag > 0 else { return SimpleVector3.zero }
        return self / mag
    }
    
    public func dot(_ other: SimpleVector3) -> Float {
        return x * other.x + y * other.y + z * other.z
    }
    
    public func cross(_ other: SimpleVector3) -> SimpleVector3 {
        return SimpleVector3(
            y * other.z - z * other.y,
            z * other.x - x * other.z,
            x * other.y - y * other.x
        )
    }
}

/// Simple 2D Vector representation for socket communication
public struct Vector2: Codable, Sendable {
    public var x: Float
    public var y: Float
    
    public init(x: Float, y: Float) {
        self.x = x
        self.y = y
    }
    
    public init(_ x: Float, _ y: Float) {
        self.x = x
        self.y = y
    }
    
    // Common vectors
    public static let zero = Vector2(0, 0)
    public static let one = Vector2(1, 1)
    public static let up = Vector2(0, 1)
    public static let down = Vector2(0, -1)
    public static let left = Vector2(-1, 0)
    public static let right = Vector2(1, 0)
    
    // Vector operations
    public static func + (lhs: Vector2, rhs: Vector2) -> Vector2 {
        return Vector2(lhs.x + rhs.x, lhs.y + rhs.y)
    }
    
    public static func - (lhs: Vector2, rhs: Vector2) -> Vector2 {
        return Vector2(lhs.x - rhs.x, lhs.y - rhs.y)
    }
    
    public static func * (lhs: Vector2, scalar: Float) -> Vector2 {
        return Vector2(lhs.x * scalar, lhs.y * scalar)
    }
    
    public static func / (lhs: Vector2, scalar: Float) -> Vector2 {
        return Vector2(lhs.x / scalar, lhs.y / scalar)
    }
    
    public var magnitude: Float {
        return sqrt(x * x + y * y)
    }
    
    public var normalized: Vector2 {
        let mag = magnitude
        guard mag > 0 else { return Vector2.zero }
        return self / mag
    }
    
    public func dot(_ other: Vector2) -> Float {
        return x * other.x + y * other.y
    }
}

/// Entity information from the engine
public struct EntityInfo: Codable, Sendable {
    public let id: UInt64
    public let label: String?
    public let transform: TransformData?
    public let components: [String]
    
    public init(id: UInt64, label: String? = nil, transform: TransformData? = nil, components: [String] = []) {
        self.id = id
        self.label = label
        self.transform = transform
        self.components = components
    }
}

// Extension to make vectors equatable and hashable
extension SimpleVector3: Equatable, Hashable {
    public static func == (lhs: SimpleVector3, rhs: SimpleVector3) -> Bool {
        return abs(lhs.x - rhs.x) < Float.ulpOfOne &&
               abs(lhs.y - rhs.y) < Float.ulpOfOne &&
               abs(lhs.z - rhs.z) < Float.ulpOfOne
    }
    
    public func hash(into hasher: inout Hasher) {
        hasher.combine(x)
        hasher.combine(y)
        hasher.combine(z)
    }
}

extension Vector2: Equatable, Hashable {
    public static func == (lhs: Vector2, rhs: Vector2) -> Bool {
        return abs(lhs.x - rhs.x) < Float.ulpOfOne &&
               abs(lhs.y - rhs.y) < Float.ulpOfOne
    }
    
    public func hash(into hasher: inout Hasher) {
        hasher.combine(x)
        hasher.combine(y)
    }
}

extension TransformData: Equatable {
    public static func == (lhs: TransformData, rhs: TransformData) -> Bool {
        return lhs.position == rhs.position &&
               lhs.rotation == rhs.rotation &&
               lhs.scale == rhs.scale
    }
}