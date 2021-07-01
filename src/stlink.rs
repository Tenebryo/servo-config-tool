use core::mem::size_of;

use std::time::Duration;

use rusb::Device;
use rusb::DeviceHandle;
use rusb::GlobalContext;


const STLINK_CMD_SIZE_V2 : u32 = 16;

#[derive(Debug, Clone, Copy)]
pub enum STLinkVersion {
    V2,
    V2_1,
    V3E,
    V3,
}

#[derive(Debug, Clone, Copy)]
pub struct UsbDescriptor {
    pub version : STLinkVersion,
    pub vendor_id : u16,
    pub product_id : u16,
    pub out_pipe : u8,
    pub in_pipe : u8,
}

const DEV_TYPES : &[UsbDescriptor] = &[
    UsbDescriptor {
        version: STLinkVersion::V2,
        vendor_id: 0x0483,
        product_id: 0x3748,
        out_pipe: 0x02,
        in_pipe: 0x81,
    }, 
    UsbDescriptor {
        version: STLinkVersion::V2_1,
        vendor_id: 0x0483,
        product_id: 0x374b,
        out_pipe: 0x01,
        in_pipe: 0x81,
    }, 
    UsbDescriptor {
        version: STLinkVersion::V2_1,  // without MASS STORAGE
        vendor_id: 0x0483,
        product_id: 0x3752,
        out_pipe: 0x01,
        in_pipe: 0x81,
    }, 
    UsbDescriptor {
        version: STLinkVersion::V3E,
        vendor_id: 0x0483,
        product_id: 0x374e,
        out_pipe: 0x01,
        in_pipe: 0x81,
    }, 
    UsbDescriptor {
        version: STLinkVersion::V3,
        vendor_id: 0x0483,
        product_id: 0x374f,
        out_pipe: 0x01,
        in_pipe: 0x81,
    }, 
    UsbDescriptor {
        version: STLinkVersion::V3,  // without MASS STORAGE
        vendor_id: 0x0483,
        product_id: 0x3753,
        out_pipe: 0x01,
        in_pipe: 0x81,
    }
];

pub struct STLink {
    pub connected : bool,
    pub device : Device<GlobalContext>,
    pub handle : Option<DeviceHandle<GlobalContext>>,
    pub dev_type : UsbDescriptor,
}

impl STLink {
    pub fn enumerate() -> Vec<STLink> {
        rusb::devices().unwrap().iter()
            .filter_map(|dev| {

                let dsc = dev.device_descriptor().unwrap();

                for desc in DEV_TYPES {
                    if dsc.vendor_id() == desc.vendor_id && dsc.product_id() == desc.product_id {

                        return Some(STLink {
                            connected: false,
                            device : dev,
                            dev_type : *desc,
                            handle : None,
                        });
                    }
                }

                None

            })
            .collect::<Vec<_>>()
    }

    pub fn connect(&mut self) {
        let mut handle = self.device.open().unwrap();

        handle.claim_interface(0).unwrap();

        self.handle = Some(handle);
        
        // self.leave_state();
        self.connected = true;
    }

    pub fn disconnect(&mut self) {
        let mut handle = core::mem::replace(&mut self.handle, None).unwrap();

        handle.release_interface(0).unwrap();

        self.connected = false;
    }

    pub fn read(&mut self, buf : &mut [u8]) -> Option<usize> {

        assert!(buf.len() >= 64);
        assert!(buf.len() % 4 == 0);

        if let Some(ref mut handle) = self.handle {
            let n = handle.read_bulk(self.dev_type.in_pipe, buf, Duration::from_millis(200)).unwrap();

            Some(n)
        } else {
            None
        }
    }
    

    pub fn write(&mut self, buf : &[u8]) {

        if let Some(ref mut handle) = self.handle {
            let n = handle.write_bulk(self.dev_type.out_pipe, buf, Duration::from_millis(200)).unwrap();

            assert_eq!(buf.len(), n);
        } else {
            panic!()
        }
    }

    pub fn transfer(&mut self, cmd : &[u8], data : Option<&[u8]>, rx_buf : Option<&mut [u8]>) -> Option<usize> {
        let mut cmd_buf = [0u8; 16];

        cmd_buf[..(cmd.len())].copy_from_slice(cmd);

        self.write(&cmd_buf);

        if let Some(data) = data {
            self.write(data);
        }

        if let Some(rx_buf) = rx_buf {
            self.read(rx_buf)
        } else {
            None
        }
    }
}

impl Drop for STLink {
    fn drop(&mut self) {
        if self.connected {
            self.disconnect();
        }
    }
}

// STLINK V2 Implementation

const STLINK_GET_VERSION                  : u8 = 0xf1;
const STLINK_DEBUG_COMMAND                : u8 = 0xf2;
const STLINK_DFU_COMMAND                  : u8 = 0xf3;
const STLINK_SWIM_COMMAND                 : u8 = 0xf4;
const STLINK_GET_CURRENT_MODE             : u8 = 0xf5;
const STLINK_GET_TARGET_VOLTAGE           : u8 = 0xf7;
const STLINK_APIV3_GET_VERSION_EX         : u8 = 0xFB;

const STLINK_MODE_DFU                     : u8 = 0x00;
const STLINK_MODE_MASS                    : u8 = 0x01;
const STLINK_MODE_DEBUG                   : u8 = 0x02;
const STLINK_MODE_SWIM                    : u8 = 0x03;
const STLINK_MODE_BOOTLOADER              : u8 = 0x04;

const STLINK_DFU_EXIT                     : u8 = 0x07;

const STLINK_SWIM_ENTER                   : u8 = 0x00;
const STLINK_SWIM_EXIT                    : u8 = 0x01;

const STLINK_DEBUG_ENTER_JTAG             : u8 = 0x00;
const STLINK_DEBUG_STATUS                 : u8 = 0x01;
const STLINK_DEBUG_FORCEDEBUG             : u8 = 0x02;
const STLINK_DEBUG_APIV1_RESETSYS         : u8 = 0x03;
const STLINK_DEBUG_APIV1_READALLREGS      : u8 = 0x04;
const STLINK_DEBUG_APIV1_READREG          : u8 = 0x05;
const STLINK_DEBUG_APIV1_WRITEREG         : u8 = 0x06;
const STLINK_DEBUG_READMEM_32BIT          : u8 = 0x07;
const STLINK_DEBUG_WRITEMEM_32BIT         : u8 = 0x08;
const STLINK_DEBUG_RUNCORE                : u8 = 0x09;
const STLINK_DEBUG_STEPCORE               : u8 = 0x0a;
const STLINK_DEBUG_APIV1_SETFP            : u8 = 0x0b;
const STLINK_DEBUG_READMEM_8BIT           : u8 = 0x0c;
const STLINK_DEBUG_WRITEMEM_8BIT          : u8 = 0x0d;
const STLINK_DEBUG_APIV1_CLEARFP          : u8 = 0x0e;
const STLINK_DEBUG_APIV1_WRITEDEBUGREG    : u8 = 0x0f;
const STLINK_DEBUG_APIV1_SETWATCHPOINT    : u8 = 0x10;
const STLINK_DEBUG_APIV1_ENTER            : u8 = 0x20;
const STLINK_DEBUG_EXIT                   : u8 = 0x21;
const STLINK_DEBUG_READCOREID             : u8 = 0x22;
const STLINK_DEBUG_APIV2_ENTER            : u8 = 0x30;
const STLINK_DEBUG_APIV2_READ_IDCODES     : u8 = 0x31;
const STLINK_DEBUG_APIV2_RESETSYS         : u8 = 0x32;
const STLINK_DEBUG_APIV2_READREG          : u8 = 0x33;
const STLINK_DEBUG_APIV2_WRITEREG         : u8 = 0x34;
const STLINK_DEBUG_APIV2_WRITEDEBUGREG    : u8 = 0x35;
const STLINK_DEBUG_APIV2_READDEBUGREG     : u8 = 0x36;
const STLINK_DEBUG_APIV2_READALLREGS      : u8 = 0x3a;
const STLINK_DEBUG_APIV2_GETLASTRWSTATUS  : u8 = 0x3b;
const STLINK_DEBUG_APIV2_DRIVE_NRST       : u8 = 0x3c;
const STLINK_DEBUG_SYNC                   : u8 = 0x3e;
const STLINK_DEBUG_APIV2_START_TRACE_RX   : u8 = 0x40;
const STLINK_DEBUG_APIV2_STOP_TRACE_RX    : u8 = 0x41;
const STLINK_DEBUG_APIV2_GET_TRACE_NB     : u8 = 0x42;
const STLINK_DEBUG_APIV2_SWD_SET_FREQ     : u8 = 0x43;
const STLINK_DEBUG_APIV2_READMEM_16BIT    : u8 = 0x47;
const STLINK_DEBUG_APIV2_WRITEMEM_16BIT   : u8 = 0x48;

const STLINK_DEBUG_ENTER_SWD              : u8 = 0xa3;

const STLINK_DEBUG_APIV3_SET_COM_FREQ     : u8 = 0x61;
const STLINK_DEBUG_APIV3_GET_COM_FREQ     : u8 = 0x62;

const STLINK_DEBUG_APIV2_DRIVE_NRST_LOW   : u8 = 0x00;
const STLINK_DEBUG_APIV2_DRIVE_NRST_HIGH  : u8 = 0x01;
const STLINK_DEBUG_APIV2_DRIVE_NRST_PULSE : u8 = 0x02;

// const STLINK_DEBUG_APIV2_SWD_SET_FREQ_MAP : u8 = {
//     4000000: 0,
//     1800000: 1,  // default
//     1200000: 2,
//     950000:  3,
//     480000:  7,
//     240000: 15,
//     125000: 31,
//     100000: 40,
//     50000:  79,
//     25000: 158,
//     # 15000: 265,
//     # 5000:  798
// }

#[repr(u8)]
pub enum StlinkDebugApiv2SwdFreq {
    Freq4000000 = 0,
    Freq1800000 = 1,
    Freq1200000 = 2,
    Freq950000  = 3,
    Freq480000  = 7,
    Freq240000  = 15,
    Freq125000  = 31,
    Freq100000  = 40,
    Freq50000   = 79,
    Freq25000   = 158,
}

const STLINK_MAXIMUM_TRANSFER_SIZE        : usize = 1024;

impl STLink {

    pub fn init(&mut self) {
    }

    pub fn leave_state(&mut self) {
        let mut rx_buf = [0u8; 64];

        let n = self.transfer(&[STLINK_GET_CURRENT_MODE], None, Some(&mut rx_buf));

        assert_eq!(n, Some(2));

        match rx_buf[0] {
            n if n == STLINK_MODE_DFU   => {self.transfer(&[STLINK_DFU_COMMAND,   STLINK_DFU_EXIT  ], None, None);}
            n if n == STLINK_MODE_DEBUG => {self.transfer(&[STLINK_DEBUG_COMMAND, STLINK_DEBUG_EXIT], None, None);}
            n if n == STLINK_MODE_SWIM  => {self.transfer(&[STLINK_SWIM_COMMAND,  STLINK_SWIM_EXIT ], None, None);}
            _ => panic!()
        }
    }

    pub fn set_swd_freq(&mut self, freq : StlinkDebugApiv2SwdFreq) {
        let mut rx_buf = [0u8; 64];
        self.transfer(&[STLINK_DEBUG_COMMAND, STLINK_DEBUG_APIV2_SWD_SET_FREQ, freq as u8], None, Some(&mut rx_buf));

        assert_eq!(rx_buf[0], 0x80);
    }

    pub fn enter_debug_swd(&mut self) {
        let mut rx_buf = [0u8; 64];
        self.transfer(&[STLINK_DEBUG_COMMAND, STLINK_DEBUG_APIV2_ENTER, STLINK_DEBUG_ENTER_SWD], None, Some(&mut rx_buf));
    }
    
    pub fn debug_resetsys(&mut self) {
        let mut rx_buf = [0u8; 64];
        self.transfer(&[STLINK_DEBUG_COMMAND, STLINK_DEBUG_APIV2_RESETSYS], None, Some(&mut rx_buf));
    }
    
    pub fn get_mem32(&mut self, addr : u32, size : u32) -> Vec<u8> {

        assert!(addr % 4 == 0);
        assert!(size % 4 == 0);
        assert!(size <= STLINK_MAXIMUM_TRANSFER_SIZE as u32);

        let mut cmd = [STLINK_DEBUG_COMMAND, STLINK_DEBUG_READMEM_32BIT, 0,0,0,0, 0,0,0,0];
        cmd[2..6].copy_from_slice(&addr.to_le_bytes());
        cmd[6..10].copy_from_slice(&size.to_le_bytes());

        let mut rx_buf = core::iter::repeat(0u8).take(size.max(64) as usize).collect::<Vec<_>>();

        let n = self.transfer(&cmd, None, Some(&mut rx_buf)).unwrap();

        rx_buf.truncate(n);

        rx_buf
    }
    
    pub fn set_mem32(&mut self, addr : u32, data : &[u8]) {

        let size = data.len() as u32;

        assert!(addr % 4 == 0);
        assert!(size % 4 == 0);
        assert!(size <= STLINK_MAXIMUM_TRANSFER_SIZE as u32);

        let mut cmd = [STLINK_DEBUG_COMMAND, STLINK_DEBUG_WRITEMEM_32BIT, 0,0,0,0, 0,0,0,0];
        cmd[2..6 ].copy_from_slice(&addr.to_le_bytes());
        cmd[6..10].copy_from_slice(&size.to_le_bytes());

        self.transfer(&cmd, Some(data), None);
    }
    
    pub fn get_mem16(&mut self, addr : u32, size : u32) -> Vec<u8> {

        assert!(addr % 2 == 0);
        assert!(size % 2 == 0);
        assert!(size <= STLINK_MAXIMUM_TRANSFER_SIZE as u32);

        let mut cmd = [STLINK_DEBUG_COMMAND, STLINK_DEBUG_APIV2_READMEM_16BIT, 0,0,0,0, 0,0,0,0];
        cmd[2..6].copy_from_slice(&addr.to_le_bytes());
        cmd[6..10].copy_from_slice(&size.to_le_bytes());

        let mut rx_buf = core::iter::repeat(0u8).take(size as usize).collect::<Vec<_>>();

        let n = self.transfer(&cmd, None, Some(&mut rx_buf)).unwrap();

        rx_buf.truncate(n);

        rx_buf
    }
    
    pub fn set_mem16(&mut self, addr : u32, size : u32, data : &[u8]) {

        assert!(addr % 2 == 0);
        assert!(size % 2 == 0);
        assert!(size <= STLINK_MAXIMUM_TRANSFER_SIZE as u32);

        let mut cmd = [STLINK_DEBUG_COMMAND, STLINK_DEBUG_APIV2_WRITEMEM_16BIT, 0,0,0,0, 0,0,0,0];
        cmd[2..6 ].copy_from_slice(&addr.to_le_bytes());
        cmd[6..10].copy_from_slice(&size.to_le_bytes());

        self.transfer(&cmd, Some(data), None);
    }

    pub fn read_struct<T : Clone>(&mut self, addr : u32) -> T {

        let mut len = size_of::<T>();
        let mut buffer = Vec::with_capacity(len);
        let mut offset = 0;

        loop {
            let n = len.min(STLINK_MAXIMUM_TRANSFER_SIZE);
            let data = self.get_mem32(addr + offset, n as u32);

            buffer.extend(data.into_iter());

            if len < STLINK_MAXIMUM_TRANSFER_SIZE {
                break;
            }

            len -= STLINK_MAXIMUM_TRANSFER_SIZE;
            offset += STLINK_MAXIMUM_TRANSFER_SIZE as u32;
        }
        
        unsafe {
            let (head, item, _) = buffer.align_to::<T>();

            assert!(head.is_empty());

            item[0].clone()
        }
    }

    pub fn read_struct_array<T : Clone>(&mut self, addr : u32, len : u32) -> Vec<T> {


        let mut len = size_of::<T>() * len as usize;
        let mut buffer = Vec::with_capacity(len);
        let mut offset = 0;

        loop {
            let n = len.min(STLINK_MAXIMUM_TRANSFER_SIZE);
            let data = self.get_mem32(addr + offset, n as u32);

            buffer.extend(data.into_iter());

            if len < STLINK_MAXIMUM_TRANSFER_SIZE {
                break;
            }

            len -= STLINK_MAXIMUM_TRANSFER_SIZE;
            offset += STLINK_MAXIMUM_TRANSFER_SIZE as u32;
        }
        

        unsafe {
            let (head, items, tail) = buffer.align_to::<T>();

            assert!(head.is_empty());
            assert!(tail.is_empty());

            items.to_vec()
        }
    }
    
    pub fn read_struct_array_with_offset<T : Clone>(&mut self, addr : u32, len : u32, offset : u32) -> Vec<T> {


        let mut len = size_of::<T>() * len as usize;
        let mut buffer = Vec::with_capacity(len);
        let mut offset = size_of::<T>() as u32 * offset;

        loop {
            let n = len.min(STLINK_MAXIMUM_TRANSFER_SIZE);
            let data = self.get_mem32(addr + offset, n as u32);

            buffer.extend(data.into_iter());

            if len < STLINK_MAXIMUM_TRANSFER_SIZE {
                break;
            }

            len -= STLINK_MAXIMUM_TRANSFER_SIZE;
            offset += STLINK_MAXIMUM_TRANSFER_SIZE as u32;
        }
        

        unsafe {
            let (head, items, tail) = buffer.align_to::<T>();

            assert!(head.is_empty());
            assert!(tail.is_empty());

            items.to_vec()
        }
    }

    pub fn write_struct<T>(&mut self, addr : u32, item : T) {

        let array = [item];
        let (_, data, _) = unsafe{ array.align_to::<u8>()};

        self.set_mem32(addr, data);
    }
    
    pub fn write_struct_array<T>(&mut self, addr : u32, items : &[T]) {

        let (_, buffer, _) = unsafe{items.align_to::<u8>()};
        let mut offset = 0usize;
        let addr_offset = 0usize;
        let mut len = buffer.len();

        loop {
            let n = len.min(STLINK_MAXIMUM_TRANSFER_SIZE);
            self.set_mem32(addr + addr_offset as u32 + offset as u32, &buffer[offset..(offset+n)]);

            if len < STLINK_MAXIMUM_TRANSFER_SIZE {
                break;
            }

            len -= STLINK_MAXIMUM_TRANSFER_SIZE;
            offset += STLINK_MAXIMUM_TRANSFER_SIZE;
        }
    }
    
    pub fn write_struct_array_offset<T>(&mut self, addr : u32, offset : u32, items : &[T]) {
        
        let (_, buffer, _) = unsafe{items.align_to::<u8>()};
        let addr = addr + offset * size_of::<T>() as u32;
        let mut offset = 0usize;
        let mut len = buffer.len();

        loop {
            let n = len.min(STLINK_MAXIMUM_TRANSFER_SIZE);
            self.set_mem32(addr + offset as u32, &buffer[offset..(offset+n)]);

            if len < STLINK_MAXIMUM_TRANSFER_SIZE {
                break;
            }

            len -= STLINK_MAXIMUM_TRANSFER_SIZE;
            offset += STLINK_MAXIMUM_TRANSFER_SIZE;
        }
    }
}