import com.dropbear.DropbearEngine
import com.dropbear.EntityId
import com.dropbear.System
import com.dropbear.ProjectScriptingMetadata
import com.dropbear.ScriptRegistration

fun playerMovement(engine: DropbearEngine, entityId: EntityId, deltaTime: Double) {

}

class Player: System {
    override fun update(engine: DropbearEngine, entityId: EntityId, deltaTime: Float) {
        TODO("Not yet implemented")
    }
}

class Metadata : ProjectScriptingMetadata {
    override fun getScripts(): List<ScriptRegistration> {
        return listOf (
            ScriptRegistration(
                tags = listOf("player", "movement"),
                script = Player()
            ),
        )
    }
}