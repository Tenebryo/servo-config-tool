use crate::controller_interface::ControllerPointers;
use crate::stlink::STLink;

const SWD_COMMAND_BUFFER_LEN: usize = 32;

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub enum Command {
    MotorStop,
    MotorStart,
    SetStepDirectionControl,
    SetPositionControl,
    SetVelocityControl,
    SetTorqueControl,
    ClearFaultState,
    PositionCommand {
        position : f32,
    },
    VelocityCommand {
        velocity : f32,
    },
    TorqueCommand {
        torque : f32,
    },
    FindUpperMotionLimit,
    FindLowerMotionLimit,
    LoadServoConfig,
    SaveServoConfig,
    SetMotionProfile {
        profile : u32,
    }
}


#[derive(Debug, Clone)]
#[repr(C)]
pub struct CommandBufferInfo {
    front : u32,
    back : u32,
    capacity : u32,
    data_addr : u32,
}

pub fn send_command(link : &mut STLink, ptrs : &ControllerPointers, cmd : Command) -> Result<(), ()> {
    let command_buffer_info = link.read_struct::<CommandBufferInfo>(ptrs.command_buffer_addr);

    if (command_buffer_info.back + 1) % command_buffer_info.capacity != command_buffer_info.front {
        link.write_struct_array_offset(command_buffer_info.data_addr, command_buffer_info.back, &[cmd]);
        link.write_struct_array_offset(ptrs.command_buffer_addr, 1, &[(command_buffer_info.back + 1) % command_buffer_info.capacity]);
        Ok(())
    } else {
        Err(())
    }
}