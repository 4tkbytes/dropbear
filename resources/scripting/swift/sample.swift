// dropbear engine script example, make the necessary changes

import dropbear

@ScriptEntry
class ChangeMyName: BaseScript {
    override func onLoad() {
        print("It's alive! It's alive! It's alive, it's alive, IT'S ALIVE!")
    }

    override func onUpdate(dt: Float) {
        print("And it's running at: \(1/dt) FPS?")
    }
}