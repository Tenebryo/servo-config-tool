use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::time::Duration;

use cgmath::Vector3;
use parking_lot::Mutex;
use winit::dpi::PhysicalSize;

use crate::controller_commands::Command;
use crate::controller_interface::*;
use crate::gui_renderer::System;
use crate::layout::LayoutRect;
use crate::line_renderer::LineRenderer;
use crate::stlink::STLink;

pub struct GuiTask {
    name : String,
    running : Arc<AtomicBool>,
}

pub struct GuiState {
    stlinks : Vec<Arc<Mutex<STLink>>>,
    connected : Arc<AtomicBool>,
    sample_buffer : Arc<Mutex<Vec<OscilloscopeSamplePoint>>>,
    controller_data : Arc<Mutex<ControllerData>>,
    controller_commands : Arc<Mutex<Vec<InterfaceCommand>>>,
    tasks : Vec<GuiTask>,
}

macro_rules! cfg_parameter_widget {
    ($ui:expr, $cmdbuf:expr, $text:expr, $label:expr, $value:expr, $offset:expr) => {
        $ui.text($text);
        let changed = $ui.input_float(im_str!($label), &mut $value)
            .enter_returns_true(true)
            .build();

        if changed {
            $cmdbuf.lock().push(
                InterfaceCommand::UpdateConfigParameter($offset, $value)
            );
        }
    };
}

impl GuiState {
    pub fn init() -> Self {
        GuiState {
            stlinks : vec![],
            connected : Arc::new(AtomicBool::new(false)),
            sample_buffer: Arc::new(Mutex::new(vec![])),
            controller_data: Arc::new(Mutex::new(ControllerData::default())),
            controller_commands: Arc::new(Mutex::new(vec![])),
            tasks : vec![]
        }
    }

    pub fn frame(&mut self, system : &mut System, ui : &mut imgui::Ui, _async_runtime : &mut tokio::runtime::Runtime, viewport : &mut crate::viewport::Viewport, line_renderer : &mut LineRenderer) {

        use imgui::im_str;

        let PhysicalSize { width, height } = system.surface.window().inner_size();

        let window_rect = LayoutRect::new(width, height);

        let (sidepanel_rect, viewport_rect) = window_rect.vertical_split_left_abs(400);

        let (viewport_rect, tool_menu_rect) = viewport_rect.horizontal_split_bottom_abs(400);

        let (devices_rect, config_menu_rect) = sidepanel_rect.horizontal_split_top_abs(100);


        imgui::Window::new(im_str!("Devices"))
            .position(devices_rect.position(), imgui::Condition::Always)
            .size(devices_rect.dimensions(), imgui::Condition::Always)
            .resizable(false)
            .movable(false)
            .collapsible(false)
            .scrollable(true)
            .build(ui, || {
                if ui.small_button(im_str!("Refresh Devices")) {
                    self.stlinks.clear();

                    self.stlinks.extend(STLink::enumerate().into_iter().map(|link| Arc::new(Mutex::new(link))));
                }

                let is_device_connected = self.stlinks.iter().any(|dev|dev.lock().connected);

                for (i, dev) in self.stlinks.iter_mut().enumerate() {

                    let dev_addr = dev.lock().device.address();
                    let dev_bus = dev.lock().device.bus_number();
                    let dev_type = dev.lock().dev_type;

                    ui.text(format!("[{}] {:?}", i, dev_type.version));

                    if dev.lock().connected {
                        ui.same_line(400.0 - 80.0);
                        if ui.small_button(im_strf!("Disconnect##Disconnect Device {:03}", i)) {
                            self.connected.store(false, std::sync::atomic::Ordering::Relaxed);
                        }
                    } else {
                        ui.same_line(400.0 - 80.0);
                        if !is_device_connected && ui.small_button(im_strf!("Connect##Connect Device {:03}", i)) {

                            let dev = dev.clone();
                            let connected = self.connected.clone();
                            let sample_buffer = self.sample_buffer.clone();
                            let controller_data = self.controller_data.clone();
                            let controller_commands = self.controller_commands.clone();

                            std::thread::spawn(|| {
                                controller_connection_task(
                                    dev, 
                                    connected, 
                                    controller_data, 
                                    sample_buffer,
                                    controller_commands,
                                );
                            });
                        }
                    }
                    ui.text(format!("  USB Bus: {}:{}", dev_bus, dev_addr));
                }
            });
        
        imgui::Window::new(im_str!("Configuration"))
            .position(config_menu_rect.position(), imgui::Condition::Always)
            .size(config_menu_rect.dimensions(), imgui::Condition::Always)
            .resizable(false)
            .movable(false)
            .collapsible(false)
            .scrollable(true)
            .build(ui, || {

                if self.connected.load(Ordering::Relaxed) {

                    let servo_config = &mut self.controller_data.lock().servo_config;

                    if imgui::CollapsingHeader::new(im_str!("Position Controller")).build(ui) {

                        // let servo_cfg = self.controller_data.lock().servo_config.clone();

                        cfg_parameter_widget!(
                            ui, self.controller_commands, 
                            "Position Gain", "Value##Position Gain", 
                            servo_config.position_gain, OFFSET_POSITION_GAIN
                        );

                        cfg_parameter_widget!(
                            ui, self.controller_commands, 
                            "Velocity Limit", "Value##Velocity Limit", 
                            servo_config.vel_max_abs, OFFSET_VEL_MAX_ABS
                        );
                        
                    }
                    
                    if imgui::CollapsingHeader::new(im_str!("Velocity Controller")).build(ui) {

                        cfg_parameter_widget!(
                            ui, self.controller_commands, 
                            "Velocity Gain", "Value##Velocity Gain", 
                            servo_config.velocity_gain, OFFSET_VELOCITY_GAIN
                        );
                        
                        cfg_parameter_widget!(
                            ui, self.controller_commands, 
                            "Velocity Integrator Gain", "Value##Velocity Integrator Gain", 
                            servo_config.velocity_integrator_gain, OFFSET_VELOCITY_INTEGRATOR_GAIN
                        );
                        
                        cfg_parameter_widget!(
                            ui, self.controller_commands, 
                            "Velocity Integrator Limit", "Value##Velocity Integrator Limit", 
                            servo_config.velocity_integrator_max_abs, OFFSET_VELOCITY_INTEGRATOR_MAX_ABS
                        );
                        
                        cfg_parameter_widget!(
                            ui, self.controller_commands, 
                            "Torque Limit", "Value##Torque Limit", 
                            servo_config.tor_max_abs, OFFSET_TOR_MAX_ABS
                        );
                    }
                    
                    if imgui::CollapsingHeader::new(im_str!("Servo Configuration")).build(ui) {

                        cfg_parameter_widget!(
                            ui, self.controller_commands, 
                            "Index Scan Speed", "Value##Index Scan Speed", 
                            servo_config.index_scan_speed, OFFSET_INDEX_SCAN_SPEED
                        );
                        
                        cfg_parameter_widget!(
                            ui, self.controller_commands, 
                            "Turns Per Step", "Value##Turns Per Step", 
                            servo_config.turns_per_step, OFFSET_TURNS_PER_STEP
                        );
                        
                        cfg_parameter_widget!(
                            ui, self.controller_commands, 
                            "Inertia", "Value##Inertia", 
                            servo_config.inertia, OFFSET_INERTIA
                        );
                        
                        cfg_parameter_widget!(
                            ui, self.controller_commands, 
                            "Torque Bandwidth", "Value##Torque Bandwidth", 
                            servo_config.torque_bandwidth, OFFSET_TORQUE_BANDWIDTH
                        );
                        
                    }
                } else {
                    ui.text("Connect to a device to see configuration.");
                }
            });


        
        imgui::Window::new(im_str!("Tuning Controls"))
            .position(tool_menu_rect.position(), imgui::Condition::Always)
            .size(tool_menu_rect.dimensions(), imgui::Condition::Always)
            .resizable(false)
            .movable(false)
            .collapsible(false)
            .scrollable(true)
            .build(ui, || {
                if self.connected.load(Ordering::Relaxed) {
                    ui.columns(4, im_str!("tool columns"), true);

                    if ui.small_button(im_str!("Start Recording")) {
                        self.controller_commands.lock().push(InterfaceCommand::StartRecording);
                    }
                    if ui.small_button(im_str!("Stop Recording")) {
                        self.controller_commands.lock().push(InterfaceCommand::StopRecording);
                    }
                    if ui.small_button(im_str!("Clear Faults")) {
                        self.controller_commands.lock().push(InterfaceCommand::SendCommand(Command::ClearFaultState));
                    }
                    if ui.small_button(im_str!("Save Configuration")) {
                        self.controller_commands.lock().push(InterfaceCommand::SendCommand(Command::SaveServoConfig));
                    }
                    if ui.small_button(im_str!("Reset Microcontroller")) {
                        self.controller_commands.lock().push(InterfaceCommand::ResetController);
                    }

                    ui.next_column();

                    if ui.small_button(im_str!("Stop Motor")) {
                        self.controller_commands.lock().push(InterfaceCommand::StopMotor);
                    }
                    if ui.small_button(im_str!("Start Motor")) {
                        self.controller_commands.lock().push(InterfaceCommand::StartMotor);
                    }
                    if ui.small_button(im_str!("Position Step 0.0")) {
                        self.controller_commands.lock().push(InterfaceCommand::PositionCommand(0.0));
                    }
                    if ui.small_button(im_str!("Position Step 1.0")) {
                        self.controller_commands.lock().push(InterfaceCommand::PositionCommand(1.0));
                    }
                    if ui.small_button(im_str!("1Hz Sine Input")) {
                        let running = Arc::new(AtomicBool::new(true));
                        let running_thread = running.clone();
                        let commands = self.controller_commands.clone();
                        std::thread::spawn(move || {
                            let mut t = 0.0;
                            while running_thread.load(Ordering::Relaxed) {
                                let x = (t/std::f32::consts::TAU).sin();
                                commands.lock().push(InterfaceCommand::PositionCommand(x));
                                std::thread::sleep(Duration::from_millis(5));
                                t += 0.005;
                            }
                        });

                        self.tasks.push(GuiTask{name : "Sine Input".to_string(), running});
                    }

                    ui.next_column();

                    for i in (0..(self.tasks.len())).rev() {
                        ui.text(format!("Task {:2}: {}", i, self.tasks[i].name));
                        if ui.small_button(im_strf!("Cancel##Cancel Task {}", i)) {
                            self.tasks[i].running.store(false, Ordering::Relaxed);
                            self.tasks.remove(i);
                        }
                    }

                    ui.next_column();
                
                } else {
                    ui.text("Connect to a device to see tuning menu.");
                }
            });
            
        let tok = ui.push_style_var(imgui::StyleVar::WindowPadding([0.0; 2]));

        imgui::Window::new(im_str!("Position/Velocity/Acceleration Plot"))
            .position(viewport_rect.position(), imgui::Condition::Always)
            .size(viewport_rect.dimensions(), imgui::Condition::Always)
            .resizable(false)
            .movable(false)
            .scroll_bar(false)
            .scrollable(false)
            .collapsible(false)
            .build(ui, || {
                
                let dim = ui.window_content_region_max();

                let sample_buffer = self.sample_buffer.lock();

                let n = sample_buffer.len();

                let funcs = [
                    |p : &OscilloscopeSamplePoint| p.pos_input,

                    |p : &OscilloscopeSamplePoint| p.pos_setpoint,
                    |p : &OscilloscopeSamplePoint| p.vel_setpoint,
                    |p : &OscilloscopeSamplePoint| p.tor_setpoint,

                    |p : &OscilloscopeSamplePoint| p.pos,
                    |p : &OscilloscopeSamplePoint| p.vel,
                    |p : &OscilloscopeSamplePoint| p.acc,
                ];

                let cols = [
                    [0.0, 0.6, 0.0, 1.0],

                    [0.2, 0.2, 0.8, 1.0],
                    [0.2, 0.2, 0.8, 1.0],
                    [0.2, 0.2, 0.8, 1.0],
                    
                    [0.8, 0.4, 0.4, 1.0],
                    [0.8, 0.4, 0.4, 1.0],
                    [0.8, 0.4, 0.4, 1.0],
                ];

                let offsets = [
                    -0.666,

                    -0.666,
                    0.0,
                    0.666,
                
                    -0.666,
                    0.0,
                    0.666,
                ];
                let mut points = Vec::with_capacity(2 * n + 1);

                for (func, (color, offset)) in funcs.iter().zip(cols.iter().zip(offsets.iter())) {

                    points.clear();

                    let min = sample_buffer.iter().map(func).min_by(|a,b| a.partial_cmp(b).unwrap()).unwrap_or(-1.0)-0.01;
                    let max = sample_buffer.iter().map(func).max_by(|a,b| a.partial_cmp(b).unwrap()).unwrap_or( 1.0)+0.01;
    
                    let diff = max - min;
    
    
                    let mut first = true;
                    for (i, pt) in sample_buffer.iter().enumerate() {
                        // let i = i * 8;
                        let val = func(pt);
                        let t = Vector3::new(
                            i as f32 / n as f32 * 2.0 - 1.0,
                            0.333 * (2.0 * (val - min) / diff - 1.0) + offset,
                            0.5
                        );
                        
                        points.push(t);
                        if first {
                            first = false;
                        } else {
                            points.push(t);
                        }
                    }
                    points.pop();
                    
                    line_renderer.draw_line(&points, *color);
                }

                viewport.update(system, dim[0] as u32, dim[1] as u32);

                if let Some(tid) = viewport.texture_id {
                    imgui::Image::new(tid, dim)
                        .build(ui);
                }

                
                let draw_list = ui.get_window_draw_list();

                let [mx, my] = ui.io().mouse_pos;

                let [wx0, wy0] = ui.window_pos();
                let [ww, wh] = ui.window_size();
                let [wx1, wy1] = [wx0 + ww, wy0 + wh];

                if sample_buffer.len() > 0 {
                    if wx0 < mx && mx < wx1 {
                        if wy0 < my && my < wy1 {
                            let ix = (((mx - wx0) / ww) * sample_buffer.len() as f32) as usize;
                            let y_pos = sample_buffer[ix].pos;
                            let y_vel = sample_buffer[ix].vel;
                            let y_acc = sample_buffer[ix].acc;
                            draw_list.add_text([mx, my], 0xFFFFFFFF, format!("  [{:.3}, {:.3}, {:.3}]", y_pos, y_vel, y_acc));
                        }
                    }
                }
            });

        tok.pop(ui);
    }
}