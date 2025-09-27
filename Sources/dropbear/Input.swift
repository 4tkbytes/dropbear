import Foundation

/// Input system for handling keyboard and mouse input
/// Provides methods to query key states and mouse position/movement
public class Input {
    /// Reference to the socket client for communication
    private let socketClient: SocketClient
    
    /// Cache for key states to reduce network calls
    private var keyStateCache: [Key: Bool] = [:]
    private var cacheTimestamp: Date = Date()
    private let cacheTimeout: TimeInterval = 0.016 // ~60fps cache timeout
    
    /// Cached mouse data
    private var cachedMousePosition: Vector2?
    private var cachedMouseDelta: Vector2?
    private var mouseCacheTimestamp: Date = Date()
    
    /// Initialize input system with socket client
    /// - Parameter socketClient: Socket client for communication with Rust engine
    public init(socketClient: SocketClient) {
        self.socketClient = socketClient
    }
    
    // MARK: - Keyboard Input
    
    /// Check if a key is currently pressed
    /// - Parameter key: The key to check
    /// - Returns: True if the key is pressed
    public func isKeyPressed(_ key: Key) async throws -> Bool {
        // Check cache first
        if shouldUseCachedKeyState() {
            if let cached = keyStateCache[key] {
                return cached
            }
        }
        
        let keyString = key.rawValue
        let request = EngineRequest.isKeyPressed(key: keyString)
        let response: EngineResponse = try await socketClient.sendEngineRequest(request)
        
        switch response {
        case .keyPressed(let pressed):
            // Update cache
            keyStateCache[key] = pressed
            cacheTimestamp = Date()
            return pressed
            
        case .error(let message):
            throw InputError.engineError(message)
            
        default:
            throw InputError.invalidResponse
        }
    }
    
    /// Check if any of the specified keys are pressed
    /// - Parameter keys: Array of keys to check
    /// - Returns: True if any key is pressed
    public func isAnyKeyPressed(_ keys: [Key]) async throws -> Bool {
        for key in keys {
            if try await isKeyPressed(key) {
                return true
            }
        }
        return false
    }
    
    /// Check if all of the specified keys are pressed
    /// - Parameter keys: Array of keys to check
    /// - Returns: True if all keys are pressed
    public func areAllKeysPressed(_ keys: [Key]) async throws -> Bool {
        for key in keys {
            if !(try await isKeyPressed(key)) {
                return false
            }
        }
        return true
    }
    
    // MARK: - Mouse Input
    
    /// Get current mouse position
    /// - Returns: Mouse position as Vector2
    public func getMousePosition() async throws -> Vector2 {
        // Check cache first
        if shouldUseCachedMouseData() {
            if let cached = cachedMousePosition {
                return cached
            }
        }
        
        let request = EngineRequest.getMousePosition
        let response: EngineResponse = try await socketClient.sendEngineRequest(request)
        
        switch response {
        case .mousePosition(let position):
            // Update cache
            cachedMousePosition = position
            mouseCacheTimestamp = Date()
            return position
            
        case .error(let message):
            throw InputError.engineError(message)
            
        default:
            throw InputError.invalidResponse
        }
    }
    
    /// Get mouse movement delta since last frame
    /// - Returns: Mouse delta as Vector2
    public func getMouseDelta() async throws -> Vector2 {
        // Check cache first
        if shouldUseCachedMouseData() {
            if let cached = cachedMouseDelta {
                return cached
            }
        }
        
        let request = EngineRequest.getMouseDelta
        let response: EngineResponse = try await socketClient.sendEngineRequest(request)
        
        switch response {
        case .mouseDelta(let delta):
            // Update cache
            cachedMouseDelta = delta
            mouseCacheTimestamp = Date()
            return delta
            
        case .error(let message):
            throw InputError.engineError(message)
            
        default:
            throw InputError.invalidResponse
        }
    }
    
    // MARK: - Convenience Methods
    
    /// Check for WASD movement input
    /// - Returns: Movement vector based on WASD keys
    public func getMovementInput() async throws -> Vector2 {
        var movement = Vector2.zero
        
        if try await isKeyPressed(.W) {
            movement.y += 1
        }
        if try await isKeyPressed(.S) {
            movement.y -= 1
        }
        if try await isKeyPressed(.A) {
            movement.x -= 1
        }
        if try await isKeyPressed(.D) {
            movement.x += 1
        }
        
        // Normalize diagonal movement
        if movement.magnitude > 0 {
            movement = movement.normalized
        }
        
        return movement
    }
    
    /// Check for arrow key movement input
    /// - Returns: Movement vector based on arrow keys
    public func getArrowKeyInput() async throws -> Vector2 {
        var movement = Vector2.zero
        
        if try await isKeyPressed(.ArrowUp) {
            movement.y += 1
        }
        if try await isKeyPressed(.ArrowDown) {
            movement.y -= 1
        }
        if try await isKeyPressed(.ArrowLeft) {
            movement.x -= 1
        }
        if try await isKeyPressed(.ArrowRight) {
            movement.x += 1
        }
        
        // Normalize diagonal movement
        if movement.magnitude > 0 {
            movement = movement.normalized
        }
        
        return movement
    }
    
    /// Check if escape key is pressed (common for pause/menu)
    public func isEscapePressed() async throws -> Bool {
        return try await isKeyPressed(.Escape)
    }
    
    /// Check if space key is pressed (common for jump/action)
    public func isSpacePressed() async throws -> Bool {
        return try await isKeyPressed(.Space)
    }
    
    /// Check if enter key is pressed (common for confirm/select)
    public func isEnterPressed() async throws -> Bool {
        return try await isKeyPressed(.Enter)
    }
    
    // MARK: - Cache Management
    
    /// Clear input cache (forces refresh on next query)
    public func clearCache() {
        keyStateCache.removeAll()
        cachedMousePosition = nil
        cachedMouseDelta = nil
        cacheTimestamp = Date.distantPast
        mouseCacheTimestamp = Date.distantPast
    }
    
    private func shouldUseCachedKeyState() -> Bool {
        return Date().timeIntervalSince(cacheTimestamp) < cacheTimeout
    }
    
    private func shouldUseCachedMouseData() -> Bool {
        return Date().timeIntervalSince(mouseCacheTimestamp) < cacheTimeout
    }
    
    // MARK: - Batch Operations
    
    /// Get state of multiple keys at once (more efficient than individual calls)
    /// - Parameter keys: Array of keys to check
    /// - Returns: Dictionary mapping keys to their pressed state
    public func getKeyStates(_ keys: [Key]) async throws -> [Key: Bool] {
        var states: [Key: Bool] = [:]
        
        // Check which keys need to be fetched (not in cache or cache expired)
        let keysToFetch = shouldUseCachedKeyState() 
            ? keys.filter { keyStateCache[$0] == nil }
            : keys
        
        // Fetch uncached keys
        for key in keysToFetch {
            states[key] = try await isKeyPressed(key)
        }
        
        // Add cached keys
        if shouldUseCachedKeyState() {
            for key in keys {
                if let cached = keyStateCache[key] {
                    states[key] = cached
                }
            }
        }
        
        return states
    }
}

/// Key enumeration for input system
public enum Key: String, CaseIterable, Codable {
    // Letter keys
    case A = "A"
    case B = "B"
    case C = "C"
    case D = "D"
    case E = "E"
    case F = "F"
    case G = "G"
    case H = "H"
    case I = "I"
    case J = "J"
    case K = "K"
    case L = "L"
    case M = "M"
    case N = "N"
    case O = "O"
    case P = "P"
    case Q = "Q"
    case R = "R"
    case S = "S"
    case T = "T"
    case U = "U"
    case V = "V"
    case W = "W"
    case X = "X"
    case Y = "Y"
    case Z = "Z"
    
    // Number keys
    case Key1 = "1"
    case Key2 = "2"
    case Key3 = "3"
    case Key4 = "4"
    case Key5 = "5"
    case Key6 = "6"
    case Key7 = "7"
    case Key8 = "8"
    case Key9 = "9"
    case Key0 = "0"
    
    // Special keys
    case Space = "Space"
    case Enter = "Enter"
    case Escape = "Escape"
    case Tab = "Tab"
    case Shift = "Shift"
    case Control = "Control"
    case Alt = "Alt"
    case Backspace = "Backspace"
    case Delete = "Delete"
    
    // Arrow keys
    case ArrowUp = "ArrowUp"
    case ArrowDown = "ArrowDown"
    case ArrowLeft = "ArrowLeft"
    case ArrowRight = "ArrowRight"
    
    // Function keys
    case F1 = "F1"
    case F2 = "F2"
    case F3 = "F3"
    case F4 = "F4"
    case F5 = "F5"
    case F6 = "F6"
    case F7 = "F7"
    case F8 = "F8"
    case F9 = "F9"
    case F10 = "F10"
    case F11 = "F11"
    case F12 = "F12"
    
    /// Get a user-friendly display name for the key
    public var displayName: String {
        switch self {
        case .Key1: return "1"
        case .Key2: return "2"
        case .Key3: return "3"
        case .Key4: return "4"
        case .Key5: return "5"
        case .Key6: return "6"
        case .Key7: return "7"
        case .Key8: return "8"
        case .Key9: return "9"
        case .Key0: return "0"
        case .ArrowUp: return "↑"
        case .ArrowDown: return "↓"
        case .ArrowLeft: return "←"
        case .ArrowRight: return "→"
        default: return self.rawValue
        }
    }
}

/// Errors that can occur with Input operations
public enum InputError: Error, LocalizedError {
    case invalidResponse
    case engineError(String)
    case unsupportedKey(String)
    
    public var errorDescription: String? {
        switch self {
        case .invalidResponse:
            return "Invalid response from engine"
        case .engineError(let message):
            return "Engine error: \(message)"
        case .unsupportedKey(let key):
            return "Unsupported key: \(key)"
        }
    }
}

// MARK: - Extensions

extension Key: CustomStringConvertible {
    public var description: String {
        return displayName
    }
}

extension Input: CustomStringConvertible {
    public var description: String {
        return "Input(system)"
    }
}