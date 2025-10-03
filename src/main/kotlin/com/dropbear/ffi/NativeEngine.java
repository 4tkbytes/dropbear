package com.dropbear.ffi;

/**
 * Native interface to the Rust game engine.
 * This class provides low-level FFI bindings via JNI.
 * 
 * All methods are static and call into native Rust code.
 */
public class NativeEngine {
    static {
        // Load the native library
        // In production, this should load from the JAR or a known location
        try {
            System.loadLibrary("eucalyptus_core");
        } catch (UnsatisfiedLinkError e) {
            System.err.println("Failed to load native library: " + e.getMessage());
        }
    }

    // =============================================================================
    // Context Management
    // =============================================================================
    
    /**
     * Set the current scripting context (internal use only)
     */
    public static native void setContext(long entityId, long worldPtr);

    // =============================================================================
    // Transform - Position
    // =============================================================================
    
    public static native double getPositionX();
    public static native double getPositionY();
    public static native double getPositionZ();
    public static native void setPosition(double x, double y, double z);

    // =============================================================================
    // Transform - Rotation (Quaternion)
    // =============================================================================
    
    public static native double getRotationX();
    public static native double getRotationY();
    public static native double getRotationZ();
    public static native double getRotationW();
    public static native void setRotation(double x, double y, double z, double w);

    // =============================================================================
    // Transform - Scale
    // =============================================================================
    
    public static native double getScaleX();
    public static native double getScaleY();
    public static native double getScaleZ();
    public static native void setScale(double x, double y, double z);

    // =============================================================================
    // Input System
    // =============================================================================
    
    /**
     * Check if a key is currently pressed
     * @param keycode The keycode to check (use KeyCode constants)
     * @return true if the key is pressed, false otherwise
     */
    public static native boolean isKeyPressed(long keycode);
    
    /**
     * Get the current mouse X position
     */
    public static native double getMouseX();
    
    /**
     * Get the current mouse Y position
     */
    public static native double getMouseY();
}
