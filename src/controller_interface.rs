use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use parking_lot::Mutex;

use crate::controller_commands::Command;
use crate::controller_commands::send_command;
use crate::stlink::STLink;

const MAGIC : [u8; 7] = [0x54, 0xA4, 0x2F, 0x6F, 0x07, 0x8A, 0x48];

const CONFIG_ADDR_ADDR : u32 = 0x2000_0000;

#[derive(Debug, Clone)]
#[repr(C)]
pub struct ControllerPointers {
    pub magic : [u8;7],
    pub ready : bool,
    pub servo_config_addr : u32,
    pub servo_state_addr : u32,
    pub oscilloscope_addr : u32,
    pub oscilloscope_data_addr : u32,
    pub command_buffer_addr : u32,
}


pub const OFFSET_POSITION_GAIN               : u32 = 0;
pub const OFFSET_VELOCITY_GAIN               : u32 = 1;
pub const OFFSET_VELOCITY_INTEGRATOR_GAIN    : u32 = 2;
pub const OFFSET_VELOCITY_INTEGRATOR_MAX_ABS : u32 = 3;
pub const OFFSET_INDEX_SCAN_SPEED            : u32 = 4;
pub const OFFSET_TURNS_PER_STEP              : u32 = 5;
pub const OFFSET_VEL_MAX_ABS                 : u32 = 6;
pub const OFFSET_TOR_MAX_ABS                 : u32 = 7;
pub const OFFSET_MAX_POS_STEP                : u32 = 8;
pub const OFFSET_INPUT_FILT_KP               : u32 = 9;
pub const OFFSET_INPUT_FILT_KI               : u32 = 10;
pub const OFFSET_INERTIA                     : u32 = 11;
pub const OFFSET_TORQUE_BANDWIDTH            : u32 = 12;
pub const OFFSET_VEL_PLLKI                   : u32 = 13;

#[derive(Debug, Clone)]
#[repr(C)]
pub struct ServoConfig {
    pub position_gain : f32,
    pub velocity_gain : f32,
    pub velocity_integrator_gain : f32,
    pub velocity_integrator_max_abs : f32,
    pub index_scan_speed : f32,
    pub turns_per_step : f32,
    pub vel_max_abs : f32,
    pub tor_max_abs : f32,
    pub max_pos_step : f32,
    pub input_filt_kp : f32,
    pub input_filt_ki : f32,
    pub inertia : f32,
    pub torque_bandwidth : f32,
    pub vel_pllki : f32,
    // pub antcogging_torque : [f32; 512],
}

impl Default for ServoConfig {
    fn default() -> Self {
        ServoConfig {
            position_gain: 0.0,
            velocity_gain: 0.0,
            velocity_integrator_gain: 0.0,
            velocity_integrator_max_abs: 0.0,
            index_scan_speed: 0.0,
            turns_per_step: 0.0,
            vel_max_abs: 0.0,
            tor_max_abs: 0.0,
            max_pos_step: 0.0,
            input_filt_kp: 0.0,
            input_filt_ki: 0.0,
            inertia: 0.0,
            torque_bandwidth: 0.0,
            vel_pllki: 0.0,
            // antcogging_torque: [0.0; 512],
        }
    }
}

#[derive(Debug, Clone)]
#[repr(C)]
pub enum ServoControlState {
  Uninit,
  Disabled,
  Aligning,
  AnticoggingCalibration,
  EnabledStepDirection,
  EnabledPositionFilter,
  EnabledPid,
  EnabledPiv,
  EnabledVelocity,
  EnabledTorque,
}

impl Default for ServoControlState {
    fn default() -> Self {
        ServoControlState::Uninit
    }
}

#[derive(Debug, Clone, Default)]
#[repr(C)]
pub struct ServoState {
    pub state : ServoControlState,
    
    pub pos_input : f32,
    pub vel_input : f32,
    pub tor_input : f32,
    pub pos_setpoint : f32,
    pub vel_setpoint : f32,
    pub tor_setpoint : f32,
    pub accel : f32,
    pub velocity : f32,
    pub position : f32,
    pub raw_position : f32,
    pub max_vel_abs_obs : f32,

    pub encoder_offset : i32,
    pub step_dir_offset : i32,
    pub anticogging_sampless : u16,
    pub anticogging_index : u16,

    pub anticogging_sum : f32,

    pub aligned : bool,
    pub anticogging_calibrated : bool,
    pub anticogging_returning : bool,
}


#[derive(Debug, Clone, Default)]
#[repr(C)]
pub struct ServoPointers {
    pub pos_pid_addr : u32,
    pub pos_piv_addr : u32,
    pub vel_piv_addr : u32,

    pub spd_tor_addr : u32,
    pub enc_addr : u32,
}

#[derive(Debug, Clone, Default)]
#[repr(C)]
pub struct ServoData {
    pub config : ServoConfig,

    pub pointers : ServoPointers,

    pub state : ServoState
}

#[derive(Debug, Clone)]
#[repr(C)]
pub struct OscilloscopeSamplePoint {
    pub pos : f32,
    pub vel : f32,
    pub acc : f32,
    pub pos_setpoint : f32,
    pub vel_setpoint : f32,
    pub tor_setpoint : f32,
    pub pos_input : f32,
    pub vel_input : f32,
}

#[derive(Debug, Clone, Default)]
#[repr(C)]
pub struct Oscilloscope {
    pub recording : bool,
    pub index : u32,
    pub interval : u32,
    pub len : u32,
}


#[derive(Debug, Clone)]
#[repr(C)]
pub struct PIDController {
    pub default_kp_gain : f32,
    pub default_ki_gain : f32,
    pub default_kd_gain : f32,
    pub kp_gain : f32,
    pub ki_gain : f32,
    pub kd_gain : f32,

    pub integral_upper_limit : f32,
    pub integral_lower_limit : f32,
    pub integrator_decay : f32,
    
    pub output_upper_limit : f32,
    pub output_lower_limit : f32,
    
    pub intergral_term : f32,
    pub previous_error : f32,
}

#[derive(Debug, Default)]
pub struct ControllerData {
    pub servo_config : ServoConfig,
    pub servo_state : ServoState,
    pub oscilloscope : Oscilloscope,
}

#[derive(Debug, Clone)]
pub enum InterfaceCommand {
    WriteServoConfig(ServoConfig),
    StartRecording,
    StopRecording,
    StopMotor,
    StartMotor,
    PositionCommand(f32),
    UpdateConfigParameter(u32, f32),
    SendCommand(Command),
    ResetController,
}

pub fn controller_connection_task(
    link : Arc<Mutex<STLink>>, 
    running : Arc<AtomicBool>,
    controller_data : Arc<Mutex<ControllerData>>,
    sample_buffer : Arc<Mutex<Vec<OscilloscopeSamplePoint>>>,
    command_list : Arc<Mutex<Vec<InterfaceCommand>>>) {

    running.store(true, std::sync::atomic::Ordering::SeqCst);

    command_list.lock().clear();

    link.lock().connect();

    link.lock().enter_debug_swd();

    let config_addr = link.lock().read_struct::<u32>(CONFIG_ADDR_ADDR);

    // println!("Base pointers location: {:4X}", config_addr);
    
    let base = link.lock().read_struct::<ControllerPointers>(config_addr);
    
    let mut osc = link.lock().read_struct::<Oscilloscope>(base.oscilloscope_addr);

    osc.recording = true;
    
    link.lock().write_struct(base.oscilloscope_addr, osc.clone());

    // println!("{:?}", osc);

    let mut last_index = osc.index;
    
    let max_sample_storage = 10_000;

    let mut record_samples = true;

    while running.load(std::sync::atomic::Ordering::Relaxed) {

        let cmds = command_list.lock().drain(0..).collect::<Vec<_>>();

        for cmd in cmds {
            match cmd {
                InterfaceCommand::WriteServoConfig(cfg) => {
                    link.lock().write_struct(base.servo_config_addr, cfg);
                },
                InterfaceCommand::StartRecording => {
                    record_samples = true;
                },
                InterfaceCommand::StopRecording => {
                    record_samples = false;
                },
                InterfaceCommand::StopMotor => {
                    send_command(&mut link.lock(), &base, Command::MotorStop).ok();
                },
                InterfaceCommand::StartMotor => {
                    send_command(&mut link.lock(), &base, Command::MotorStart).ok();
                },
                InterfaceCommand::PositionCommand(position) => {
                    send_command(&mut link.lock(), &base, Command::PositionCommand{position}).ok();
                },
                InterfaceCommand::UpdateConfigParameter(offset, value) => {
                    link.lock().write_struct_array_offset(base.servo_config_addr, offset, &[value])
                },
                InterfaceCommand::SendCommand(cmd) => {
                    send_command(&mut link.lock(), &base, cmd).ok();
                },
                InterfaceCommand::ResetController => {
                    link.lock().debug_resetsys()
                },
            }
        }

        if record_samples {
            osc = link.lock().read_struct::<Oscilloscope>(base.oscilloscope_addr);
            let index = osc.index;

            let start_off = last_index;
            let mut end_off = index;

            if index < last_index {
                end_off = osc.len;
                last_index = 0;
            } else {
                last_index = index;
            }

            let mut data = link.lock().read_struct_array_with_offset::<OscilloscopeSamplePoint>(base.oscilloscope_data_addr, end_off - start_off, start_off);

            let mut lock = sample_buffer.lock();
            lock.append(&mut data);

            if lock.len() > max_sample_storage {
                let to_remove = lock.len() - max_sample_storage;
                lock.drain(0..to_remove);
            }
        }

        controller_data.lock().servo_state = link.lock().read_struct::<ServoState>(base.servo_state_addr);
        controller_data.lock().servo_config = link.lock().read_struct::<ServoConfig>(base.servo_config_addr);


        std::thread::sleep(std::time::Duration::from_millis(5));
    }

    link.lock().disconnect();
}