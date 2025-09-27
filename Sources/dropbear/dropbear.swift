/// A class that all scripts inherit, which define simple functions which are ran
/// in the dropbear rust engine. 
public protocol RunnableScript: AnyObject {
    /// The init function. This is the constructor for the class to initialise. 
    /// 
    /// You can setup whatever variables and states, however it is recommended to use
    /// the `onLoad` function instead as that is actually ran during the runtime
    /// compared to `init`, which is initialised when the library is loaded. 
    init()

    /// Loads the content/variables/states into the entity/scene. 
    /// 
    /// This is ran only once and is loaded during runtime. 
    func onLoad()

    /// Updates the engine context with the content inside the function
    /// 
    /// This is ran every frame and is loaded during the runtime. 
    /// - Parameter dt: Deltatime/the time it takes for the previous frame to render as a `Float`
    func onUpdate(dt: Float)

    /// Internal: Run the script (called by engine)
    func run()
}

/// Registry for managing script classes and their metadata. 
@MainActor
public class ScriptRegistry {
    private static var scriptClasses: [String: RunnableScript.Type] = [:]
    private static var entityScripts: [String: String] = [:]  // entity -> fileName mapping
    
    /// Register a script class with its file name
    public static func registerScript<T: RunnableScript>(_ type: T.Type, fileName: String) {
        scriptClasses[fileName] = type
    }
    
    /// Register a script for a specific entity
    public static func registerEntityScript(_ fileName: String, entity: String) {
        entityScripts[entity] = fileName
    }
    
    /// Get script class by file name
    public static func getScript(fileName: String) -> RunnableScript.Type? {
        return scriptClasses[fileName]
    }
    
    /// Get script file name for specific entity
    public static func getScriptForEntity(_ entity: String) -> String? {
        return entityScripts[entity]
    }
    
    /// Create script instance by file name
    public static func createScript(fileName: String) -> RunnableScript? {
        return getScript(fileName: fileName)?.init()
    }
    
    /// Create script instance for entity (checks entity-specific first, then falls back to fileName)
    public static func createScriptForEntity(_ entity: String, fallbackFileName: String) -> RunnableScript? {
        if let specificFileName = getScriptForEntity(entity) {
            return createScript(fileName: specificFileName)
        }
        return createScript(fileName: fallbackFileName)
    }
}

import Foundation

/// Base class for all scripts - handles engine connection automatically
open class BaseScript: RunnableScript {
    private var _engine: DropbearEngine?
    private var setupTask: Task<Void, Never>?
    
    /// Access to the connected engine
    public var engine: DropbearEngine? {
        return _engine
    }
    
    /// Safe engine access with auto-connection
    public func withEngine<T>(_ operation: @Sendable (DropbearEngine) async throws -> T) async rethrows -> T? {
        guard let engine = await ensureEngineConnected() else {
            print("⚠️ Engine connection failed")
            return nil
        }
        
        return try await operation(engine)
    }
    
    required public init() {
        // Start engine setup immediately but don't block
        setupTask = Task {
            _ = await ensureEngineConnected()
        }
    }
    
    /// Override these methods in your script
    open func onLoad() {}
    
    open func onUpdate(dt: Float) {}
    
    /// Internal engine connection management
    private func ensureEngineConnected() async -> DropbearEngine? {
        if let engine = _engine {
            return engine
        }
        
        do {
            let newEngine = DropbearEngine()
            try await newEngine.connect()
            _engine = newEngine
            print("🔧 Engine connected successfully")
            return newEngine
        } catch {
            print("❌ Failed to connect to engine: \(error)")
            return nil
        }
    }
    
    /// Run the script (called by the script system)
    public func run() async {
        // Wait for engine setup to complete
        await setupTask?.value
        
        // Call user's onLoad method
        onLoad()
        
        // Game loop simulation (in real game, this would be called by the engine)
        var lastTime = Date()
        while true {
            let currentTime = Date()
            let deltaTime = Float(currentTime.timeIntervalSince(lastTime))
            lastTime = currentTime
            
            onUpdate(dt: deltaTime)
            
            // Don't overwhelm the CPU
            try? await Task.sleep(nanoseconds: 16_666_667) // ~60fps
        }
    }
}

/// A macro for a class of a script that can be used with any entity (when added).  
/// 
/// Let's say that you have an entity of a player. You want to get movement for
/// that Player. The Eucalyptus Editor only allows for one `Swift` file per entity. 
/// To combat that, there is a macro called `@ScriptEntry`, which allows for that class
/// to be ran (in no particular order) in tandem with the Player.
/// 
/// In the case that you want a script to be locked to **only** a specific entity,
/// you can use the `@Script(name: /*Entity Label*/)` to lock that class to run only on
/// that entity, improving production as you won't have to constantly rewrite scripts. 
@attached(member, names: named(init))
public macro Script() = #externalMacro(module: "dropbear_macro", type: "ScriptEntryMacro")

/// A macro for a class of a script that can be used by a **specific** entity. 
/// 
/// Imagine you have an enemy. You have a class that deals with movement, a class that deals with 
/// health, but you want an Enemy specific class for its own system. This macro helps with dealing with 
/// such an issue, allowing you to attach this script to other entities as well. 
/// 
/// This macro also gives the class a higher priority compared to the `@ScriptEntry` classes, allowing this
/// script to run earlier than any ScriptEntry derived classes. 
/// 
/// FYI: This macro does not update if you change the label. If the label in editor is 
/// different than what is provided, this class will not run for that entity. 
/// 
/// # Parameters
/// - name: A String to the label of the entity set by you.  
@attached(member, names: named(init))
public macro Script(entity: String) = #externalMacro(module: "dropbear_macro", type: "ScriptMacro")

public func getInput() -> Input {
    // For now, create a dummy socket client - this should be improved
    let socketClient = SocketClient()
    return Input(socketClient: socketClient)
}

// todo
public func getCurrentScene() -> Scene? {
    if true /* check if script is attached to scene */ {
        let socketClient = SocketClient()
        return Scene(socketClient: socketClient)
    } else {
        return nil
    }
}

// todo
public func getAttachedEntity() -> Entity {
    let socketClient = SocketClient()
    return Entity(id: 0, socketClient: socketClient, label: "dummy")
}

// todo
public func getScene() -> Scene {
    let socketClient = SocketClient()
    return Scene(socketClient: socketClient)
}