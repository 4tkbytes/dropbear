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
