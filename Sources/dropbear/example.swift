// example testing module, do not use in production

@ScriptEntry
class Player: BaseScript {
    override func onLoad() {
        let input = dropbear.getInput();


    }

    override func onUpdate(dt: Float) {
        print("Player is running: \(dt)")
    }
}