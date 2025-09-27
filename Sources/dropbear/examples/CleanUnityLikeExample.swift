import Foundation

/// Example of a clean Unity-like PlayerController script
/// Notice how there's no visible async/await complexity!
@Script
class PlayerController: BaseScript {
    
    // MARK: - Unity-like Lifecycle Methods
    
    override func onLoad() {
        print("🎮 Player controller loaded!")
        print("✅ Engine is automatically connected and ready!")
        
        // Set up player entity when component loads - no async complexity visible!
        Task {
            await withEngine { engine in
                let player = try await engine.scene.createEntity(label: "Player")
                try await player.setPosition(SimpleVector3(x: 0, y: 0, z: 0))
                print("🏃 Player created at origin")
            }
        }
    }
    
    override func onUpdate(dt: Float) {
        // Clean update loop - just like Unity's Update()
        // Handle input each frame
        Task {
            await withEngine { engine in
                await handlePlayerInput(engine: engine)
            }
        }
    }
    
    // MARK: - Game Logic (Your actual game code)
    
    /// Handle player input each frame
    private func handlePlayerInput(engine: DropbearEngine) async {
        do {
            // Simple, clean input handling
            var movement = SimpleVector3(x: 0, y: 0, z: 0)
            
            if try await engine.input.isKeyPressed(.w) { movement.z -= 1 }
            if try await engine.input.isKeyPressed(.s) { movement.z += 1 }
            if try await engine.input.isKeyPressed(.a) { movement.x -= 1 }
            if try await engine.input.isKeyPressed(.d) { movement.x += 1 }
            
            // Apply movement to player
            if movement.x != 0 || movement.z != 0 {
                if let player = try? await engine.scene.getEntity("Player") {
                    try await player.translate(movement)
                }
            }
            
        } catch {
            // Handle errors gracefully
            print("⚠️ Input error: \(error)")
        }
    }
}

// MARK: - Advanced Example: Enemy AI

/// Example of an Enemy AI script - also Unity-like!
@Script 
class EnemyAI: BaseScript {
    private var patrolPoints: [SimpleVector3] = []
    private var currentTargetIndex = 0
    private var patrolSpeed: Float = 2.0
    
    override func onLoad() {
        print("🤖 Enemy AI loaded!")
        
        // Set up enemy and patrol points
        Task {
            await withEngine { engine in
                // Create enemy entity
                let enemy = try await engine.scene.createEntity(label: "Enemy")
                try await enemy.setPosition(SimpleVector3(x: 5, y: 0, z: 0))
                print("👾 Enemy created at (5, 0, 0)")
                
                // Set up patrol points
                patrolPoints = [
                    SimpleVector3(x: 5, y: 0, z: 0),
                    SimpleVector3(x: 10, y: 0, z: 0),
                    SimpleVector3(x: 10, y: 0, z: 5),
                    SimpleVector3(x: 5, y: 0, z: 5)
                ]
            }
        }
    }
    
    override func onUpdate(dt: Float) {
        // AI behavior runs every frame
        Task {
            await withEngine { engine in
                await updatePatrol(engine: engine, deltaTime: dt)
            }
        }
    }
    
    private func updatePatrol(engine: DropbearEngine, deltaTime: Float) async {
        guard !patrolPoints.isEmpty else { return }
        
        do {
            if let enemy = try? await engine.scene.getEntity("Enemy") {
                let currentPos = try await enemy.getPosition()
                let targetPos = patrolPoints[currentTargetIndex]
                
                // Simple movement towards target
                let direction = SimpleVector3(
                    x: targetPos.x - currentPos.x,
                    y: 0,
                    z: targetPos.z - currentPos.z
                )
                
                let distance = sqrt(direction.x * direction.x + direction.z * direction.z)
                
                if distance < 0.5 {
                    // Reached target, move to next patrol point
                    currentTargetIndex = (currentTargetIndex + 1) % patrolPoints.count
                } else {
                    // Move towards target
                    let normalizedDir = SimpleVector3(
                        x: direction.x / distance,
                        y: 0,
                        z: direction.z / distance
                    )
                    
                    let moveAmount = SimpleVector3(
                        x: normalizedDir.x * patrolSpeed * deltaTime,
                        y: 0,
                        z: normalizedDir.z * patrolSpeed * deltaTime
                    )
                    
                    try await enemy.translate(moveAmount)
                }
            }
        } catch {
            print("⚠️ Enemy AI error: \(error)")
        }
    }
}

// MARK: - Quick Demo

/// Simple script showing the absolute minimum
@Script
class MinimalScript: BaseScript {
    override func onLoad() {
        print("🎯 Minimal script loaded! Engine ready automatically!")
    }
    
    override func onUpdate(dt: Float) {
        // This runs every frame - add your game logic here
    }
}