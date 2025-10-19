@file:OptIn(ExperimentalForeignApi::class, ExperimentalNativeApi::class)
@file:Suppress("EXPECT_ACTUAL_CLASSIFIERS_ARE_IN_BETA_WARNING")

package com.dropbear.ffi

import com.dropbear.EntityId
import com.dropbear.ffi.generated.*
import com.dropbear.input.KeyCode
import com.dropbear.logging.Logger
import com.dropbear.math.Transform
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
}