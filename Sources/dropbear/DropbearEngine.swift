import Foundation

/// Main interface for communicating with the Dropbear game engine
/// Provides high-level access to scene, input, and entity systems
public class DropbearEngine {
    /// Socket client for communication with Rust engine
    public let socketClient: SocketClient
    
    /// Scene management
    public let scene: Scene
    
    /// Input handling
    public let input: Input
    
    /// Connection status
    public var isConnected: Bool {
        // For Network framework, we need to track connection state manually
        return true // Simplified for now - could track actual connection state
    }
    
    /// Initialize the Dropbear engine interface
    /// - Parameters:
    ///   - host: Engine host address (default: "127.0.0.1")
    ///   - port: Engine port (default: 7878)
    public init(host: String = "127.0.0.1", port: UInt16 = 7878) {
        self.socketClient = SocketClient(host: host, port: Int(port))
        self.scene = Scene(socketClient: socketClient)
        self.input = Input(socketClient: socketClient)
    }
    
    // MARK: - Connection Management
    
    /// Connect to the Dropbear engine
    public func connect() async throws {
        try await socketClient.connect()
    }
    
    /// Disconnect from the engine
    public func disconnect() async {
        await socketClient.disconnect()
    }
    
    /// Test connection with ping
    public func ping() async throws -> Bool {
        let request = EngineRequest.ping
        let response: EngineResponse = try await socketClient.sendEngineRequest(request)
        
        switch response {
        case .pong:
            return true
        case .error(let message):
            throw EngineError.connectionError(message)
        default:
            return false
        }
    }
    
    // MARK: - System Information
    
    /// Get current delta time from engine
    public func getDeltaTime() async throws -> Float {
        let request = EngineRequest.getDeltaTime
        let response: EngineResponse = try await socketClient.sendEngineRequest(request)
        
        switch response {
        case .deltaTime(let dt):
            return dt
        case .error(let message):
            throw EngineError.systemError(message)
        default:
            throw EngineError.invalidResponse
        }
    }
    
    // MARK: - Convenience Methods
    
    /// Quick entity lookup by ID
    public func getEntity(id: UInt64) async throws -> Entity? {
        return try await scene.getEntity(id: id)
    }
    
    /// Quick entity lookup by label
    public func getEntity(label: String) async throws -> Entity? {
        return try await scene.getEntity(label)
    }
    
    /// Create a new entity with optional label
    public func createEntity(label: String? = nil) async throws -> Entity {
        return try await scene.createEntity(label: label)
    }
    
    /// Check if a key is pressed
    public func isKeyPressed(_ key: Key) async throws -> Bool {
        return try await input.isKeyPressed(key)
    }
    
    /// Get mouse position
    public func getMousePosition() async throws -> Vector2 {
        return try await input.getMousePosition()
    }
    
    /// Get movement input (WASD)
    public func getMovementInput() async throws -> Vector2 {
        return try await input.getMovementInput()
    }
}

/// Global engine instance (optional convenience)
/// Note: This is a global mutable state for convenience in simple use cases
/// In production code, prefer passing engine instances explicitly
nonisolated(unsafe) public var Engine: DropbearEngine?

/// Initialize the global engine instance
/// - Parameters:
///   - host: Engine host address (default: "127.0.0.1")
///   - port: Engine port (default: 7878)
public func initializeEngine(host: String = "127.0.0.1", port: UInt16 = 7878) {
    Engine = DropbearEngine(host: host, port: port)
}

/// Connect to engine using global instance
public func connectToEngine() async throws {
    guard let engine = Engine else {
        throw EngineError.notInitialized
    }
    try await engine.connect()
}

/// Errors that can occur with engine operations
public enum EngineError: Error, LocalizedError {
    case notInitialized
    case connectionError(String)
    case systemError(String)
    case invalidResponse
    
    public var errorDescription: String? {
        switch self {
        case .notInitialized:
            return "Engine not initialized. Call initializeEngine() first."
        case .connectionError(let message):
            return "Connection error: \(message)"
        case .systemError(let message):
            return "System error: \(message)"
        case .invalidResponse:
            return "Invalid response from engine"
        }
    }
}