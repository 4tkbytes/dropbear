@file:OptIn(ExperimentalForeignApi::class, ExperimentalNativeApi::class)
@file:Suppress("EXPECT_ACTUAL_CLASSIFIERS_ARE_IN_BETA_WARNING")

package com.dropbear.ffi

import com.dropbear.EntityId
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

    @Suppress("unused")
    fun init(worldHandle: COpaquePointer?, inputHandle: COpaquePointer?) {
        this.worldHandle = worldHandle
        this.inputHandle = inputHandle
        if (this.worldHandle == null) {
            Logger.error("NativeEngine: Error - Invalid world handle received!")
        }
        if (this.inputHandle == null) {
            Logger.error("NativeEngine: Error - Invalid input handle received!")
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
                println("getMousePosition failed with code: $result")
                return null
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
                println("isMouseButtonPressed failed with code: $result")
                return false
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
                println("getMouseDelta failed with code: $result")
                return null
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
                println("isCursorLocked failed with code: $result")
                return false
            }
        }
    }

    actual fun setCursorLocked(locked: Boolean) {
        val lockedInt = if (locked) 1 else 0
        val input = inputHandle ?: return

        val result = dropbear_set_cursor_locked(
            input.reinterpret(),
            lockedInt
        )

        if (result != 0) {
            println("setCursorLocked failed with code: $result")
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
                println("getLastMousePos failed with code: $result")
                return null
            }
        }
    }

    actual fun getStringProperty(entityHandle: Long, label: String): String? {
        TODO("Not yet implemented")
    }

    actual fun getIntProperty(entityHandle: Long, label: String): Int? {
        TODO("Not yet implemented")
    }

    actual fun getLongProperty(entityHandle: Long, label: String): Long? {
        TODO("Not yet implemented")
    }

    actual fun getFloatProperty(entityHandle: Long, label: String): Float? {
        TODO("Not yet implemented")
    }

    actual fun getDoubleProperty(entityHandle: Long, label: String): Double? {
        TODO("Not yet implemented")
    }

    actual fun getBoolProperty(entityHandle: Long, label: String): Boolean? {
        TODO("Not yet implemented")
    }

    actual fun getVec3Property(entityHandle: Long, label: String): FloatArray? {
        TODO("Not yet implemented")
    }

    actual fun setStringProperty(entityHandle: Long, label: String, value: String) {
    }

    actual fun setIntProperty(entityHandle: Long, label: String, value: Int) {
    }

    actual fun setLongProperty(entityHandle: Long, label: String, value: Long) {
    }

    actual fun setFloatProperty(entityHandle: Long, label: String, value: Double) {
    }

    actual fun setBoolProperty(entityHandle: Long, label: String, value: Boolean) {
    }

    actual fun setVec3Property(entityHandle: Long, label: String, value: FloatArray) {
    }
}