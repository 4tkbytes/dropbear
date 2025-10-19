package com.dropbear.ffi;

import com.dropbear.math.Transform;

public class JNINative {
    static {
        System.loadLibrary("eucalyptus_core");
    }

    // entity and ecs
    public static native long getEntity(long handle, String label);

    // transformations
    public static native Transform getTransform(long handle, long entityHandle);
    public static native void setTransform(long worldHandle, long id, Transform transform);

    // input
    public static native void printInputState(long inputHandle);
    public static native boolean isKeyPressed(long inputHandle, int ordinal);
}
