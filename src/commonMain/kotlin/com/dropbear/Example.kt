import com.dropbear.DropbearEngine
import com.dropbear.EntityId
import com.dropbear.System
import com.dropbear.ProjectScriptingMetadata
import com.dropbear.ScriptRegistration

fun playerMovement(engine: DropbearEngine, entityId: EntityId, deltaTime: Double) {

}

class Player: System {
    override fun load(engine: DropbearEngine) {
        TODO("Not yet implemented")
    }

    override fun update(engine: DropbearEngine, deltaTime: Float) {
        TODO("Not yet implemented")
    }

    override fun destroy(engine: DropbearEngine) {
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

fun getProjectScriptMetadata(): ProjectScriptingMetadata = Metadata()