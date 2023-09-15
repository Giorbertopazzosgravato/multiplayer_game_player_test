use cgmath::prelude::*;
use std::{iter, thread};
use std::default::Default;
use std::ops::{Add, Mul};
use std::process::exit;
use std::sync::{Arc, Mutex};
use image::GenericImageView;
use rand::{Rng};
use wgpu::{Buffer, Color, ColorWrites, DeviceDescriptor, Features, FragmentState, PipelineLayoutDescriptor, RenderPipeline, ShaderModuleDescriptor, ShaderSource, Surface, SurfaceError, VertexState};
use wgpu::util::{DeviceExt};
use winit::{
    event::*,
    event_loop::{ControlFlow, EventLoop},
    window::{Window, WindowBuilder},
};
use wgpu_glyph::{ab_glyph, GlyphBrush, GlyphBrushBuilder, HorizontalAlign, Layout, Section, Text};
use winit::dpi::PhysicalSize;
use winit::event::WindowEvent::KeyboardInput;
use winit::keyboard::{KeyCode};
use crate::instance::Instance;

mod player;
pub mod vertex;
mod instance;

use crate::player::{Player, PossibleMovements};
use crate::vertex::Vertex;

const HOST_ADDR: &str = "localhost:7878";


struct State {
    surface: Surface,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    size: PhysicalSize<u32>,
    window: Window,
    render_pipeline: RenderPipeline,
    player: Player,

    pub players_position: Arc<Mutex<Box<Vec<[f32; 2]>>>>,
    // instances: Vec<Instance>,
    // instance_buffer: Buffer,
    // glyph_brush: GlyphBrush<()>,
    // staging_belt: StagingBelt,
}

impl State {
    async unsafe fn new(window: Window) -> Self {
        let size = window.inner_size();
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::DX12,
            dx12_shader_compiler: Default::default(),
        });
        let surface = unsafe { instance.create_surface(&window) }.unwrap();
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .unwrap();
        let (device, queue) = adapter.request_device(
            &DeviceDescriptor{
                features: Features::POLYGON_MODE_LINE | Features::POLYGON_MODE_POINT,
                limits: if cfg!(target_arch = "wasm32") {
                    wgpu::Limits::downlevel_webgl2_defaults()
                }else {
                    wgpu::Limits::default()
                },
                label: None,
            },
            None
        ).await.unwrap();

        let player = Player::new(HOST_ADDR, &device);

        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(surface_caps.formats[0]);
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
        };
        surface.configure(&device, &config);

        let shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("Shader"),
            source: ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });

        // let staging_belt = StagingBelt::new(1024);
        // let font = ab_glyph::FontArc::try_from_slice(include_bytes!("./ARCADECLASSIC.TTF")).unwrap();
        // let mut glyph_brush = GlyphBrushBuilder::using_font(font).build(&device, wgpu::TextureFormat::Bgra8UnormSrgb);

        let render_pipeline_layout =
            device.create_pipeline_layout(&PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[player.get_bind_group_layout()],
                push_constant_ranges: &[],
            });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[Vertex::desc()],
            },
            fragment: Some(FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState {
                        color: wgpu::BlendComponent::REPLACE,
                        alpha: wgpu::BlendComponent::REPLACE,
                    }),
                    write_mask: ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
        });


        Self {
            surface,
            device,
            queue,
            size,
            config,
            render_pipeline,
            window,

            player,
            players_position: Arc::new(Mutex::new(Box::new(vec![]))),
        }
    }

    pub fn window(&self) -> &Window {
        &self.window
    }

    pub fn resize(&mut self, new_size: PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);
        }
    }

    #[allow(unused_variables)]
    fn input(&mut self, event: &WindowEvent) -> bool {
        match event {
            KeyboardInput { device_id, event, is_synthetic } => {
                if event.physical_key == KeyCode::KeyW && event.state == ElementState::Pressed {
                    println!("forward");
                    self.player.input.forward = true;
                    return true;
                }
                if event.physical_key == KeyCode::KeyS && event.state == ElementState::Pressed {
                    println!("backward");
                    self.player.input.backward = true;
                    return true;
                }
                if event.physical_key == KeyCode::KeyA && event.state == ElementState::Pressed {
                    println!("left");
                    self.player.input.left = true;
                    return true;
                }
                if event.physical_key == KeyCode::KeyD && event.state == ElementState::Pressed {
                    println!("right");
                    self.player.input.right = true;
                    return true;
                }
                // ===============================================================================
                if event.physical_key == KeyCode::KeyW && event.state == ElementState::Released {
                    println!("stop forward");
                    self.player.input.forward = false;
                    return true;
                }
                if event.physical_key == KeyCode::KeyA && event.state == ElementState::Released {
                    println!("stop left");
                    self.player.input.left = false;
                    return true;
                }
                if event.physical_key == KeyCode::KeyS && event.state == ElementState::Released {
                    println!("stop backward");
                    self.player.input.backward = false;
                    return true;
                }
                if event.physical_key == KeyCode::KeyD && event.state == ElementState::Released {
                    println!("stop right");
                    self.player.input.right = false;
                    return true;
                }
            }
            _ => {
                println!("no input");
                self.player.add_movement(PossibleMovements::NoInput, &self.queue).expect("app crashed");
                return false;
            }
        };
        println!("no input");
        self.player.add_movement(PossibleMovements::NoInput, &self.queue).expect("app crashed");
        false
    }

    fn update(&mut self) {
        let players_clone = Arc::clone(&self.players_position);
        let player_stream_clone = Arc::clone(&self.player.stream);
        // thread::spawn(move||{
        //     let mut players = players_clone.lock().unwrap();
        //     let stream = player_stream_clone;
        //     let new_players_position = Player::get_players_position(&stream);
        //     **players = new_players_position;
        // });
        if self.player.input.forward {
            self.player.add_movement(PossibleMovements::Forward, &self.queue).expect("gyat dayum");
        }
        if self.player.input.backward {
            self.player.add_movement(PossibleMovements::Backwards, &self.queue).expect("gyat dayum");
        }
        if self.player.input.left {
            self.player.add_movement(PossibleMovements::Left, &self.queue).expect("gyat dayum");
        }
        if self.player.input.right {
            self.player.add_movement(PossibleMovements::Right, &self.queue).expect("gyat dayum");
        }
        if !self.player.input.input() {
            self.player.add_movement(PossibleMovements::NoInput, &self.queue).expect("gyat dayum");
        }
    }

    fn render(&mut self) -> Result<(), SurfaceError> {
        let output = self.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(Color {
                            r: 0.1,
                            g: 0.2,
                            b: 0.3,
                            a: 1.0,
                        }),
                        store: true,
                    },
                })],
                depth_stencil_attachment: None,
            });
            let (vertex_buffer, index_buffer, num_indices, bind_group) = self.player.get_buffers();
            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_bind_group(0, bind_group, &[]);
            render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
            render_pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            render_pass.draw_indexed(0..num_indices, 0, 0..1);
        }
        self.queue.submit(iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}

pub async fn run() {
    let event_loop = EventLoop::new().unwrap();
    let mut size = PhysicalSize::new(600u32, 600);

    let window = WindowBuilder::new()
        .with_title("super fun game")
        .with_inner_size(size)
        .build(&event_loop)
        .unwrap();
    window.set_resizable(true);

    let mut state = unsafe { State::new(window) }.await;
    event_loop.run(move |event, _, control_flow| {
        match event {
            Event::WindowEvent {
                ref event,
                window_id,
            } if window_id == state.window().id() => {
                if !state.input(event) {
                    match event {
                        WindowEvent::CloseRequested => {
                            *control_flow = ControlFlow::Exit;
                            exit(0);
                        }
                        KeyboardInput {
                            event: KeyEvent{
                                physical_key: KeyCode::Escape, state: ElementState::Pressed, ..
                            },
                            ..
                        } => {
                            *control_flow = ControlFlow::Exit;
                            exit(0)
                        },
                        WindowEvent::Resized(physical_size) => {
                            state.resize(*physical_size);
                            size = *physical_size;
                        }
                        _ => {}
                    }
                }
            }
            Event::RedrawRequested(window_id) if window_id == state.window().id() => {
                state.update();
                match state.render() {
                    Ok(_) => {}
                    // Reconfigure the surface if it's lost or outdated
                    Err(SurfaceError::Lost | SurfaceError::Outdated) => {
                        state.resize(state.size)
                    }
                    // The system is out of memory, we should probably quit
                    Err(SurfaceError::OutOfMemory) => *control_flow = ControlFlow::Exit,
                    Err(SurfaceError::Timeout) => log::warn!("Surface timeout"),
                }
            }
            _ => {}
        }
        state.window.request_redraw();
    }).expect("TODO: panic message");
}