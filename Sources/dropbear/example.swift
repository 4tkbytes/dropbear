// example testing module, do not use in production

@ScriptEntry
class Player: BaseScript {
    override func onLoad() {
        print("Player script loaded")
        // let current_scene = dropbear.getCurrentScene()
        // let player = current_scene?.getEntity("player")
        if dropbear.getInput().isKeyPressed(Key.W) {
            // player?.moveForward()
        }
    }

    override func onUpdate(dt: Float) {
        print("Player is running: \(dt)")
    }
}