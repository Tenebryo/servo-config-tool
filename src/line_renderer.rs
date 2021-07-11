use cgmath::*;
use imgui::TextureId;

use vulkano::buffer::BufferUsage;
use vulkano::buffer::cpu_pool::CpuBufferPoolChunk;
use vulkano::buffer::CpuBufferPool;
use vulkano::command_buffer::AutoCommandBufferBuilder;
use vulkano::command_buffer::DynamicState;
use vulkano::command_buffer::PrimaryAutoCommandBuffer;
use vulkano::command_buffer::SubpassContents;
use vulkano::descriptor::descriptor_set::PersistentDescriptorSet;
use vulkano::format::ClearValue;
use vulkano::format::Format;
use vulkano::memory::pool::StdMemoryPool;
use vulkano::pipeline::input_assembly::PrimitiveTopology;
use vulkano::render_pass::RenderPass;
use vulkano::render_pass::Subpass;
use vulkano::{image::StorageImage, pipeline::GraphicsPipeline};
use vulkano::{impl_vertex, pipeline::GraphicsPipelineAbstract};

use std::sync::Arc;

use crate::viewport::Viewport;

pub mod line_fs {vulkano_shaders::shader!{ty: "fragment",path: "src/shaders/line.frag",               include: [],}}
pub mod line_vs {vulkano_shaders::shader!{ty: "vertex",  path: "src/shaders/line.vert",               include: [],}}

use crate::gui_renderer::System;

#[derive(Debug, Default, Clone, Copy)]
pub struct Vertex{
    pub pos : [f32;3],
    pub col : [f32;4],
}

impl_vertex!(Vertex, pos, col);

pub struct LineRenderer {
    pub pipeline : Arc<dyn GraphicsPipelineAbstract + Send + Sync>,
    pub render_pass : Arc<RenderPass>,
    pub image : Option<Arc<StorageImage>>,
    pub vertex_pool : CpuBufferPool<Vertex>,
    pub uniform_pool : CpuBufferPool<line_vs::ty::UniformBlock0>,
    pub vertex_buffers : Vec<Arc<CpuBufferPoolChunk<Vertex, Arc<StdMemoryPool>>>>,
    pub texture_id : Option<TextureId>,
}

impl LineRenderer {
    pub fn init(system : &System) -> Self {
        let render_pass = Arc::new(
            vulkano::ordered_passes_renderpass!(system.device.clone(),
                attachments: {
                    depth: {
                        load: Clear,
                        store: DontCare,
                        format: Format::D16Unorm,
                        samples: 4,
                    },
                    msaa: {
                        load: Clear,
                        store: DontCare,
                        format: Format::R8G8B8A8Unorm,
                        samples: 4,
                    },
                    color: {
                        load: DontCare,
                        store: Store,
                        format: Format::R8G8B8A8Unorm,
                        samples: 1,
                    }
                },
                passes: [
                    {
                        color: [msaa],
                        depth_stencil: {depth},
                        input : [],
                        resolve:[color]
                    }
                ]
            )
            .unwrap(),


        );



        let line_fs = line_fs::Shader::load(system.device.clone()).expect("failed to create shader module");
        let line_vs = line_vs::Shader::load(system.device.clone()).expect("failed to create shader module");

        let pipeline = Arc::new(
            GraphicsPipeline::start()
                .vertex_input_single_buffer::<Vertex>()
                .vertex_shader(line_vs.main_entry_point(), ())
                .primitive_topology(PrimitiveTopology::LineList)
                .viewports_dynamic_scissors_irrelevant(1)
                // .depth_stencil_simple_depth()
                .depth_write(true)
                .line_width_dynamic()
                .blend_alpha_blending()
                .fragment_shader(line_fs.main_entry_point(), ())
                .render_pass(Subpass::from(render_pass.clone(), 0).unwrap())
                .build(system.device.clone())
                .unwrap(),
        );

        let vertex_pool = CpuBufferPool::<Vertex>::new(system.device.clone(), BufferUsage::all());
        let uniform_pool = CpuBufferPool::<line_vs::ty::UniformBlock0>::new(system.device.clone(), BufferUsage::all());

        LineRenderer {
            render_pass,
            pipeline,
            image : None,
            vertex_pool,
            uniform_pool,
            vertex_buffers : vec![],
            texture_id : None,
        }
    }

    pub fn render(&mut self, _system : &mut System, viewport : &Viewport, cmd_buf_builder : &mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>, tmatrix : Matrix4<f32>, width : u32, height : u32) {

        let framebuffer = viewport.create_framebuffer(self.render_pass.clone());

        let v_matrix = 
            Matrix4::from_nonuniform_scale(1.0, width as f32 / height as f32, 1.0) * 
            Matrix4::from_translation(Vector3::new(0.0, 0.0, 0.5)) *
            Matrix4::from_nonuniform_scale(1.0, 1.0, 0.0001);

        if let Some(framebuffer) = framebuffer {

            cmd_buf_builder.begin_render_pass(
                framebuffer, 
                SubpassContents::Inline, 
                // vec![1.0.into(), [0.0, 0.0, 0.0, 1.0].into()]
                vec![1.0.into(), [0.05, 0.05, 0.05, 1.0].into(), ClearValue::None]
            ).expect("failed to start render pass");

            for vb in self.vertex_buffers.drain(0..) {

                let ds = DynamicState {
                    viewports : Some(vec![vulkano::pipeline::viewport::Viewport {
                        origin : [0.0; 2],
                        dimensions : [width as f32, height as f32],
                        depth_range : 0.0..1.0,
                    }]),
                    line_width: Some(3.0),
                    ..DynamicState::none()
                };

                let uniforms = self.uniform_pool.next(
                    line_vs::ty::UniformBlock0 {
                        matrix : (v_matrix * tmatrix).into(),
                        viewport : [width as f32, height as f32],
                    }
                ).unwrap();

                let layout = self.pipeline.layout().descriptor_set_layout(0).unwrap();
                let desc_set = Arc::new(PersistentDescriptorSet::start(layout.clone())
                    .add_buffer(uniforms).unwrap()
                    .build().unwrap()
                );

                cmd_buf_builder
                    .draw(
                        self.pipeline.clone(), &ds, vec![vb.clone()], 
                        desc_set, 
                        (),
                        vec![]
                    )
                    .expect("failed to draw line");
            }

            cmd_buf_builder.end_render_pass()
                .expect("Failed to finish render pass");

        }
    }

    pub fn draw_line(&mut self, path : &[Vector3<f32>], col : [f32; 4]) {

        let path = path.iter()
            .map(|p| Vertex {
                pos: [p.x, p.y, p.z],
                col,
            })
            .collect::<Vec<_>>();

        let new_vb = Arc::new(
            self.vertex_pool.chunk(path).expect("failed to allocated vertex buffer")
        );

        self.vertex_buffers.push(new_vb);

    }

    pub fn clear_line_buffer(&mut self) {

        self.vertex_buffers.clear();        
    }
}