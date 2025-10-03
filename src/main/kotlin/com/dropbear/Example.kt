package com.dropbear

import com.dropbear.input.Input
import com.dropbear.input.KeyCode
import com.dropbear.math.Vector3D

class Example : RunnableScript {
    override var engine: DropbearEngine = DropbearEngine()
    
    private var speed = 0.1
    
    override fun load() {
        println("Example script loaded!")
    }
    
    override fun update() {
        val transform = engine.getTransform()
        engine.getEntity("player")?.getTransform(engine)
        
        if (Input.isKeyPressed(KeyCode.W)) {
            transform.translate(Vector3D(0.0, 0.0, -speed))
        }
        if (Input.isKeyPressed(KeyCode.S)) {
            transform.translate(Vector3D(0.0, 0.0, speed))
        }
        if (Input.isKeyPressed(KeyCode.A)) {
            transform.translate(Vector3D(-speed, 0.0, 0.0))
        }
        if (Input.isKeyPressed(KeyCode.D)) {
            transform.translate(Vector3D(speed, 0.0, 0.0))
        }
        
        if (Input.isKeyPressed(KeyCode.SPACE)) {
            transform.translate(Vector3D(0.0, speed, 0.0))
        }
        if (Input.isKeyPressed(KeyCode.SHIFT)) {
            transform.translate(Vector3D(0.0, -speed, 0.0))
        }
    }
}
