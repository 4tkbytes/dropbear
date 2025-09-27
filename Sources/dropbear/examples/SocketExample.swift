import Foundation

/// Example demonstrating how to use the Dropbear engine with socket communication
public class ExampleScript: RunnableScript {
    private var engine: DropbearEngine?
    private var player: Entity?
    private var deltaTime: Float = 0.0
    
    public required init() {
        // Initialize is called when the library loads
    }
    
    public func onLoad() {
        // This runs once during runtime initialization
        Task {
            await setupEngine()
        }
    }
    
    public func onUpdate(dt: Float) {
        self.deltaTime = dt
        
        // Run update logic asynchronously
        Task {
            await updateGame()
        }
    }
    
    public func run() {
        // Called by engine - delegates to onUpdate
        // The actual implementation depends on your engine's calling pattern
    }
    
    // MARK: - Engine Setup
    
    private func setupEngine() async {
        do {
            // Initialize engine connection
            engine = DropbearEngine(host: "127.0.0.1", port: 7878)
            
            // Connect to the engine
            try await engine!.connect()
            print("✅ Connected to Dropbear engine!")
            
            // Test connection
            let pingSuccess = try await engine!.ping()
            print("🏓 Ping test: \(pingSuccess ? "SUCCESS" : "FAILED")")
            
            // Get scene information
            let sceneInfo = try await engine!.scene.getSceneInfo()
            print("🎬 Scene has \(sceneInfo.entityCount) entities")
            
            // Try to find a player entity
            player = try await engine!.scene.getEntity("player")
            if player == nil {
                // Create a player if it doesn't exist
                player = try await engine!.scene.createEntity(label: "player")
                print("👤 Created player entity with ID: \(player!.id)")
                
                // Set initial position
                try await player!.setPosition(SimpleVector3(0, 1, 0))
                try await player!.setScale(SimpleVector3.one)
            } else {
                print("👤 Found existing player entity with ID: \(player!.id)")
            }
            
        } catch {
            print("❌ Failed to setup engine: \(error)")
        }
    }
    
    // MARK: - Game Logic
    
    private func updateGame() async {
        guard let engine = engine, let player = player else { return }
        
        do {
            // Handle input
            await handlePlayerInput(engine: engine, player: player)
            
            // Update game logic
            await updateGameLogic(engine: engine)
            
        } catch {
            print("⚠️ Update error: \(error)")
        }
    }
    
    private func handlePlayerInput(engine: DropbearEngine, player: Entity) async {
        do {
            // Get movement input
            let movement = try await engine.input.getMovementInput()
            
            if movement.magnitude > 0 {
                // Move player based on input
                let moveSpeed: Float = 5.0
                let moveVector = SimpleVector3(
                    movement.x * moveSpeed * deltaTime,
                    0,
                    movement.y * moveSpeed * deltaTime
                )
                
                try await player.translate(by: moveVector)
                
                let position = try await player.getPosition()
                print("🏃 Player moved to: (\(position.x), \(position.y), \(position.z))")
            }
            
            // Handle jumping
            if try await engine.input.isSpacePressed() {
                try await player.translate(by: SimpleVector3(0, 2.0 * deltaTime, 0))
                print("🦘 Player jumped!")
            }
            
            // Handle rotation
            let mouseDelta = try await engine.input.getMouseDelta()
            if mouseDelta.magnitude > 0 {
                let sensitivity: Float = 0.1
                let rotationY = mouseDelta.x * sensitivity
                
                var rotation = try await player.getRotation()
                rotation.y += rotationY
                try await player.setRotation(rotation)
            }
            
            // Check for escape to quit
            if try await engine.input.isEscapePressed() {
                print("👋 Escape pressed - disconnecting...")
                await engine.disconnect()
            }
            
        } catch {
            // Don't spam errors for input handling
            if !(error is InputError) {
                print("⚠️ Input handling error: \(error)")
            }
        }
    }
    
    private func updateGameLogic(engine: DropbearEngine) async {
        do {
            // Example: Rotate all entities slowly
            let allEntities = try await engine.scene.getAllEntities()
            
            for entity in allEntities {
                if entity.id != player?.id { // Don't rotate the player
                    if try await entity.hasComponent("Transform") {
                        var rotation = try await entity.getRotation()
                        rotation.y += 30.0 * deltaTime // 30 degrees per second
                        try await entity.setRotation(rotation)
                    }
                }
            }
            
        } catch {
            // Don't spam errors for game logic
            if error.localizedDescription != "Entity has no Transform component" {
                print("⚠️ Game logic error: \(error)")
            }
        }
    }
    
    // MARK: - Utility Methods
    
    private func printEntityInfo(_ entity: Entity) async {
        do {
            try await entity.refreshInfo()
            let components = try await entity.getComponents()
            let position = try await entity.getPosition()
            
            print("📋 Entity \(entity.id):")
            print("   Label: \(entity.label ?? "none")")
            print("   Position: (\(position.x), \(position.y), \(position.z))")
            print("   Components: \(components.joined(separator: ", "))")
            
        } catch {
            print("❌ Failed to get entity info: \(error)")
        }
    }
    
    deinit {
        // Clean up connection - simplified for deinit
        // Proper cleanup should be done before object is deallocated
    }
}

// MARK: - Simple Usage Example

/// Simple example showing basic usage
public func runSimpleExample() async {
    do {
        // Initialize and connect
        let engine = DropbearEngine()
        try await engine.connect()
        
        print("🎮 Connected to Dropbear engine!")
        
        // Test basic operations
        let deltaTime = try await engine.getDeltaTime()
        print("⏱️ Current delta time: \(deltaTime)")
        
        // Test input
        let wasdPressed = try await engine.input.isAnyKeyPressed([.W, .A, .S, .D])
        print("🎹 WASD pressed: \(wasdPressed)")
        
        // Test scene
        let entityCount = try await engine.scene.getEntityCount()
        print("🏗️ Entities in scene: \(entityCount)")
        
        // Create and manipulate entity
        let testEntity = try await engine.createEntity(label: "test")
        try await testEntity.setPosition(SimpleVector3(1, 2, 3))
        let position = try await testEntity.getPosition()
        print("📍 Test entity position: (\(position.x), \(position.y), \(position.z))")
        
        // Clean up
        try await engine.scene.deleteEntity(testEntity)
        await engine.disconnect()
        
        print("✅ Example completed successfully!")
        
    } catch {
        print("❌ Example failed: \(error)")
    }
}