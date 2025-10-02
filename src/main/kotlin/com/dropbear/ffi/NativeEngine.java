package com.dropbear.ffi;

import com.dropbear.math.Transform;

/**
 * A class for dealing with native functions that is required by the Kotlin scripting
 * and the Rust game engine.
 */
public class NativeEngine {
    /**
     * Fetches the transform of the entity by its label in the editor
     * @param label The label of the entity
     * @return The {@link Transform} component if the entity exists, null if not.
     */
    public native Transform getTransformOfEntity(String label);

    /**
     * Fetches the label of the entity this script is currently attached to.
     * @return The label of the attached entity or null if its not attached
     */
    public native String getLabelOfAttachedEntity();
}
