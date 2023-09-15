use std::io::{Read, Write};
use std::net::TcpStream;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use anyhow::Result;
use wgpu::{BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor, BindGroupLayoutEntry, Buffer, BufferBindingType, BufferUsages, Queue, ShaderStages};
use wgpu::util::{BufferInitDescriptor, DeviceExt};
use crate::vertex::Vertex;

const BUFFER_SIZE: usize = 8;
const MOVEMENT_SPEED: f32 = 0.05;
pub enum PossibleMovements {
    NoInput = 0,

    Forward = 1,
    Backwards = 2,
    Left = 3,
    Right = 4,
}
pub struct Player {
    buffer: Vec<u8>,
    pub stream: Arc<Mutex<TcpStream>>,

    vertex_buffer: Buffer,
    index_buffer: Buffer,
    num_indices: u32,

    position_buffer: Buffer,
    position_bind_group: BindGroup,
    position_bind_group_layout: BindGroupLayout,
    position: [f32; 2],

    pub input: Input,
}
impl Player {
    pub fn new(host_addr: &str, device: &wgpu::Device) -> Self {
        let stream = Arc::new(Mutex::new(TcpStream::connect(host_addr).unwrap())); // this crashes the program if the host isnt hosting
        let (vertices, indices) = Self::create_shape_optimized(40);
        let vertex_buffer =  device.create_buffer_init(&BufferInitDescriptor{
            label: Some("Vertex buffer"),
            contents: bytemuck::cast_slice(vertices.as_slice()),
            usage: BufferUsages::VERTEX,
        });

        let index_buffer = device.create_buffer_init(&BufferInitDescriptor{
            label: Some("Index Buffer"),
            contents: bytemuck::cast_slice(indices.as_slice()),
            usage: BufferUsages::INDEX,
        });
        let position = [0.0f32; 2];
        let position_uniform = device.create_buffer_init(&BufferInitDescriptor{
            label: Some("position uniform"),
            contents: bytemuck::cast_slice(&position),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        });
        let position_bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor{
            label: Some("position bind group layout"),
            entries: &[BindGroupLayoutEntry{
                binding: 0,
                visibility: ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let position_bind_group = device.create_bind_group(&BindGroupDescriptor{
            label: Some("position bind group"),
            layout: &position_bind_group_layout,
            entries: &[BindGroupEntry{
                binding: 0,
                resource: position_uniform.as_entire_binding(),
            }],
        });


        Self{
            buffer: Vec::new(),
            stream,
            vertex_buffer,
            index_buffer,
            num_indices: indices.len() as u32,

            position_buffer: position_uniform,
            position_bind_group,
            position_bind_group_layout,
            position,

            input: Input::default(),
        }
    }

    pub fn add_movement(&mut self, movement: PossibleMovements, queue: &Queue) -> Result<()> {
        match movement {
            PossibleMovements::NoInput => {
                self.buffer.push(0);
            }
            PossibleMovements::Forward => {
                self.buffer.push(1);
                self.position[1] += MOVEMENT_SPEED;
            }
            PossibleMovements::Backwards => {
                self.buffer.push(2);
                self.position[1] -= MOVEMENT_SPEED;
            }
            PossibleMovements::Left => {
                self.buffer.push(3);
                self.position[0] -= MOVEMENT_SPEED;
            }
            PossibleMovements::Right => {
                self.buffer.push(4);
                self.position[0] += MOVEMENT_SPEED;
            }
        };
        Self::rewrite_position_buffer(self, queue);
        if self.buffer.len() == BUFFER_SIZE {
            {
                Self::send_buffer(self)?;
            }
            self.buffer = Vec::new();
        }
        Ok(())
    }

    pub fn send_buffer(&mut self) -> Result<()> {
        println!("sending buffer");
        let buffer = self.buffer.as_slice();
        let stream = Arc::clone(&self.stream);
        println!("deadlock, trust me");
        {
            let mut stream = stream.lock().unwrap();
            match stream.write(buffer){
                Ok(_) => {}
                Err(err) => { return Err(err.into()) }
            };
        }
        // make the read operation after the write op (and make them multithreaded)
        Ok(())
    }

    pub fn get_buffers(&self) -> (&Buffer, &Buffer, u32, &BindGroup) {
        (&self.vertex_buffer, &self.index_buffer, self.num_indices, &self.position_bind_group)
    }
    pub fn get_bind_group_layout(&self) -> &BindGroupLayout {
        &self.position_bind_group_layout
    }

    pub fn get_players_position(stream: &Arc<Mutex<TcpStream>>) -> Vec<[f32; 2]> {
        let mut buf: [u8; 4] = [0; 4];
        let mut other_players_position: Vec<[f32; 2]> = vec![];
        let mut stream = stream.lock().unwrap();
        println!("peeking");
        let len = stream.peek(&mut buf).unwrap();
        println!("finished peeking");
        // for _ in 0..len / 4 {
        //     stream.read(&mut buf).unwrap();
        //     let x = f32::from_be_bytes(buf);
        //     stream.read(&mut buf).unwrap();
        //     let y = f32::from_be_bytes(buf);
        //     other_players_position.push([x, y]);
        // }
        other_players_position
    }
    fn rewrite_position_buffer(&mut self,  queue: &Queue){
        queue.write_buffer(&self.position_buffer, 0, bytemuck::cast_slice(&self.position));
    }
    fn create_shape_optimized(num_vertices: u32) -> (Vec<Vertex>, Vec<u16>){
        let mut vertices: Vec<Vertex> = Vec::new();

        for i in 1..= num_vertices {
            vertices.push(Vertex{
                position: [
                    f32::cos(f32::to_radians(i as f32*(90./(num_vertices as f32/4.)))) / 8.0,
                    f32::sin(f32::to_radians(i as f32*(90./(num_vertices as f32/4.)))) / 8.0,
                    0.0
                ],
                color: [1.0, 0.0, 1.0],
            });
        }
        let mut indices: Vec<u16> = Vec::new();
        for i in 1..vertices.len() - 1 {
            indices.push(0);
            indices.push(i as u16);
            indices.push((i + 1) as u16);
        }
        (vertices, indices)
    }
}
pub struct Input{
    pub forward: bool,
    pub backward: bool,
    pub left: bool,
    pub right: bool,
}
impl Default for Input{
    fn default() -> Self {
        Input{
            forward: false,
            backward: false,
            left: false,
            right: false,
        }
    }
}
impl Input{
    pub fn input(&self) -> bool {
        self.right || self.backward || self.left || self.forward
    }

}