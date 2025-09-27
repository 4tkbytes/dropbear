// example testing module, do not use in production

import Foundation

@Script
class Player: BaseScript {
    override func onLoad() {
        print("Player script loaded")
        // Clean, simple API - just like Unity!
        // No Task {}, no await setupEngine(), no connection management needed
        
        // You can access engine directly when needed
        // The BaseScript handles all the connection complexity behind the scenes
        print("✅ Engine ready to use!")
    }
    
    override func onUpdate(dt: Float) {
        print("Player is running: \(dt)")
        
        // If you need to use async engine features, wrap them in Task
        // But the engine is already set up and ready
        Task {
            if let engine = engine {
                do {
                    // Clean async API usage when needed
                    if try await engine.input.isKeyPressed(Key.W) {
                        print("W key pressed - moving forward!")
                        // Get current scene and move player
                        // let scene = engine.scene
                        // let player = try await scene.getEntity("player")
                        // try await player?.translate(SimpleVector3(x: 0, y: 0, z: 1))
                    }
                } catch {
                    print("Input check failed: \(error)")
                }
            }
        }
    }
}