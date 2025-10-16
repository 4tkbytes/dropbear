import com.dropbear.DropbearEngine
import com.dropbear.Runnable
import com.dropbear.System

@Runnable
class Example: System() {
    override fun load(engine: DropbearEngine) {
        val entity = engine.getEntity("example")
        val transform = entity?.getTransform()
        transform?.position?.x = 10.0
        entity?.setTransform(transform)
    }

    override fun update(engine: DropbearEngine, deltaTime: Float) {
    }

    override fun destroy(engine: DropbearEngine) {

    }
}