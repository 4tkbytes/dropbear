// File to get an idea of what is generated
@file:OptIn(ExperimentalForeignApi::class, ExperimentalNativeApi::class)
import com.dropbear.DropbearEngine
import com.dropbear.EntityId
import com.dropbear.EntityRef
import com.dropbear.ProjectScriptingMetadata
import com.dropbear.ScriptRegistration
import com.dropbear.ffi.NativeEngine
import kotlinx.cinterop.COpaquePointer
import kotlinx.cinterop.ExperimentalForeignApi
import kotlin.experimental.ExperimentalNativeApi
import kotlin.native.CName

// import /* CLASS */

private val scriptInstances: List<ScriptRegistration> by lazy {
    Metadata().getScripts()
}

class Metadata: ProjectScriptingMetadata {
    override fun getScripts(): List<ScriptRegistration> {
        return listOf(
            ScriptRegistration(
                tags = listOf("player"),
                script = Player()
            ),

//            ScriptRegistration(
//                tags = /* TAGS */,
//                script = /* CLASS NAME */
//            ),

        )
    }
}

fun getDropbearEngine(worldPointer: COpaquePointer?, currentEntity: Long?): DropbearEngine {
    val nativeEngine = NativeEngine()
    nativeEngine.init(worldPointer)
    val dropbearEngine = DropbearEngine(nativeEngine, if (currentEntity == null) null else EntityRef(EntityId(currentEntity.toULong())))
    return dropbearEngine
}

@CName("dropbear_load")
fun loadScriptByTag(worldPointer: COpaquePointer?, currentEntity: Long?, tag: String?) {
    if (tag == null) return
    val scripts = scriptInstances.filter { it.tags.contains(tag) }
    val engine = getDropbearEngine(worldPointer, currentEntity)
    for (script in scripts) {
        script.script.load(engine)
    }
}

@CName("dropbear_update")
fun updateScriptByTag(worldPointer: COpaquePointer?, currentEntity: Long?, tag: String?, deltaTime: Double) {
    if (tag == null) return
    val scripts = scriptInstances.filter { it.tags.contains(tag) }
    val engine = getDropbearEngine(worldPointer, currentEntity)
    for (script in scripts) {
        script.script.update(engine, deltaTime.toFloat())
    }
}

@CName("dropbear_destroy")
fun destroyScriptByTag(worldPointer: COpaquePointer?, currentEntity: Long?, tag: String?) {
    if (tag == null) return
    val scripts = scriptInstances.filter { it.tags.contains(tag) }
    val engine = getDropbearEngine(worldPointer, currentEntity)
    for (script in scripts) {
        script.script.destroy(engine)
    }
}
