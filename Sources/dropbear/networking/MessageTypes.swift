import Foundation

/// Request types that can be sent to the Rust engine
public enum EngineRequest: Codable, Sendable {
    // Entity operations
    case getEntityInfo(id: UInt64)
    case getEntityTransform(id: UInt64)
    case setEntityTransform(id: UInt64, transform: TransformData)
    case getEntityComponent(id: UInt64, componentType: String)
    
    // Scene operations
    case getSceneInfo
    case getAllEntities
    case getEntitiesByLabel(label: String)
    case createEntity(label: String?)
    case deleteEntity(id: UInt64)
    
    // Input operations
    case isKeyPressed(key: String)
    case getMousePosition
    case getMouseDelta
    
    // System operations
    case getDeltaTime
    case ping
    
    // Coding keys for JSON serialization
    private enum CodingKeys: String, CodingKey {
        case type
        case id
        case transform
        case componentType = "component_type"
        case label
        case key
    }
    
    // Custom encoding
    public func encode(to encoder: Encoder) throws {
        var container = encoder.container(keyedBy: CodingKeys.self)
        
        switch self {
        case .getEntityInfo(let id):
            try container.encode("GetEntityInfo", forKey: .type)
            try container.encode(id, forKey: .id)
        case .getEntityTransform(let id):
            try container.encode("GetEntityTransform", forKey: .type)
            try container.encode(id, forKey: .id)
        case .setEntityTransform(let id, let transform):
            try container.encode("SetEntityTransform", forKey: .type)
            try container.encode(id, forKey: .id)
            try container.encode(transform, forKey: .transform)
        case .getEntityComponent(let id, let componentType):
            try container.encode("GetEntityComponent", forKey: .type)
            try container.encode(id, forKey: .id)
            try container.encode(componentType, forKey: .componentType)
        case .getSceneInfo:
            try container.encode("GetSceneInfo", forKey: .type)
        case .getAllEntities:
            try container.encode("GetAllEntities", forKey: .type)
        case .getEntitiesByLabel(let label):
            try container.encode("GetEntitiesByLabel", forKey: .type)
            try container.encode(label, forKey: .label)
        case .createEntity(let label):
            try container.encode("CreateEntity", forKey: .type)
            if let label = label {
                try container.encode(label, forKey: .label)
            }
        case .deleteEntity(let id):
            try container.encode("DeleteEntity", forKey: .type)
            try container.encode(id, forKey: .id)
        case .isKeyPressed(let key):
            try container.encode("IsKeyPressed", forKey: .type)
            try container.encode(key, forKey: .key)
        case .getMousePosition:
            try container.encode("GetMousePosition", forKey: .type)
        case .getMouseDelta:
            try container.encode("GetMouseDelta", forKey: .type)
        case .getDeltaTime:
            try container.encode("GetDeltaTime", forKey: .type)
        case .ping:
            try container.encode("Ping", forKey: .type)
        }
    }
    
    // Custom decoding (if needed for responses)
    public init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        let type = try container.decode(String.self, forKey: .type)
        
        switch type {
        case "GetEntityInfo":
            let id = try container.decode(UInt64.self, forKey: .id)
            self = .getEntityInfo(id: id)
        case "GetEntityTransform":
            let id = try container.decode(UInt64.self, forKey: .id)
            self = .getEntityTransform(id: id)
        case "SetEntityTransform":
            let id = try container.decode(UInt64.self, forKey: .id)
            let transform = try container.decode(TransformData.self, forKey: .transform)
            self = .setEntityTransform(id: id, transform: transform)
        case "GetEntityComponent":
            let id = try container.decode(UInt64.self, forKey: .id)
            let componentType = try container.decode(String.self, forKey: .componentType)
            self = .getEntityComponent(id: id, componentType: componentType)
        case "GetSceneInfo":
            self = .getSceneInfo
        case "GetAllEntities":
            self = .getAllEntities
        case "GetEntitiesByLabel":
            let label = try container.decode(String.self, forKey: .label)
            self = .getEntitiesByLabel(label: label)
        case "CreateEntity":
            let label = try container.decodeIfPresent(String.self, forKey: .label)
            self = .createEntity(label: label)
        case "DeleteEntity":
            let id = try container.decode(UInt64.self, forKey: .id)
            self = .deleteEntity(id: id)
        case "IsKeyPressed":
            let key = try container.decode(String.self, forKey: .key)
            self = .isKeyPressed(key: key)
        case "GetMousePosition":
            self = .getMousePosition
        case "GetMouseDelta":
            self = .getMouseDelta
        case "GetDeltaTime":
            self = .getDeltaTime
        case "Ping":
            self = .ping
        default:
            throw DecodingError.dataCorrupted(
                DecodingError.Context(codingPath: decoder.codingPath, debugDescription: "Unknown request type: \(type)")
            )
        }
    }
}

/// Response types received from the Rust engine
public enum EngineResponse: Codable, Sendable {
    // Entity responses
    case entityInfo(id: UInt64, label: String?, transform: TransformData?, components: [String])
    case entityTransform(TransformData)
    case entityComponent(componentType: String, data: [String: String])
    
    // Scene responses
    case sceneInfo(entityCount: Int, entities: [EntityInfo])
    case entityList([EntityInfo])
    case entityCreated(id: UInt64)
    
    // Input responses
    case keyPressed(Bool)
    case mousePosition(Vector2)
    case mouseDelta(Vector2)
    
    // System responses
    case deltaTime(Float)
    case pong
    
    // Error responses
    case error(message: String)
    case success
    
    // Coding keys for JSON serialization
    private enum CodingKeys: String, CodingKey {
        case type = "type"
        case id
        case label
        case transform
        case components
        case componentType = "component_type"
        case data
        case entityCount = "entity_count"
        case entities
        case message
    }
    
    // Custom decoding
    public init(from decoder: Decoder) throws {
        // Try to decode as different response types
        if let container = try? decoder.container(keyedBy: CodingKeys.self) {
            // Handle structured responses with type field
            if let type = try? container.decode(String.self, forKey: .type) {
                switch type {
                case "EntityInfo":
                    let id = try container.decode(UInt64.self, forKey: .id)
                    let label = try container.decodeIfPresent(String.self, forKey: .label)
                    let transform = try container.decodeIfPresent(TransformData.self, forKey: .transform)
                    let components = try container.decode([String].self, forKey: .components)
                    self = .entityInfo(id: id, label: label, transform: transform, components: components)
                    return
                case "Error":
                    let message = try container.decode(String.self, forKey: .message)
                    self = .error(message: message)
                    return
                default:
                    break
                }
            }
        }
        
        // Try to decode as direct value types
        if let transform = try? TransformData(from: decoder) {
            self = .entityTransform(transform)
            return
        }
        
        if let bool = try? Bool(from: decoder) {
            self = .keyPressed(bool)
            return
        }
        
        if let vector = try? Vector2(from: decoder) {
            self = .mousePosition(vector)
            return
        }
        
        if let float = try? Float(from: decoder) {
            self = .deltaTime(float)
            return
        }
        
        if let string = try? String(from: decoder), string == "Pong" {
            self = .pong
            return
        }
        
        if let string = try? String(from: decoder), string == "Success" {
            self = .success
            return
        }
        
        // Fallback to error
        self = .error(message: "Unknown response format")
    }
    
    public func encode(to encoder: Encoder) throws {
        var container = encoder.container(keyedBy: CodingKeys.self)
        
        switch self {
        case .entityInfo(let id, let label, let transform, let components):
            try container.encode("EntityInfo", forKey: .type)
            try container.encode(id, forKey: .id)
            try container.encodeIfPresent(label, forKey: .label)
            try container.encodeIfPresent(transform, forKey: .transform)
            try container.encode(components, forKey: .components)
        case .entityTransform(let transform):
            try transform.encode(to: encoder)
        case .entityComponent(let componentType, let data):
            try container.encode("EntityComponent", forKey: .type)
            try container.encode(componentType, forKey: .componentType)
            try container.encode(data, forKey: .data)
        case .sceneInfo(let entityCount, let entities):
            try container.encode("SceneInfo", forKey: .type)
            try container.encode(entityCount, forKey: .entityCount)
            try container.encode(entities, forKey: .entities)
        case .entityList(let entities):
            try container.encode("EntityList", forKey: .type)
            try container.encode(entities, forKey: .entities)
        case .entityCreated(let id):
            try container.encode("EntityCreated", forKey: .type)
            try container.encode(id, forKey: .id)
        case .keyPressed(let pressed):
            try pressed.encode(to: encoder)
        case .mousePosition(let position):
            try position.encode(to: encoder)
        case .mouseDelta(let delta):
            try delta.encode(to: encoder)
        case .deltaTime(let dt):
            try dt.encode(to: encoder)
        case .pong:
            try "Pong".encode(to: encoder)
        case .error(let message):
            try container.encode("Error", forKey: .type)
            try container.encode(message, forKey: .message)
        case .success:
            try "Success".encode(to: encoder)
        }
    }
}