package com.dropbear.ffi;

public class JNINative {
    static {
        System.loadLibrary("eucalyptus_core");
    }

    public static native long getEntity(long handle, String label);
}
