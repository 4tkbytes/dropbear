/// A class that all scripts inherit, which define simple functions which are ran
/// in the dropbear rust engine. 
public class RunnableScript {
    /// Loads the content/variables/states into the entity/scene. 
    /// It runs ONLY ONCE. 
    public func onLoad() {
        fatalError("Hey there! I'm not overriden. You might wanna do something about this onLoad function!")
    }

    /// Updates the engine context with the content inside the function
    /// - Parameter dt: Deltatime/the time it takes for the previous frame to render as a `Float`
    public func onUpdate(dt: Float) {
        fatalError("Hey there! I'm not overriden. You might wanna do something about this onUpdate function!")
    }
}

public func getInput() -> Input {
    Input()
}

// todo
public func getCurrentScene() -> Scene! {
    if true /* check if script is attached to scene */ {
        Scene()
    } else {
        nil
    }
}

// todo
public func getAttachedEntity() -> Entity {
    Entity()
}

// todo
public func getScene() -> Scene {
    Scene()
}