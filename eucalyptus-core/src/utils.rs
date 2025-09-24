
use crate::states::Node;

pub const PROTO_TEXTURE: &[u8] = include_bytes!("../../resources/proto.png");

pub fn search_nodes_recursively<'a, F>(nodes: &'a [Node], matcher: &F, results: &mut Vec<&'a Node>)
where
    F: Fn(&Node) -> bool,
{
    for node in nodes {
        if matcher(node) {
            results.push(node);
        }
        match node {
            Node::File(_) => {}
            Node::Folder(folder) => {
                search_nodes_recursively(&folder.nodes, matcher, results);
            }
        }
    }
}

/// Progress events for project creation
pub enum ProjectProgress {
    Step {
        _progress: f32,
        _message: String,
    },
    #[allow(dead_code)] // idk why its giving me this warning :(
    Error(String),
    Done,
}

#[derive(Clone)]
pub enum ViewportMode {
    None,
    CameraMove,
    Gizmo,
}
