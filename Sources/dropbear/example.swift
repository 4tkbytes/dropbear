// example testing module, do not use in production

class Player: RunnableScript {
    override func onLoad() {
        print("I have risen")
        // let current_scene = dropbear.getCurrentScene()
        // let player = current_scene?.getEntity("player")
        if dropbear.getInput().isKeyPressed(Key.W) {
            // player?.moveForward()
        }
    }

    override func onUpdate(dt: Float) {
        print("I am currently awake")
    }
}