/// A component that is added to all entities to show all child entities
#[derive(Default, Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Parent(Vec<hecs::Entity>);

impl Parent {
    /// Creates a new parent component with the provided child entities.
    pub fn new(children: Vec<hecs::Entity>) -> Self {
        Self(children)
    }

    /// Returns an immutable view into the stored child entities.
    pub fn children(&self) -> &[hecs::Entity] {
        &self.0
    }

    /// Returns a mutable view into the stored child entities.
    pub fn children_mut(&mut self) -> &mut Vec<hecs::Entity> {
        &mut self.0
    }

    /// Adds a new child entity to this parent component.
    pub fn push(&mut self, child: hecs::Entity) {
        self.0.push(child);
    }

    /// Removes all children from this parent component.
    pub fn clear(&mut self) {
        self.0.clear();
    }

    /// Returns whether this parent does not track any child entities.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

/// Propagates transforms through the hierarchy, updating WorldTransform for all entities.
pub fn propagate_transforms(world: &mut hecs::World) {
    use dropbear_engine::entity::{LocalTransform, WorldTransform};

    // update root entities
    {
        let mut query = world
            .query::<(&LocalTransform, &mut WorldTransform)>()
            .without::<&Parent>();
        for (_, (local, world_transform)) in query.iter() {
            world_transform.sync_from_local(local);
        }
    }

    let mut parent_child_map: Vec<(hecs::Entity, Vec<hecs::Entity>)> = Vec::new();
    {
        let mut query = world.query::<&Parent>();
        for (entity, parent_comp) in query.iter() {
            if !parent_comp.is_empty() {
                parent_child_map.push((entity, parent_comp.children().to_vec()));
            }
        }
    }

    // update transform for all children of entities
    fn propagate_to_children(
        world: &mut hecs::World,
        parent_entity: hecs::Entity,
        children: &[hecs::Entity],
        parent_child_map: &[(hecs::Entity, Vec<hecs::Entity>)],
    ) {
        let parent_world_transform =
            if let Ok(mut query) = world.query_one::<&WorldTransform>(parent_entity) {
                if let Some(transform) = query.get() {
                    *transform
                } else {
                    return;
                }
            } else {
                return;
            };

        for &child_entity in children {
            if let Ok(mut child_query) =
                world.query_one::<(&LocalTransform, &mut WorldTransform)>(child_entity)
            {
                if let Some((local, world_transform)) = child_query.get() {
                    world_transform.update_from_parent(local, &parent_world_transform);
                }
            }

            if let Some((_, grandchildren)) =
                parent_child_map.iter().find(|(e, _)| *e == child_entity)
            {
                propagate_to_children(world, child_entity, grandchildren, parent_child_map);
            }
        }
    }

    for (parent_entity, children) in &parent_child_map {
        propagate_to_children(world, *parent_entity, children, &parent_child_map);
    }
}
