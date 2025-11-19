package com.dropbear

import com.dropbear.math.Transform

/**
 * A component that contains the local and world [Transform] of an entity.
 */
class EntityTransform(var local: Transform, var world: Transform)