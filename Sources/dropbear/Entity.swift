import Foundation

/// Represents a game entity with transform and component data
/// Provides methods to manipulate entity properties through socket communication
public class Entity {
    /// The unique identifier for this entity
    public let id: UInt64
    
    /// Optional label for the entity
    public private(set) var label: String?
    
    /// Cached transform data
    private var cachedTransform: TransformData?
    
    /// Cached components list
    private var cachedComponents: [String] = []
    
    /// Reference to the socket client for communication
    private weak var socketClient: SocketClient?
    
    /// Initialize entity with ID and socket client
    /// - Parameters:
    ///   - id: Unique entity identifier
    ///   - socketClient: Socket client for communication with Rust engine
    ///   - label: Optional entity label
    public init(id: UInt64, socketClient: SocketClient, label: String? = nil) {
        self.id = id
        self.socketClient = socketClient
        self.label = label
    }
    
    // MARK: - Entity Information
    
    /// Refresh entity information from the engine
    public func refreshInfo() async throws {
        guard let client = socketClient else {
            throw EntityError.noSocketClient
        }
        
        let request = EngineRequest.getEntityInfo(id: id)
        let response: EngineResponse = try await client.sendEngineRequest(request)
        
        switch response {
        case .entityInfo(let entityId, let entityLabel, let transform, let components):
            guard entityId == id else {
                throw EntityError.invalidResponse
            }
            
            self.label = entityLabel
            self.cachedTransform = transform
            self.cachedComponents = components
            
        case .error(let message):
            throw EntityError.engineError(message)
            
        default:
            throw EntityError.invalidResponse
        }
    }
    
    /// Get list of component types attached to this entity
    public func getComponents() async throws -> [String] {
        // Return cached if available
        if !cachedComponents.isEmpty {
            return cachedComponents
        }
        
        // Otherwise refresh from engine
        try await refreshInfo()
        return cachedComponents
    }
    
    /// Check if entity has a specific component type
    public func hasComponent(_ componentType: String) async throws -> Bool {
        let components = try await getComponents()
        return components.contains(componentType)
    }
    
    // MARK: - Transform Operations
    
    /// Get the entity's transform
    public func getTransform() async throws -> TransformData {
        guard let client = socketClient else {
            throw EntityError.noSocketClient
        }
        
        let request = EngineRequest.getEntityTransform(id: id)
        let response: EngineResponse = try await client.sendEngineRequest(request)
        
        switch response {
        case .entityTransform(let transform):
            cachedTransform = transform
            return transform
            
        case .error(let message):
            throw EntityError.engineError(message)
            
        default:
            throw EntityError.invalidResponse
        }
    }
    
    /// Set the entity's transform
    public func setTransform(_ transform: TransformData) async throws {
        guard let client = socketClient else {
            throw EntityError.noSocketClient
        }
        
        let request = EngineRequest.setEntityTransform(id: id, transform: transform)
        let response: EngineResponse = try await client.sendEngineRequest(request)
        
        switch response {
        case .success:
            cachedTransform = transform
            
        case .error(let message):
            throw EntityError.engineError(message)
            
        default:
            throw EntityError.invalidResponse
        }
    }
    
    // MARK: - Convenience Transform Methods
    
    /// Get entity position
    public func getPosition() async throws -> SimpleVector3 {
        let transform = try await getTransform()
        return transform.position
    }
    
    /// Set entity position
    public func setPosition(_ position: SimpleVector3) async throws {
        var transform: TransformData
        if let cached = cachedTransform {
            transform = cached
        } else {
            transform = try await getTransform()
        }
        transform.position = position
        try await setTransform(transform)
    }
    
    /// Get entity rotation (in degrees)
    public func getRotation() async throws -> SimpleVector3 {
        let transform = try await getTransform()
        return transform.rotation
    }
    
    /// Set entity rotation (in degrees)
    public func setRotation(_ rotation: SimpleVector3) async throws {
        var transform: TransformData
        if let cached = cachedTransform {
            transform = cached
        } else {
            transform = try await getTransform()
        }
        transform.rotation = rotation
        try await setTransform(transform)
    }
    
    /// Get entity scale
    public func getScale() async throws -> SimpleVector3 {
        let transform = try await getTransform()
        return transform.scale
    }
    
    /// Set entity scale
    public func setScale(_ scale: SimpleVector3) async throws {
        var transform: TransformData
        if let cached = cachedTransform {
            transform = cached
        } else {
            transform = try await getTransform()
        }
        transform.scale = scale
        try await setTransform(transform)
    }
    
    // MARK: - Transform Manipulation
    
    /// Move entity by offset
    public func translate(by offset: SimpleVector3) async throws {
        let currentPosition = try await getPosition()
        try await setPosition(currentPosition + offset)
    }
    
    /// Rotate entity by offset (in degrees)
    public func rotate(by offset: SimpleVector3) async throws {
        let currentRotation = try await getRotation()
        try await setRotation(currentRotation + offset)
    }
    
    /// Scale entity by factor
    public func scale(by factor: SimpleVector3) async throws {
        let currentScale = try await getScale()
        let newScale = SimpleVector3(
            currentScale.x * factor.x,
            currentScale.y * factor.y,
            currentScale.z * factor.z
        )
        try await setScale(newScale)
    }
    
    /// Scale entity uniformly
    public func scale(by factor: Float) async throws {
        try await scale(by: SimpleVector3(factor, factor, factor))
    }
    
    // MARK: - Component Access
    
    /// Get component data (generic method)
    public func getComponent<T: Codable>(_ componentType: String, as type: T.Type) async throws -> T {
        guard let client = socketClient else {
            throw EntityError.noSocketClient
        }
        
        let request = EngineRequest.getEntityComponent(id: id, componentType: componentType)
        let response: EngineResponse = try await client.sendEngineRequest(request)
        
        switch response {
        case .entityComponent(let returnedType, let data):
            guard returnedType == componentType else {
                throw EntityError.invalidResponse
            }
            
            // Convert AnyCodable data to requested type
            let jsonData = try JSONSerialization.data(withJSONObject: data)
            let component = try JSONDecoder().decode(T.self, from: jsonData)
            return component
            
        case .error(let message):
            throw EntityError.engineError(message)
            
        default:
            throw EntityError.invalidResponse
        }
    }
}

/// Errors that can occur with Entity operations
public enum EntityError: Error, LocalizedError {
    case noSocketClient
    case invalidResponse
    case engineError(String)
    case componentNotFound(String)
    case transformNotAvailable
    
    public var errorDescription: String? {
        switch self {
        case .noSocketClient:
            return "No socket client available for communication"
        case .invalidResponse:
            return "Invalid response from engine"
        case .engineError(let message):
            return "Engine error: \(message)"
        case .componentNotFound(let componentType):
            return "Component '\(componentType)' not found on entity"
        case .transformNotAvailable:
            return "Transform component not available"
        }
    }
}

// MARK: - Extensions

extension Entity: Equatable {
    public static func == (lhs: Entity, rhs: Entity) -> Bool {
        return lhs.id == rhs.id
    }
}

extension Entity: Hashable {
    public func hash(into hasher: inout Hasher) {
        hasher.combine(id)
    }
}

extension Entity: CustomStringConvertible {
    public var description: String {
        return "Entity(id: \(id))"
    }
}