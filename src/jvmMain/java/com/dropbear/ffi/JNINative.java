package com.dropbear.ffi;

import com.dropbear.Camera;
import com.dropbear.EntityTransform;
import com.dropbear.math.Transform;
import org.jetbrains.annotations.Nullable;

/**
 * Describes all the functions that are available in
 * the `eucalyptus_core` dynamic library. 
 */
public class JNINative {
    static {
        System.loadLibrary("eucalyptus_core");
    }

    // getters
    public static native long getEntity(long worldHandle, String label);
    public static native long getAsset(long assetRegistryHandle, String eucaURI);

    // model
    public static native long getModel(long worldHandle, long entityHandle);
    public static native void setModel(long worldHandle, long assetHandle, long entityHandle, long modelHandle);
    public static native boolean isModelHandle(long assetRegistryHandle, long handle);
    public static native boolean isUsingModel(long worldHandle, long entityHandle, long modelHandle);

    // texture
    public static native long getTexture(long worldHandle, long assetHandle, long entityHandle, String name);
    public static native String getTextureName(long assetHandle, long textureHandle);
    public static native void setTexture(long worldHandle, long assetRegistryHandle, long entityHandle,
                                         String oldMaterialName, long textureHandle);
    public static native boolean isTextureHandle(long assetRegistryHandle, long handle);
    public static native boolean isUsingTexture(long worldHandle, long entityHandle, long textureHandle);

    // camera
    public static native Camera getCamera(long worldHandle, String label);
    public static native Camera getAttachedCamera(long worldHandle, long entityHandle);
    public static native void setCamera(long worldHandle, Camera camera);

    // transformations
    public static native EntityTransform getTransform(long handle, long entityHandle);
    public static native void propagateTransform(long worldHandle, long id);
    public static native void setTransform(long worldHandle, long id, EntityTransform transform);

    // properties
    public static native String getStringProperty(long worldHandle, long entityHandle, String label);
    public static native int getIntProperty(long worldHandle, long entityHandle, String label);
    public static native long getLongProperty(long worldHandle, long entityHandle, String label);
    public static native double getFloatProperty(long worldHandle, long entityHandle, String label);
    public static native boolean getBoolProperty(long worldHandle, long entityHandle, String label);
    public static native float[] getVec3Property(long worldHandle, long entityHandle, String label);

    public static native void setStringProperty(long worldHandle, long entityHandle, String label, String value);
    public static native void setIntProperty(long worldHandle, long entityHandle, String label, int value);
    public static native void setLongProperty(long worldHandle, long entityHandle, String label, long value);
    public static native void setFloatProperty(long worldHandle, long entityHandle, String label, double value);
    public static native void setBoolProperty(long worldHandle, long entityHandle, String label, boolean value);
    public static native void setVec3Property(long worldHandle, long entityHandle, String label, float[] value);

    // input
    public static native void printInputState(long inputHandle);
    public static native boolean isKeyPressed(long inputHandle, int ordinal);
    public static native float[] getMousePosition(long inputHandle);
    public static native boolean isMouseButtonPressed(long inputHandle, int ordinal);
    public static native float[] getMouseDelta(long inputHandle);
    public static native boolean isCursorLocked(long inputHandle);
    public static native void setCursorLocked(long inputHandle, long graphicsHandle, boolean locked);
    public static native float[] getLastMousePos(long inputHandle);
    public static native boolean isCursorHidden(long inputHandle);
    public static native void setCursorHidden(long inputHandle, long graphicsHandle, boolean hidden);
    public static native String getLastErrorMsg(long assetHandle);
    public static native long getLastErrMsgPtr(long assetHandle);
    public static native String[] getAllTextures(long worldHandle, long entityHandle);
}
