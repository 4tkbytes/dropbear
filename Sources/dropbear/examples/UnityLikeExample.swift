import Foundation

/// Simple Unity-like PlayerController without @Script macro for testing
class UnityLikePlayerController: BaseScript {
    
    override func onLoad() {
        print("🎮 Unity-like Player Controller loaded!")
        print("✅ Engine is automatically connected and ready!")
        
        // Set up player entity - clean and Unity-like!
        Task {
            try? await withEngine { engine in
                let player = try await engine.scene.createEntity(label: "Player")
                try await player.setPosition(SimpleVector3(x: 0, y: 0, z: 0))
                print("🏃 Player created at origin")
            }
        }
    }
    
    override func onUpdate(dt: Float) {
        // Clean update loop - just like Unity's Update()
        Task {
            try? await withEngine { engine in
                await handlePlayerInput(engine: engine)
            }
        }
    }
    
    // MARK: - Game Logic
    
    private func handlePlayerInput(engine: DropbearEngine) async {
        do {
            var movement = SimpleVector3(x: 0, y: 0, z: 0)
            
            // Check input keys (using correct Key enum values)
            if try await engine.input.isKeyPressed(.W) { movement.z -= 1 }
            if try await engine.input.isKeyPressed(.S) { movement.z += 1 }
            if try await engine.input.isKeyPressed(.A) { movement.x -= 1 }
            if try await engine.input.isKeyPressed(.D) { movement.x += 1 }
            
            // Apply movement to player
            if movement.x != 0 || movement.z != 0 {
                if let player = try? await engine.scene.getEntity("Player") {
                    try await player.translate(by: movement)
                }
            }
            
        } catch {
            print("⚠️ Input error: \(error)")
        }
    }
}

/// Simple Enemy AI without @Script macro  
class UnityLikeEnemyAI: BaseScript {
    private var patrolPoints: [SimpleVector3] = []
    private var currentTargetIndex = 0
    private var patrolSpeed: Float = 2.0
    
    override func onLoad() {
        print("🤖 Unity-like Enemy AI loaded!")
        
        // Set up enemy and patrol
        Task {
            try? await withEngine { engine in
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
        Task {
            try? await withEngine { engine in
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
                
                let direction = SimpleVector3(
                    x: targetPos.x - currentPos.x,
                    y: 0,
                    z: targetPos.z - currentPos.z
                )
                
                let distance = sqrt(direction.x * direction.x + direction.z * direction.z)
                
                if distance < 0.5 {
                    currentTargetIndex = (currentTargetIndex + 1) % patrolPoints.count
                } else {
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
                    
                    try await enemy.translate(by: moveAmount)
                }
            }
        } catch {
            print("⚠️ Enemy AI error: \(error)")
        }
    }
}

/// Demo function showing the Unity-like experience
func demonstrateUnityLikeScripts() async {
    print("🚀 Demonstrating Unity-like script system...")
    
    let playerController = UnityLikePlayerController()
    let enemyAI = UnityLikeEnemyAI()
    
    // Run scripts (in real game, the engine would do this)
    Task {
        await playerController.run()
    }
    
    Task {
        await enemyAI.run()
    }
    
    print("✅ Scripts are running with Unity-like simplicity!")
    print("   - No visible async/await complexity")
    print("   - Automatic engine connection")
    print("   - Clean onLoad/onUpdate lifecycle")
}