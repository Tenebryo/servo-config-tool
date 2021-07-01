#![allow(dead_code, unused_macros)]

use cgmath::Matrix4;
use vulkano::image::view::ImageView;
use winit::event::{Event, WindowEvent};
use winit::event_loop::ControlFlow;

use winit::event_loop::EventLoop;

macro_rules! im_strf {
    ($($args:tt)*) => {
        &imgui::ImString::from(format!($($args)*))
    };
}

struct WindowRect {
    pos : [f32; 2],
    size : [f32; 2],
}

mod clipboard;
mod gui_renderer;
mod gui_logic;
mod viewport;
mod line_renderer;
mod stlink;
mod controller_commands;
mod controller_interface;
mod layout;

fn main() {

    let mut async_runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();


    let event_loop = EventLoop::new();
    
    let mut gui_state = gui_logic::GuiState::init();

    let (mut system, mut gui_ctx) = gui_renderer::init("Servo Tuner", &event_loop);
    
    let mut line_renderer = line_renderer::LineRenderer::init(&mut system);

    let mut viewport = viewport::Viewport::new();

    event_loop.run(move |event, _, control_flow| {

        match event {
            Event::NewEvents(_) => {
                // gui_ctx.io_mut().update_delta_time(Instant::now());
            }
            Event::MainEventsCleared => {
                system.platform
                    .prepare_frame(gui_ctx.io_mut(), &system.surface.window())
                    .expect("Failed to prepare frame");
                system.surface.window().request_redraw();
            }
            Event::RedrawRequested(_) => {

                if let Ok((mut cmd_buf_builder, swapchain_image, image_num)) = system.start_frame() {


                    let mut ui = gui_ctx.frame();

                    let run = true;

                    gui_state.frame(&mut system, &mut ui, &mut async_runtime, &mut viewport, &mut line_renderer);


                    if !run {
                        *control_flow = ControlFlow::Exit;
                    }
                    
                    system.platform.prepare_render(&ui, system.surface.window());
                    let draw_data = ui.render();

                    if let Some(viewport_image) = viewport.image.clone() {
                        cmd_buf_builder.clear_color_image(viewport_image, [0.1; 4].into()).unwrap();

                        line_renderer.render(&mut system, &viewport, &mut cmd_buf_builder, Matrix4::from_nonuniform_scale(1.0, viewport.height as f32 / viewport.width as f32, 1.0), viewport.width, viewport.height)
                    }

                    cmd_buf_builder.clear_color_image(swapchain_image.clone(), [0.0; 4].into())
                        .expect("Failed to create image clear command");

                    system.renderer
                        .draw_commands(&mut cmd_buf_builder, system.queue.clone(), ImageView::new(swapchain_image.clone()).unwrap(), draw_data)
                        .expect("Rendering failed");

                    // viewport.update(&mut system, ui_state.viewport_dims[0] as u32, ui_state.viewport_dims[1] as u32);

                    system.end_frame(cmd_buf_builder, image_num);
                }
            }
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => *control_flow = ControlFlow::Exit,
            event => {
                system.platform.handle_event(gui_ctx.io_mut(), system.surface.window(), &event);
            }
        }
    });
}