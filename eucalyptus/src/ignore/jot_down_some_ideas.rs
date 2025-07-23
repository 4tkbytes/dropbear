// NOTE TO ANYONE:
// DO NOT INCLUDE THIS IN THE MODULE. THIS IS NOT SUPPOSED TO
// HAVE ANY SORT OF ANALYSING FROM rust-analyzer SO DO NOT ADD IT
// TO A MODULE. THIS IS PURELY FOR DRAWING UP IDEAS AND STORING IT FOR LATER

pub enum ItemType {
    Player(Player),
    Light(Light),
    Entity(Entity),
}

// technically the root node of everything.
pub struct Assets {
    nodes: Vec<Nodes>,
}

// node can either be file or folder
pub enum Node {
    File(File),
    Folder(Folder),
}

// File is only a file, but it derives nodes
pub struct File {
    pub name: String,
    pub path: PathBuf, // can be just a reference from the root node
                       // contents/metadata
}

// Folder can contain either a file or another folder, but it derives nodes
pub struct Folder {
    name: String,
    nodes: Vec<Nodes>,
}

pub enum LightType {
    Directional,
    Point,
    Spot,
    // more to come
}

pub struct Player {
    camera: Camera,
    model: Model,
    physics: Physics,
    logic: Logic,
}

impl Player {
    fn move_forward() {
        camera.move_forward();
        model.move_forward();
        physics.move_forward();
    }
}

pub struct Light {
    logic: Logic,
}

pub struct Entity {
    model: Model,
    physics: Physics,
    logic: Logic,
}

pub enum EditorState {}

pub struct ProjectConfig {
    project_name: String,
    project_path: String,
}
