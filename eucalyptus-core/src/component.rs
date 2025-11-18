/// Get a component and execute a closure if it exists
///
/// # Usage
/// ```
/// use eucalyptus_core::with_component;
///
/// with_component!(scene_entity, Transform, /*mut*/ |transform| {
///     transform.position.x += 1.0;
/// });
#[macro_export]
macro_rules! with_component {
    // immutable
    ($entity:expr, $comp_type:ty, $closure:expr) => {
        if let Some(comp) = $entity.get_component::<$comp_type>() {
            $closure(comp)
        }
    };

    // mutable
    ($entity:expr, $comp_type:ty, mut $closure:expr) => {
        if let Some(comp) = $entity.get_component_mut::<$comp_type>() {
            $closure(comp)
        }
    };
}

/// Get a component or return early from the function
///
/// # Usage
/// ```
/// use dropbear_engine::entity::MeshRenderer;
/// use eucalyptus_core::get_component;
///
/// fn process_entity(entity: &SceneEntity) {
///     let renderer = get_component!(entity, MeshRenderer);
///     println!("Processing mesh: {:?}", renderer.handle);
/// }
#[macro_export]
macro_rules! get_component {
    ($entity:expr, $comp_type:ty) => {
        match $entity.get_component::<$comp_type>() {
            Some(comp) => comp,
            None => return,
        }
    };

    ($entity:expr, $comp_type:ty, mut) => {
        match $entity.get_component_mut::<$comp_type>() {
            Some(comp) => comp,
            None => return,
        }
    };
}

/// Try to get a component, or execute else block
///
/// # Usage
/// ```
/// if_component!(scene_entity, Transform, |/*mut*/ transform| {
///     transform.position = Vec3::new(1.0, 2.0, 3.0);
/// } else {
///     println!("No transform found");
/// });
#[macro_export]
macro_rules! if_component {
    ($entity:expr, $comp_type:ty, |$comp:ident| $then:block else $else:block) => {
        if let Some($comp) = $entity.get_component::<$comp_type>() {
            $then
        } else {
            $else
        }
    };

    ($entity:expr, $comp_type:ty, |$comp:ident| $then:block) => {
        if let Some($comp) = $entity.get_component::<$comp_type>() {
            $then
        }
    };

    ($entity:expr, $comp_type:ty, |mut $comp:ident| $then:block else $else:block) => {
        if let Some(mut $comp) = $entity.get_component_mut::<$comp_type>() {
            $then
        } else {
            $else
        }
    };

    ($entity:expr, $comp_type:ty, |mut $comp:ident| $then:block) => {
        if let Some(mut $comp) = $entity.get_component_mut::<$comp_type>() {
            $then
        }
    };
}