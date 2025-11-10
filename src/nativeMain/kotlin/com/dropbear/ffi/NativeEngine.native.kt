@file:OptIn(ExperimentalForeignApi::class, ExperimentalNativeApi::class)
@file:Suppress("EXPECT_ACTUAL_CLASSIFIERS_ARE_IN_BETA_WARNING")

// guys how the fuck do i get rid of the type error messages they dont show
// any errors in the compiler, only in the IDE. it pmo so much.
// note: there are no resultant errors or issues, just annoying. thats really it.

package com.dropbear.ffi

import com.dropbear.Camera
import com.dropbear.EntityId
import com.dropbear.asset.TextureHandle
import com.dropbear.exception.DropbearNativeException
import com.dropbear.exceptionOnError
import com.dropbear.ffi.generated.*
import com.dropbear.input.KeyCode
import com.dropbear.input.MouseButton
import com.dropbear.input.MouseButtonCodes
import com.dropbear.logging.Logger
import com.dropbear.math.Transform
import com.dropbear.math.Vector2D
import kotlinx.cinterop.*
import kotlin.experimental.ExperimentalNativeApi

actual class NativeEngine {
    private var worldHandle: COpaquePointer? = null
    private var inputHandle: COpaquePointer? = null
    private var graphicsHandle: COpaquePointer? = null
    private var assetHandle: COpaquePointer? = null

    @Suppress("unused")
    fun init(worldHandle: COpaquePointer?, inputHandle: COpaquePointer?, graphicsHandle: COpaquePointer?, assetHandle: COpaquePointer?) {
        this.worldHandle = worldHandle
        this.inputHandle = inputHandle
        this.graphicsHandle = graphicsHandle
        this.assetHandle = assetHandle
        if (this.worldHandle == null) {
            Logger.error("NativeEngine: Error - Invalid world handle received!")
        }
        if (this.inputHandle == null) {
            Logger.error("NativeEngine: Error - Invalid input handle received!")
        }
        if (this.graphicsHandle == null) {
            Logger.error("NativeEngine: Error - Invalid graphics handle received!")
        }
        if (this.assetHandle == null) {
            Logger.error("NativeEngine: Error - Invalid asset handle received!")
        }
    }

    actual fun getEntity(label: String): Long? {
        val world = worldHandle ?: return null
        memScoped {
            val outEntity = alloc<LongVar>()
            val result = dropbear_get_entity(
                label = label,
                world_ptr = world.reinterpret(),
                out_entity = outEntity.ptr
            )
            return if (result == 0) outEntity.value else null
        }
    }

    actual fun getTransform(entityId: EntityId): Transform? {
        val world = worldHandle ?: return null
        memScoped {
            val outTransform = alloc<NativeTransform>()
            val result = dropbear_get_transform(
                world_ptr = world.reinterpret(),
                entity_id = entityId.id,
                out_transform = outTransform.ptr
            )
            if (result == 0) {
                return Transform(
                    position = com.dropbear.math.Vector3D(
                        outTransform.position_x,
                        outTransform.position_y,
                        outTransform.position_z
                    ),
                    rotation = com.dropbear.math.QuaternionD(
                        outTransform.rotation_x,
                        outTransform.rotation_y,
                        outTransform.rotation_z,
                        outTransform.rotation_w
                    ),
                    scale = com.dropbear.math.Vector3D(
                        outTransform.scale_x,
                        outTransform.scale_y,
                        outTransform.scale_z
                    )
                )
            } else {
                return null
            }
        }
    }

    actual fun setTransform(entityId: EntityId, transform: Transform) {
        val world = worldHandle ?: return
        memScoped {
            val nativeTransform = cValue<NativeTransform> {
                position_x = transform.position.x
                position_y = transform.position.y
                position_z = transform.position.z

                rotation_w = transform.rotation.w
                rotation_x = transform.rotation.x
                rotation_y = transform.rotation.y
                rotation_z = transform.rotation.z

                scale_x = transform.scale.x
                scale_y = transform.scale.y
                scale_z = transform.scale.z
            }

            dropbear_set_transform(
                world_ptr = world.reinterpret(),
                entity_id = entityId.id,
                transform = nativeTransform
            )
        }
    }

    actual fun printInputState() {
        val input = inputHandle ?: return
        dropbear_print_input_state(input_state_ptr = input.reinterpret())
    }

    actual fun isKeyPressed(key: KeyCode): Boolean {
        val input = inputHandle ?: return false
        memScoped {
            val out = alloc<IntVar>()
            dropbear_is_key_pressed(
                input.reinterpret(),
                key.ordinal,
                out.ptr
            )
            return out.value != 0
        }
    }

    actual fun getMousePosition(): Vector2D? {
        val input = inputHandle ?: return null
        memScoped {
            val xVar = alloc<FloatVar>()
            val yVar = alloc<FloatVar>()

            val result = dropbear_get_mouse_position(
                input.reinterpret(),
                xVar.ptr,
                yVar.ptr
            )

            if (result == 0) {
                val x = xVar.value.toDouble()
                val y = yVar.value.toDouble()
                return Vector2D(x, y)
            } else {
                if (exceptionOnError) {
                    throw DropbearNativeException("getMousePosition failed with code: $result")
                } else {
                    println("getMousePosition failed with code: $result")
                    return null
                }
            }
        }
    }

    actual fun isMouseButtonPressed(button: MouseButton): Boolean {
        val buttonCode: Int = when (button) {
            MouseButton.Left -> MouseButtonCodes.LEFT
            MouseButton.Right -> MouseButtonCodes.RIGHT
            MouseButton.Middle -> MouseButtonCodes.MIDDLE
            MouseButton.Back -> MouseButtonCodes.BACK
            MouseButton.Forward -> MouseButtonCodes.FORWARD
            is MouseButton.Other -> button.value
        }

        val input = inputHandle ?: return false

        memScoped {
            val pressedVar = alloc<IntVar>()

            val result = dropbear_is_mouse_button_pressed(
                input.reinterpret(),
                buttonCode,
                pressedVar.ptr
            )

            if (result == 0) {
                return pressedVar.value != 0
            } else {
                if (exceptionOnError) {
                    throw DropbearNativeException("isMouseButtonPressed failed with code: $result")
                } else {
                    println("isMouseButtonPressed failed with code: $result")
                    return false
                }
            }
        }
    }

    actual fun getMouseDelta(): Vector2D? {
        val input = inputHandle ?: return null
        memScoped {
            val deltaXVar = alloc<FloatVar>()
            val deltaYVar = alloc<FloatVar>()

            val result = dropbear_get_mouse_delta(
                input.reinterpret(),
                deltaXVar.ptr,
                deltaYVar.ptr
            )

            if (result == 0) {
                val deltaX = deltaXVar.value.toDouble()
                val deltaY = deltaYVar.value.toDouble()
                return Vector2D(deltaX, deltaY)
            } else {
                if (exceptionOnError) {
                    throw DropbearNativeException("getMouseDelta failed with code: $result")
                } else {
                    println("getMouseDelta failed with code: $result")
                    return null
                }
            }
        }
    }

    actual fun isCursorLocked(): Boolean {
        val input = inputHandle ?: return false
        memScoped {
            val lockedVar = alloc<IntVar>()

            val result = dropbear_is_cursor_locked(
                input.reinterpret(),
                lockedVar.ptr
            )

            if (result == 0) {
                return lockedVar.value != 0
            } else {
                if (exceptionOnError) {
                    throw DropbearNativeException("isCursorLocked failed with code: $result")
                } else {
                    println("isCursorLocked failed with code: $result")
                    return false
                }
            }
        }
    }

    actual fun setCursorLocked(locked: Boolean) {
        val lockedInt = if (locked) 1 else 0
        val input = inputHandle ?: return
        val graphics = graphicsHandle ?: return

        val result = dropbear_set_cursor_locked(
            graphics.reinterpret(),
            input.reinterpret(),
            lockedInt
        )

        if (result != 0) {
            if (exceptionOnError) {
                throw DropbearNativeException("setCursorLocked failed with code: $result")
            } else {
                println("setCursorLocked failed with code: $result")
            }
        }
    }

    actual fun getLastMousePos(): Vector2D? {
        val input = inputHandle ?: return null
        memScoped {
            val xVar = alloc<FloatVar>()
            val yVar = alloc<FloatVar>()

            val result = dropbear_get_last_mouse_pos(
                input.reinterpret(),
                xVar.ptr,
                yVar.ptr
            )

            if (result == 0) {
                val x = xVar.value.toDouble()
                val y = yVar.value.toDouble()
                return Vector2D(x, y)
            } else {
                if (exceptionOnError) {
                    throw DropbearNativeException("getLastMousePos failed with code: $result")
                } else {
                    println("getLastMousePos failed with code: $result")
                    return null
                }
            }
        }
    }

    actual fun isCursorHidden(): Boolean {
        val input = inputHandle ?: return false
        memScoped {
            val hiddenVar = alloc<IntVar>()

            val result = dropbear_is_cursor_hidden(
                input.reinterpret(),
                hiddenVar.ptr
            )

            if (result == 0) {
                return hiddenVar.value != 0
            } else {
                if (exceptionOnError) {
                    throw DropbearNativeException("isCursorHidden failed with code: $result")
                } else {
                    println("isCursorHidden failed with code: $result")
                    return false
                }
            }
        }
    }

    actual fun setCursorHidden(hidden: Boolean) {
        val hiddenInt = if (hidden) 1 else 0
        val input = inputHandle ?: return
        val graphics = graphicsHandle ?: return

        val result = dropbear_set_cursor_hidden(
            graphics.reinterpret(),
            input.reinterpret(),
            hiddenInt
        )

        if (result != 0) {
            if (exceptionOnError) {
                throw DropbearNativeException("setCursorHidden failed with code: $result")
            } else {
                println("setCursorHidden failed with code: $result")
            }
        }
    }

    actual fun getStringProperty(entityHandle: Long, label: String): String? {
        val world = worldHandle ?: return null
        memScoped {
            val bufferSize = 256
            val output = allocArray<ByteVar>(bufferSize)

            // warning: this could potentially cause a buffer overflow idk
            val result = dropbear_get_string_property(
                world.reinterpret(),
                entityHandle,
                label,
                output,
                bufferSize
            )

            if (result == 0) {
                val string = output.toKString()
                return string
            } else {
                if (exceptionOnError) {
                    throw DropbearNativeException("getStringProperty failed with code: $result")
                } else {
                    println("getStringProperty failed with code: $result")
                    return null
                }
            }
        }
    }

    actual fun getIntProperty(entityHandle: Long, label: String): Int? {
        val world = worldHandle ?: return null
        memScoped {
            val output = alloc<IntVar>()

            val result = dropbear_get_int_property(
                world.reinterpret(),
                entityHandle,
                label,
                output.ptr,
            )

            if (result == 0) {
                val string = output.value
                return string
            } else {
                if (exceptionOnError) {
                    throw DropbearNativeException("getIntProperty failed with code: $result")
                } else {
                    println("getIntProperty failed with code: $result")
                    return null
                }
            }
        }
    }

    actual fun getLongProperty(entityHandle: Long, label: String): Long? {
        val world = worldHandle ?: return null
        memScoped {
            val output = alloc<LongVar>()

            val result = dropbear_get_long_property(
                world.reinterpret(),
                entityHandle,
                label,
                output.ptr
            )

            if (result == 0) {
                return output.value
            } else {
                if (exceptionOnError) {
                    throw DropbearNativeException("getLongProperty failed with code: $result")
                } else {
                    println("getLongProperty failed with code: $result")
                    return null
                }
            }
        }
    }

    actual fun getFloatProperty(entityHandle: Long, label: String): Float? {
        val world = worldHandle ?: return null
        memScoped {
            val output = alloc<FloatVar>()

            val result = dropbear_get_float_property(
                world.reinterpret(),
                entityHandle,
                label,
                output.ptr
            )

            if (result == 0) {
                return output.value
            } else {
                if (exceptionOnError) {
                    throw DropbearNativeException("getFloatProperty failed with code: $result")
                } else {
                    println("getFloatProperty failed with code: $result")
                    return null
                }
            }
        }
    }

    actual fun getDoubleProperty(entityHandle: Long, label: String): Double? {
        val world = worldHandle ?: return null
        memScoped {
            val output = alloc<DoubleVar>()

            val result = dropbear_get_double_property(
                world.reinterpret(),
                entityHandle,
                label,
                output.ptr
            )

            if (result == 0) {
                return output.value
            } else {
                if (exceptionOnError) {
                    throw DropbearNativeException("getDoubleProperty failed with code: $result")
                } else {
                    println("getDoubleProperty failed with code: $result")
                    return null
                }
            }
        }
    }

    actual fun getBoolProperty(entityHandle: Long, label: String): Boolean? {
        val world = worldHandle ?: return null
        memScoped {
            val output = alloc<IntVar>()

            val result = dropbear_get_bool_property(
                world.reinterpret(),
                entityHandle,
                label,
                output.ptr
            )

            if (result == 0) {
                return output.value != 0
            } else {
                if (exceptionOnError) {
                    throw DropbearNativeException("getBoolProperty failed with code: $result")
                } else {
                    println("getBoolProperty failed with code: $result")
                    return null
                }
            }
        }
    }

    actual fun getVec3Property(entityHandle: Long, label: String): FloatArray? {
        val world = worldHandle ?: return null
        memScoped {
            val outX = alloc<FloatVar>()
            val outY = alloc<FloatVar>()
            val outZ = alloc<FloatVar>()

            val result = dropbear_get_vec3_property(
                world.reinterpret(),
                entityHandle,
                label,
                outX.ptr,
                outY.ptr,
                outZ.ptr
            )

            if (result == 0) {
                return floatArrayOf(outX.value, outY.value, outZ.value)
            } else {
                if (exceptionOnError) {
                    throw DropbearNativeException("getVec3Property failed with code: $result")
                } else {
                    println("getVec3Property failed with code: $result")
                    return null
                }
            }
        }
    }

    actual fun setStringProperty(entityHandle: Long, label: String, value: String) {
        val world = worldHandle ?: return

        val result = dropbear_set_string_property(
            world.reinterpret(),
            entityHandle,
            label,
            value
        )

        if (result != 0) {
            if (exceptionOnError) {
                throw DropbearNativeException("setStringProperty failed with code: $result")
            } else {
                println("setStringProperty failed with code: $result")
            }
        }
    }

    actual fun setIntProperty(entityHandle: Long, label: String, value: Int) {
        val world = worldHandle ?: return

        val result = dropbear_set_int_property(
            world.reinterpret(),
            entityHandle,
            label,
            value
        )

        if (result != 0) {
            if (exceptionOnError) {
                throw DropbearNativeException("setIntProperty failed with code: $result")
            } else {
                println("setIntProperty failed with code: $result")
            }
        }
    }

    actual fun setLongProperty(entityHandle: Long, label: String, value: Long) {
        val world = worldHandle ?: return

        val result = dropbear_set_long_property(
            world.reinterpret(),
            entityHandle,
            label,
            value
        )

        if (result != 0) {
            if (exceptionOnError) {
                throw DropbearNativeException("setLongProperty failed with code: $result")
            } else {
                println("setLongProperty failed with code: $result")
            }
        }
    }

    actual fun setFloatProperty(entityHandle: Long, label: String, value: Double) {
        val world = worldHandle ?: return

        val result = dropbear_set_float_property(
            world.reinterpret(),
            entityHandle,
            label,
            value.toFloat()
        )

        if (result != 0) {
            if (exceptionOnError) {
                throw DropbearNativeException("setFloatProperty failed with code: $result")
            } else {
                println("setFloatProperty failed with code: $result")
            }
        }
    }

    actual fun setBoolProperty(entityHandle: Long, label: String, value: Boolean) {
        val world = worldHandle ?: return
        val intValue = if (value) 1 else 0

        val result = dropbear_set_bool_property(
            world.reinterpret(),
            entityHandle,
            label,
            intValue
        )

        if (result != 0) {
            if (exceptionOnError) {
                throw DropbearNativeException("setBoolProperty failed with code: $result")
            } else {
                println("setBoolProperty failed with code: $result")
            }
        }
    }

    actual fun setVec3Property(entityHandle: Long, label: String, value: FloatArray) {
        val world = worldHandle ?: return

        if (value.size < 3) {
            if (exceptionOnError) {
                throw DropbearNativeException("setVec3Property: FloatArray must have at least 3 elements")
            } else {
                println("setVec3Property: FloatArray must have at least 3 elements")
                return
            }
        }

        val result = dropbear_set_vec3_property(
            world.reinterpret(),
            entityHandle,
            label,
            value[0],
            value[1],
            value[2]
        )

        if (result != 0) {
            if (exceptionOnError) {
                throw DropbearNativeException("setVec3Property failed with code: $result")
            } else {
                println("setVec3Property failed with code: $result")
            }
        }
    }

    actual fun getCamera(label: String): Camera? {
        val world = worldHandle ?: return null
        memScoped {
            val outCamera = alloc<NativeCamera>()

            val result = dropbear_get_camera(
                world.reinterpret(),
                label,
                outCamera.ptr
            )

            if (result == 0) {
                return Camera(
                    label = outCamera.label?.toKString() ?: "",
                    id = EntityId(outCamera.entity_id.toLong()),
                    eye = com.dropbear.math.Vector3D(
                        outCamera.eye.x.toDouble(),
                        outCamera.eye.y.toDouble(),
                        outCamera.eye.z.toDouble()
                    ),
                    target = com.dropbear.math.Vector3D(
                        outCamera.target.x.toDouble(),
                        outCamera.target.y.toDouble(),
                        outCamera.target.z.toDouble()
                    ),
                    up = com.dropbear.math.Vector3D(
                        outCamera.up.x.toDouble(),
                        outCamera.up.y.toDouble(),
                        outCamera.up.z.toDouble()
                    ),
                    aspect = outCamera.aspect,
                    fov_y = outCamera.fov_y,
                    znear = outCamera.znear,
                    zfar = outCamera.zfar,
                    yaw = outCamera.yaw,
                    pitch = outCamera.pitch,
                    speed = outCamera.speed,
                    sensitivity = outCamera.sensitivity
                )
            } else {
                if (exceptionOnError) {
                    throw DropbearNativeException("getCamera failed with code: $result")
                } else {
                    println("getCamera failed with code: $result")
                    return null
                }
            }
        }
    }

    actual fun getAttachedCamera(entityId: EntityId): Camera? {
        val world = worldHandle ?: return null
        memScoped {
            val outCamera = alloc<NativeCamera>()

            val result = dropbear_get_attached_camera(
                world.reinterpret(),
                entityId.id,
                outCamera.ptr
            )

            if (result == 0) {
                return Camera(
                    label = outCamera.label?.toKString() ?: "",
                    id = EntityId(outCamera.entity_id.toLong()),
                    eye = com.dropbear.math.Vector3D(
                        outCamera.eye.x.toDouble(),
                        outCamera.eye.y.toDouble(),
                        outCamera.eye.z.toDouble()
                    ),
                    target = com.dropbear.math.Vector3D(
                        outCamera.target.x.toDouble(),
                        outCamera.target.y.toDouble(),
                        outCamera.target.z.toDouble()
                    ),
                    up = com.dropbear.math.Vector3D(
                        outCamera.up.x.toDouble(),
                        outCamera.up.y.toDouble(),
                        outCamera.up.z.toDouble()
                    ),
                    aspect = outCamera.aspect,
                    fov_y = outCamera.fov_y,
                    znear = outCamera.znear,
                    zfar = outCamera.zfar,
                    yaw = outCamera.yaw,
                    pitch = outCamera.pitch,
                    speed = outCamera.speed,
                    sensitivity = outCamera.sensitivity
                )
            } else {
                if (exceptionOnError) {
                    throw DropbearNativeException("getAttachedCamera failed with code: $result")
                } else {
                    println("getAttachedCamera failed with code: $result")
                    return null
                }
            }
        }
    }

    actual fun setCamera(camera: Camera) {
        val world = worldHandle ?: return
        memScoped {
            val nativeCamera = cValue<NativeCamera> {
                label = camera.label.cstr.ptr
                entity_id = camera.id.id

                eye.x = camera.eye.x.toFloat()
                eye.y = camera.eye.y.toFloat()
                eye.z = camera.eye.z.toFloat()

                target.x = camera.target.x.toFloat()
                target.y = camera.target.y.toFloat()
                target.z = camera.target.z.toFloat()

                up.x = camera.up.x.toFloat()
                up.y = camera.up.y.toFloat()
                up.z = camera.up.z.toFloat()

                aspect = camera.aspect
                fov_y = camera.fov_y
                znear = camera.znear
                zfar = camera.zfar

                yaw = camera.yaw
                pitch = camera.pitch
                speed = camera.speed
                sensitivity = camera.sensitivity
            }

            val result = dropbear_set_camera(
                world.reinterpret(),
                nativeCamera.ptr
            )

            if (result != 0) {
                if (exceptionOnError) {
                    throw DropbearNativeException("setCamera failed with code: $result")
                } else {
                    println("setCamera failed with code: $result")
                }
            }
        }
    }

    actual fun getModel(entityHandle: Long): Long? {
        TODO("Not yet implemented")
    }

    actual fun setModel(entityHandle: Long, modelHandle: Long) {
    }

    actual fun getTexture(entityHandle: Long, name: String): Long? {
        TODO("Not yet implemented")
    }

    actual fun isUsingModel(entityHandle: Long, modelHandle: Long): Boolean {
        TODO("Not yet implemented")
    }

    actual fun isUsingTexture(entityHandle: Long, name: String): Boolean {
        TODO("Not yet implemented")
    }

    actual fun getAsset(eucaURI: String): Long? {
        TODO("Not yet implemented")
    }

    actual fun isModelHandle(id: Long): Boolean {
        TODO("Not yet implemented")
    }

    actual fun isTextureHandle(id: Long): Boolean {
        TODO("Not yet implemented")
    }

    actual fun setTextureOverride(entityHandle: Long, oldMaterialName: String, newTextureHandle: TextureHandle) {
    }

    actual fun getTextureName(textureHandle: Long): String? {
        TODO("Not yet implemented")
    }
}