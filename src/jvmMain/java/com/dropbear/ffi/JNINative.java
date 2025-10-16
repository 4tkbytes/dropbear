package com.dropbear.ffi;

import com.dropbear.math.Transform;

public class JNINative {
    static {
        System.loadLibrary("eucalyptus_core");
    }

    public static native long getEntity(long handle, String label);

    public static native Transform getTransform(long handle, long entityHandle);
    public static native void setTransform(long worldHandle, long id, Transform transform);
}
