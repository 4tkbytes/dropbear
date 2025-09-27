import Foundation

/// Scene management for game objects and entities
/// Provides methods to create, query, and manipulate entities within the scene
public class Scene {
    /// Reference to the socket client for communication
    private let socketClient: SocketClient
    
    /// Cached entities in the scene
    private var cachedEntities: [UInt64: Entity] = [:]
    
    /// Last known entity count
    private var lastEntityCount: Int = 0
    
    /// Initialize scene with socket client
    /// - Parameter socketClient: Socket client for communication with Rust engine
    public init(socketClient: SocketClient) {
        self.socketClient = socketClient
    }
    
    // MARK: - Scene Information
    
    /// Get information about the current scene
    public func getSceneInfo() async throws -> SceneInfo {
        let request = EngineRequest.getSceneInfo
        let response: EngineResponse = try await socketClient.sendEngineRequest(request)
        
        switch response {
        case .sceneInfo(let entityCount, let entities):
            lastEntityCount = entityCount
            
            // Update cached entities
            cachedEntities.removeAll()
            for entityInfo in entities {
                let entity = Entity(id: entityInfo.id, socketClient: socketClient, label: entityInfo.label)
                cachedEntities[entityInfo.id] = entity
            }
            
            return SceneInfo(entityCount: entityCount, entities: entities)
            
        case .error(let message):
            throw SceneError.engineError(message)
            
        default:
            throw SceneError.invalidResponse
        }
    }
    
    /// Get all entities in the scene
    public func getAllEntities() async throws -> [Entity] {
        let request = EngineRequest.getAllEntities
        let response: EngineResponse = try await socketClient.sendEngineRequest(request)
        
        switch response {
        case .entityList(let entityInfos):
            // Update cached entities
            cachedEntities.removeAll()
            var entities: [Entity] = []
            
            for entityInfo in entityInfos {
                let entity = Entity(id: entityInfo.id, socketClient: socketClient, label: entityInfo.label)
                cachedEntities[entityInfo.id] = entity
                entities.append(entity)
            }
            
            return entities
            
        case .error(let message):
            throw SceneError.engineError(message)
            
        default:
            throw SceneError.invalidResponse
        }
    }
    
    /// Get entity by ID
    /// - Parameter id: Entity ID
    /// - Returns: Entity if found
    public func getEntity(id: UInt64) async throws -> Entity? {
        // Check cache first
        if let cachedEntity = cachedEntities[id] {
            return cachedEntity
        }
        
        // Try to get entity info from engine
        let request = EngineRequest.getEntityInfo(id: id)
        let response: EngineResponse = try await socketClient.sendEngineRequest(request)
        
        switch response {
        case .entityInfo(let entityId, let label, _, _):
            let entity = Entity(id: entityId, socketClient: socketClient, label: label)
            cachedEntities[entityId] = entity
            return entity
            
        case .error(let message):
            if message.contains("not found") {
                return nil
            }
            throw SceneError.engineError(message)
            
        default:
            throw SceneError.invalidResponse
        }
    }
    
    /// Get entity by label
    /// - Parameter label: Entity label to search for
    /// - Returns: First entity with matching label, if found
    public func getEntity(_ label: String) async throws -> Entity? {
        let request = EngineRequest.getEntitiesByLabel(label: label)
        let response: EngineResponse = try await socketClient.sendEngineRequest(request)
        
        switch response {
        case .entityList(let entityInfos):
            guard let firstEntityInfo = entityInfos.first else {
                return nil
            }
            
            let entity = Entity(id: firstEntityInfo.id, socketClient: socketClient, label: firstEntityInfo.label)
            cachedEntities[firstEntityInfo.id] = entity
            return entity
            
        case .error(let message):
            if message.contains("not found") {
                return nil
            }
            throw SceneError.engineError(message)
            
        default:
            throw SceneError.invalidResponse
        }
    }
    
    /// Get all entities with a specific label
    /// - Parameter label: Entity label to search for
    /// - Returns: Array of entities with matching label
    public func getEntities(withLabel label: String) async throws -> [Entity] {
        let request = EngineRequest.getEntitiesByLabel(label: label)
        let response: EngineResponse = try await socketClient.sendEngineRequest(request)
        
        switch response {
        case .entityList(let entityInfos):
            var entities: [Entity] = []
            
            for entityInfo in entityInfos {
                let entity = Entity(id: entityInfo.id, socketClient: socketClient, label: entityInfo.label)
                cachedEntities[entityInfo.id] = entity
                entities.append(entity)
            }
            
            return entities
            
        case .error(let message):
            throw SceneError.engineError(message)
            
        default:
            throw SceneError.invalidResponse
        }
    }
    
    // MARK: - Entity Management
    
    /// Create a new entity in the scene
    /// - Parameter label: Optional label for the new entity
    /// - Returns: The newly created entity
    public func createEntity(label: String? = nil) async throws -> Entity {
        let request = EngineRequest.createEntity(label: label)
        let response: EngineResponse = try await socketClient.sendEngineRequest(request)
        
        switch response {
        case .entityCreated(let id):
            let entity = Entity(id: id, socketClient: socketClient, label: label)
            cachedEntities[id] = entity
            return entity
            
        case .error(let message):
            throw SceneError.engineError(message)
            
        default:
            throw SceneError.invalidResponse
        }
    }
    
    /// Delete an entity from the scene
    /// - Parameter entity: Entity to delete
    public func deleteEntity(_ entity: Entity) async throws {
        try await deleteEntity(id: entity.id)
    }
    
    /// Delete an entity by ID
    /// - Parameter id: ID of entity to delete
    public func deleteEntity(id: UInt64) async throws {
        let request = EngineRequest.deleteEntity(id: id)
        let response: EngineResponse = try await socketClient.sendEngineRequest(request)
        
        switch response {
        case .success:
            cachedEntities.removeValue(forKey: id)
            
        case .error(let message):
            throw SceneError.engineError(message)
            
        default:
            throw SceneError.invalidResponse
        }
    }
    
    // MARK: - Cache Management
    
    /// Clear cached entities (forces refresh on next access)
    public func clearCache() {
        cachedEntities.removeAll()
    }
    
    /// Refresh scene data from engine
    public func refresh() async throws {
        clearCache()
        _ = try await getSceneInfo()
    }
    
    // MARK: - Scene Queries
    
    /// Get the current number of entities in the scene
    public func getEntityCount() async throws -> Int {
        let sceneInfo = try await getSceneInfo()
        return sceneInfo.entityCount
    }
    
    /// Check if an entity exists in the scene
    /// - Parameter id: Entity ID to check
    /// - Returns: True if entity exists
    public func hasEntity(id: UInt64) async throws -> Bool {
        return try await getEntity(id: id) != nil
    }
    
    /// Check if an entity with the given label exists
    /// - Parameter label: Entity label to check
    /// - Returns: True if entity exists
    public func hasEntity(withLabel label: String) async throws -> Bool {
        return try await getEntity(label) != nil
    }
    
    // MARK: - Convenience Methods
    
    /// Find entities with Transform component
    public func getEntitiesWithTransform() async throws -> [Entity] {
        let allEntities = try await getAllEntities()
        var entitiesWithTransform: [Entity] = []
        
        for entity in allEntities {
            if try await entity.hasComponent("Transform") {
                entitiesWithTransform.append(entity)
            }
        }
        
        return entitiesWithTransform
    }
    
    /// Find entities with Script component
    public func getScriptedEntities() async throws -> [Entity] {
        let allEntities = try await getAllEntities()
        var scriptedEntities: [Entity] = []
        
        for entity in allEntities {
            if try await entity.hasComponent("Script") {
                scriptedEntities.append(entity)
            }
        }
        
        return scriptedEntities
    }
}

/// Information about the scene
public struct SceneInfo {
    public let entityCount: Int
    public let entities: [EntityInfo]
    
    public init(entityCount: Int, entities: [EntityInfo]) {
        self.entityCount = entityCount
        self.entities = entities
    }
}

/// Errors that can occur with Scene operations
public enum SceneError: Error, LocalizedError {
    case invalidResponse
    case engineError(String)
    case entityNotFound(UInt64)
    case entityNotFoundWithLabel(String)
    
    public var errorDescription: String? {
        switch self {
        case .invalidResponse:
            return "Invalid response from engine"
        case .engineError(let message):
            return "Engine error: \(message)"
        case .entityNotFound(let id):
            return "Entity with ID \(id) not found"
        case .entityNotFoundWithLabel(let label):
            return "Entity with label '\(label)' not found"
        }
    }
}

// MARK: - Extensions

extension Scene: CustomStringConvertible {
    public var description: String {
        return "Scene(system)"
    }
}