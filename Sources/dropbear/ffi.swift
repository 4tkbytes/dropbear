private class InternalFFIData {
    static let shared = InternalFFIData()
    let input_data: RawInputData

    private init() {

    }
}

struct FFIInputState: Codable {
    var keysPressed: [UInt32]
    var mousePosition: (Float, Float)
    var mouseDelta: (Float, Float)
    var mouseButtons: [UInt32]

    init(keysPressed: [UInt32] = [],
        mousePosition: (Float, Float) = (0, 0),
        mouseDelta: (Float, Float) = (0, 0),
        mouseButtons: [UInt32] = [])
    {
        self.keysPressed = keysPressed
        self.mousePosition = mousePosition
        self.mouseDelta = mouseDelta
        self.mouseButtons = mouseButtons
    }
}