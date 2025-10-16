@file:OptIn(ExperimentalForeignApi::class, ExperimentalNativeApi::class)
@file:Suppress("EXPECT_ACTUAL_CLASSIFIERS_ARE_IN_BETA_WARNING")

package com.dropbear.ffi

import com.dropbear.EntityId
import com.dropbear.ffi.generated.*
import com.dropbear.logging.Logger
import com.dropbear.math.Transform
import kotlinx.cinterop.*
import kotlin.experimental.ExperimentalNativeApi

actual class NativeEngine {
    private var worldHandle: COpaquePointer? = null

    @Suppress("unused") // called from jni
    fun init(handle: COpaquePointer?) {
        this.worldHandle = handle
        if (this.worldHandle == null) {
            Logger.info("NativeEngine: Error - Invalid world handle received!")
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
}