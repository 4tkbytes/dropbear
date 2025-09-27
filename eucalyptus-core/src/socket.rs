use anyhow::{Context, Result};
use hecs::{Entity, World};
use serde::{Deserialize, Serialize};
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::net::{TcpListener, TcpStream};
use std::thread;
use tokio::sync::{mpsc, oneshot};

use crate::input::InputState;
use crate::states::ScriptComponent;
use dropbear_engine::entity::Transform;
use glam::{Vec2, Vec3};

pub const DEFAULT_PORT: u16 = 7878;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EngineRequest {
    // Entity operations
    GetEntityInfo { id: u64 },
    GetEntityTransform { id: u64 },
    SetEntityTransform { id: u64, transform: TransformData },
    GetEntityComponent { id: u64, component_type: String },
    
    // Scene operations
    GetSceneInfo,
    GetAllEntities,
    GetEntitiesByLabel { label: String },
    CreateEntity { label: Option<String> },
    DeleteEntity { id: u64 },
    
    // Input operations
    IsKeyPressed { key: String },
    GetMousePosition,
    GetMouseDelta,
    
    // System operations
    GetDeltaTime,
    Ping,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EngineResponse {
    // Entity responses
    EntityInfo {
        id: u64,
        label: Option<String>,
        transform: Option<TransformData>,
        components: Vec<String>,
    },
    EntityTransform(TransformData),
    EntityComponent {
        component_type: String,
        data: serde_json::Value,
    },
    
    // Scene responses
    SceneInfo {
        entity_count: usize,
        entities: Vec<EntityInfo>,
    },
    EntityList(Vec<EntityInfo>),
    EntityCreated { id: u64 },
    
    // Input responses
    KeyPressed(bool),
    MousePosition(Vec2),
    MouseDelta(Vec2),
    
    // System responses
    DeltaTime(f32),
    Pong,
    
    // Error responses
    Error { message: String },
    Success,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityInfo {
    pub id: u64,
    pub label: Option<String>,
    pub transform: Option<TransformData>,
    pub components: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransformData {
    pub position: Vec3,
    pub rotation: Vec3, // Euler angles in degrees
    pub scale: Vec3,
}

impl From<Transform> for TransformData {
    fn from(transform: Transform) -> Self {
        let (axis, angle) = transform.rotation.to_axis_angle();
        let rotation = (axis * angle.to_degrees()).as_vec3();
        
        Self {
            position: transform.position.as_vec3(),
            rotation,
            scale: transform.scale.as_vec3(),
        }
    }
}

impl From<TransformData> for Transform {
    fn from(data: TransformData) -> Self {
        use glam::{DQuat, DVec3};
        let rotation = DQuat::from_euler(
            glam::EulerRot::XYZ,
            data.rotation.x.to_radians() as f64,
            data.rotation.y.to_radians() as f64,
            data.rotation.z.to_radians() as f64,
        );
        
        Transform {
            position: DVec3::new(data.position.x as f64, data.position.y as f64, data.position.z as f64),
            rotation,
            scale: DVec3::new(data.scale.x as f64, data.scale.y as f64, data.scale.z as f64),
        }
    }
}

/// Request to be processed by the main thread
#[derive(Debug)]
pub struct SocketRequest {
    pub request: EngineRequest,
    pub response_tx: oneshot::Sender<EngineResponse>,
}

/// Socket server that runs on a separate thread and communicates via channels
pub struct SocketServer {
    port: u16,
    request_tx: mpsc::UnboundedSender<SocketRequest>,
}

impl SocketServer {
    /// Create a new socket server
    /// Returns the server and a receiver for processing requests on the main thread
    pub fn new(port: u16) -> (Self, mpsc::UnboundedReceiver<SocketRequest>) {
        let (request_tx, request_rx) = mpsc::unbounded_channel();
        
        let server = Self {
            port,
            request_tx,
        };
        
        (server, request_rx)
    }

    /// Start the socket server in a background thread
    pub fn start(self) -> Result<()> {
        let addr = format!("127.0.0.1:{}", self.port);
        let listener = TcpListener::bind(&addr)
            .with_context(|| format!("Failed to bind to {}", addr))?;
        
        log::info!("Socket server listening on {}", addr);
        
        thread::spawn(move || {
            for stream in listener.incoming() {
                match stream {
                    Ok(stream) => {
                        let request_tx = self.request_tx.clone();
                        thread::spawn(move || {
                            if let Err(e) = handle_client(stream, request_tx) {
                                log::error!("Client error: {}", e);
                            }
                        });
                    }
                    Err(e) => log::error!("Connection failed: {}", e),
                }
            }
        });
        
        Ok(())
    }
}

/// Handle a single client connection
fn handle_client(
    stream: TcpStream,
    request_tx: mpsc::UnboundedSender<SocketRequest>,
) -> Result<()> {
    let mut reader = BufReader::new(&stream);
    let mut writer = BufWriter::new(&stream);
    
    loop {
        let mut line = String::new();
        match reader.read_line(&mut line) {
            Ok(0) => break, // Client disconnected
            Ok(_) => {
                let request: EngineRequest = serde_json::from_str(&line.trim())
                    .context("Failed to parse request")?;
                
                // Create oneshot channel for response
                let (response_tx, response_rx) = oneshot::channel();
                
                // Send request to main thread
                if request_tx.send(SocketRequest { request, response_tx }).is_err() {
                    log::error!("Failed to send request to main thread");
                    break;
                }
                
                // Wait for response from main thread
                match response_rx.blocking_recv() {
                    Ok(response) => {
                        let response_json = serde_json::to_string(&response)?;
                        writeln!(writer, "{}", response_json)?;
                        writer.flush()?;
                    }
                    Err(_) => {
                        log::error!("Failed to receive response from main thread");
                        break;
                    }
                }
            }
            Err(e) => {
                log::error!("Failed to read from client: {}", e);
                break;
            }
        }
    }
    
    Ok(())
}

/// Handler for processing requests on the main thread
/// This should be called from your main game loop
pub struct SocketRequestHandler;

impl SocketRequestHandler {
    /// Process pending socket requests
    /// Call this from your main thread/game loop
    pub fn process_requests(
        request_rx: &mut mpsc::UnboundedReceiver<SocketRequest>,
        world: &mut World,
        input_state: &InputState,
        delta_time: f32,
    ) {
        // Process all pending requests without blocking
        while let Ok(socket_request) = request_rx.try_recv() {
            let response = Self::handle_request(
                socket_request.request,
                world,
                input_state,
                delta_time,
            );
            
            // Send response back to socket thread
            let _ = socket_request.response_tx.send(response);
        }
    }
    
    fn handle_request(
        request: EngineRequest,
        world: &mut World,
        input_state: &InputState,
        delta_time: f32,
    ) -> EngineResponse {
        match request {
            EngineRequest::Ping => EngineResponse::Pong,
            
            EngineRequest::GetDeltaTime => {
                EngineResponse::DeltaTime(delta_time)
            }
            
            EngineRequest::IsKeyPressed { key } => {
                let pressed = match key.as_str() {
                    "W" => input_state.is_key_pressed(winit::keyboard::KeyCode::KeyW),
                    "A" => input_state.is_key_pressed(winit::keyboard::KeyCode::KeyA),
                    "S" => input_state.is_key_pressed(winit::keyboard::KeyCode::KeyS),
                    "D" => input_state.is_key_pressed(winit::keyboard::KeyCode::KeyD),
                    "Space" => input_state.is_key_pressed(winit::keyboard::KeyCode::Space),
                    "Escape" => input_state.is_key_pressed(winit::keyboard::KeyCode::Escape),
                    "Enter" => input_state.is_key_pressed(winit::keyboard::KeyCode::Enter),
                    "Tab" => input_state.is_key_pressed(winit::keyboard::KeyCode::Tab),
                    "Shift" => input_state.is_key_pressed(winit::keyboard::KeyCode::ShiftLeft),
                    "Control" => input_state.is_key_pressed(winit::keyboard::KeyCode::ControlLeft),
                    "Alt" => input_state.is_key_pressed(winit::keyboard::KeyCode::AltLeft),
                    "ArrowUp" => input_state.is_key_pressed(winit::keyboard::KeyCode::ArrowUp),
                    "ArrowDown" => input_state.is_key_pressed(winit::keyboard::KeyCode::ArrowDown),
                    "ArrowLeft" => input_state.is_key_pressed(winit::keyboard::KeyCode::ArrowLeft),
                    "ArrowRight" => input_state.is_key_pressed(winit::keyboard::KeyCode::ArrowRight),
                    _ => false,
                };
                EngineResponse::KeyPressed(pressed)
            }
            
            EngineRequest::GetMousePosition => {
                let pos = Vec2::new(input_state.mouse_pos.0 as f32, input_state.mouse_pos.1 as f32);
                EngineResponse::MousePosition(pos)
            }
            
            EngineRequest::GetMouseDelta => {
                if let Some(delta) = input_state.mouse_delta {
                    let delta_vec = Vec2::new(delta.0 as f32, delta.1 as f32);
                    EngineResponse::MouseDelta(delta_vec)
                } else {
                    EngineResponse::MouseDelta(Vec2::ZERO)
                }
            }
            
            EngineRequest::GetEntityInfo { id } => {
                let entity = Entity::from_bits(id).unwrap();
                
                if let Ok(entity_ref) = world.entity(entity) {
                    let mut components = Vec::new();
                    let mut transform_data = None;
                    let label = None; // TODO: Implement label system
                    
                    // Check for Transform component
                    if let Some(transform) = entity_ref.get::<&Transform>() {
                        transform_data = Some((*transform).into());
                        components.push("Transform".to_string());
                    }
                    
                    // Check for ScriptComponent
                    if let Some(_script) = entity_ref.get::<&ScriptComponent>() {
                        components.push("Script".to_string());
                    }
                    
                    EngineResponse::EntityInfo {
                        id,
                        label,
                        transform: transform_data,
                        components,
                    }
                } else {
                    EngineResponse::Error {
                        message: format!("Entity with id {} not found", id),
                    }
                }
            }
            
            EngineRequest::GetEntityTransform { id } => {
                let entity = Entity::from_bits(id).unwrap();
                
                if let Ok(entity_ref) = world.entity(entity) {
                    if let Some(transform) = entity_ref.get::<&Transform>() {
                        EngineResponse::EntityTransform((*transform).into())
                    } else {
                        EngineResponse::Error {
                            message: "Entity has no Transform component".to_string(),
                        }
                    }
                } else {
                    EngineResponse::Error {
                        message: format!("Entity with id {} not found", id),
                    }
                }
            }
            
            EngineRequest::SetEntityTransform { id, transform } => {
                let entity = Entity::from_bits(id).unwrap();
                
                if let Ok(entity_ref) = world.entity(entity) {
                    if let Some(mut transform_comp) = entity_ref.get::<&mut Transform>() {
                        *transform_comp = transform.into();
                        EngineResponse::Success
                    } else {
                        EngineResponse::Error {
                            message: "Entity has no Transform component".to_string(),
                        }
                    }
                } else {
                    EngineResponse::Error {
                        message: format!("Entity with id {} not found", id),
                    }
                }
            }
            
            EngineRequest::GetSceneInfo => {
                let mut entities = Vec::new();
                
                for (entity, _) in world.query::<()>().iter() {
                    let id = entity.to_bits().get();
                    let mut components = Vec::new();
                    let mut transform_data = None;
                    
                    if let Ok(entity_ref) = world.entity(entity) {
                        if let Some(transform) = entity_ref.get::<&Transform>() {
                            transform_data = Some((*transform).into());
                            components.push("Transform".to_string());
                        }
                        
                        if entity_ref.get::<&ScriptComponent>().is_some() {
                            components.push("Script".to_string());
                        }
                    }
                    
                    entities.push(EntityInfo {
                        id,
                        label: None, // TODO: Implement label system
                        transform: transform_data,
                        components,
                    });
                }
                
                EngineResponse::SceneInfo {
                    entity_count: entities.len(),
                    entities,
                }
            }
            
            EngineRequest::GetAllEntities => {
                let mut entities = Vec::new();
                
                for (entity, _) in world.query::<()>().iter() {
                    let id = entity.to_bits().get();
                    let mut components = Vec::new();
                    let mut transform_data = None;
                    
                    if let Ok(entity_ref) = world.entity(entity) {
                        if let Some(transform) = entity_ref.get::<&Transform>() {
                            transform_data = Some((*transform).into());
                            components.push("Transform".to_string());
                        }
                        
                        if entity_ref.get::<&ScriptComponent>().is_some() {
                            components.push("Script".to_string());
                        }
                    }
                    
                    entities.push(EntityInfo {
                        id,
                        label: None, // TODO: Implement label system
                        transform: transform_data,
                        components,
                    });
                }
                
                EngineResponse::EntityList(entities)
            }
            
            EngineRequest::CreateEntity { label: _ } => {
                // Create new entity with transform
                let entity = world.spawn((Transform::default(),));
                let id = entity.to_bits().get();
                EngineResponse::EntityCreated { id }
            }
            
            EngineRequest::DeleteEntity { id } => {
                let entity = Entity::from_bits(id).unwrap();
                if world.despawn(entity).is_ok() {
                    EngineResponse::Success
                } else {
                    EngineResponse::Error {
                        message: format!("Failed to delete entity with id {}", id),
                    }
                }
            }
            
            _ => EngineResponse::Error {
                message: "Request not implemented".to_string(),
            },
        }
    }
}