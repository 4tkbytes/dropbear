/// Get a component and execute a closure if it exists
///
/// # Usage
/// ```
/// use eucalyptus_core::with_component;
///
/// use eucalyptus_core::scene::SceneEntity;
///
/// struct Transform {
///     position: [f32; 3],
/// }
///
/// let scene_entity = SceneEntity::default();
///
/// with_component!(scene_entity, Transform, /*mut*/ |transform| {
///     transform.position.x += 1.0;
/// });
#[macro_export]
macro_rules! with_component {
    // immutable
    ($entity:expr, $comp_type:ty, $closure:expr) => {
        $crate::traits::SerializableComponent::
        if let Some(comp) = $entity.as_any().downcast_ref::<$comp_type>() {
            $closure(comp)
        }
    };

    // mutable
    ($entity:expr, $comp_type:ty, mut $closure:expr) => {
        if let Some(comp) = $entity.as_any_mut().downcast_mut()::<$comp_type>() {
            $closure(comp)
        }
    };
}

/// Try to get a component, or execute else block
///
/// # Usage
/// ```
/// use eucalyptus_core::if_component;
///
/// use eucalyptus_core::scene::SceneEntity;
///
/// struct Transform {
///     position: [f32; 3],
/// }
///
/// let scene_entity = SceneEntity::default();
///
/// if_component!(scene_entity, Transform, /*mut*/ |transform| {
///     let position = transform.position.get(0);
/// } else {
///     println!("No transform found");
/// });
#[macro_export]
macro_rules! if_component {
    ($entity:expr, $comp_type:ty, |$comp:ident| $then:block else $else:block) => {
        if let Some($comp) = $entity.as_any().downcast_ref::<$comp_type>() {
            $then
        } else {
            $else
        }
    };

    ($entity:expr, $comp_type:ty, |$comp:ident| $then:block) => {
        if let Some($comp) = $entity.as_any().downcast_ref::<$comp_type>() {
            $then
        }
    };

    ($entity:expr, $comp_type:ty, |mut $comp:ident| $then:block else $else:block) => {
        if let Some(mut $comp) =  $entity.as_any_mut().downcast_mut()::<$comp_type>() {
            $then
        } else {
            $else
        }
    };

    ($entity:expr, $comp_type:ty, |mut $comp:ident| $then:block) => {
        if let Some(mut $comp) = $entity.as_any_mut().downcast_mut()::<$comp_type>() {
            $then
        }
    };
}