use std::io::Write;
use std::time::{Duration, Instant};
use std::sync::mpsc::Sender;

use std::ffi::CString;

use nix::libc;
use rusb::{Device, DeviceHandle, GlobalContext};

use crate::controller::{Controller, update_controllers};

pub struct ControllerPoll {
    pub buffer: [u8; 37],
    pub time: Instant,
}

pub fn start_adapter_polling(sender: Sender<ControllerPoll>) {
    for device in rusb::devices().unwrap().iter() {
        let device_desc = device.device_descriptor().unwrap();
        println!("Bus {:03} Device {:03} ID {:04x}:{:04x}",
            device.bus_number(),
            device.address(),
            device_desc.vendor_id(),
            device_desc.product_id());

        if is_gc_adapter(&device) {
            println!("found gc adapter");
            let device = match start_gc_adapter(device, &sender) {
                Err(err) => {
                    println!("error: {:?}", err);
                    println!("source: {:?}", err.source());
                    println!("description: {:?}", err.to_string());
                    break;
                },
                Ok(device) => {
                    device
                },
            };
            match try_snoop_usb(device, &sender) {
                Ok(_) => println!("returned ok?"),
                Err(err) => {
                    println!("error in usb snoop {:?}, description {}", err, err.to_string());
                }
            }

            break;
        }
    }
}

#[repr(C)]
#[derive(Default, Debug)]
struct UsbmonPacket {
    id: u64,			/*  0: URB ID - from submission to callback */
	type_: libc::c_char,	/*  8: Same as text; extensible. */
	xfer_type: u8, /*    ISO (0), Intr, Control, Bulk (3) */
	epnum: u8,	/*     Endpoint number and transfer direction */
	devnum: u8,	/*     Device address */
	busnum: u16,		/* 12: Bus number */
	flag_setup: i8,	/* 14: Same as text */
	flag_data: i8,		/* 15: Same as text; Binary zero is OK. */
	ts_sec: u64,		/* 16: gettimeofday */
	ts_usec: u32,		/* 24: gettimeofday */
	status: i32,		/* 28: */
	length: u32,	/* 32: Length of data (submitted or actual) */
	len_cap: u32,	/* 36: Delivered length */
    /*
	union {			/* 40: */
		unsigned char setup[SETUP_LEN],	/* Only for Control S-type */
		struct iso_rec {		/* Only for ISO */
			int error_count,
			int numdesc,
		} iso;
	} s;
    */
    s: u64,
	interval: i32,		/* 48: Only for Interrupt and ISO */
	start_frame: i32,	/* 52: For ISO */
	xfer_flags: u32, /* 56: copy of URB's transfer_flags */
	ndesc: u32,	/* 60: Actual number of ISO descriptors */
}				/* 64 total length */ 

#[repr(C)]
pub struct MonGetArg {
    hdr: *mut UsbmonPacket,
    //data: *mut u8,
    data: *mut nix::libc::c_void,
    alloc: nix::libc::size_t,
    //void *data;
    //size_t alloc;       /* Length of data (can be zero) */
}

//cursed c shit
fn try_snoop_usb(device: Device<GlobalContext>, sender: &Sender<ControllerPoll>) -> Result<(), Box<dyn std::error::Error>> {
    unsafe {
        let path = CString::new("/dev/usbmon0")?;
        let usbmon_file = nix::libc::open(path.as_ptr(), 0);
        println!("opened usbmon file, fd {} errno {}", usbmon_file, nix::errno::errno());
        if usbmon_file == -1 && nix::errno::errno() == nix::libc::EACCES {
            Err("usb sniffing failed, probably because we're not root")?;
        }

        const MON_IOC_MAGIC: u64 = 0x92;
        //https://www.kernel.org/doc/Documentation/usb/usbmon.txt
        nix::ioctl_write_ptr!(mon_iocx_getx, MON_IOC_MAGIC, 10, MonGetArg);
        let mut packet_info = Default::default();
        let mut data = [0u8; 37];
        let mut event = MonGetArg {
            hdr: (&mut packet_info),
            data: data.as_mut_ptr() as *mut nix::libc::c_void,
            alloc: data.len(),
        };
        let mut unix_epoch = None;
        loop {
            let res = mon_iocx_getx(usbmon_file, &mut event);
            if let None = unix_epoch {
                unix_epoch = Some(Instant::now() - Duration::from_secs(packet_info.ts_sec) - Duration::from_micros(packet_info.ts_usec as u64));
                println!("first data {:?}", data);
                println!("cfg {:?}", packet_info);
            }
            //I think this filters everything?
            if packet_info.devnum == device.address() && packet_info.busnum == device.bus_number() as u16 && packet_info.type_ == 'C' as i8 {
                //println!("got data {:?}", data);
                if data[4] == 0 || data[5] == 0 || data[6] == 0 || data[7] == 0 {
                    println!("wrong data? {:?}", data);
                    println!("cfg {:?}", packet_info);
                }

                sender.send(ControllerPoll {
                    buffer: data,
                    time: unix_epoch.unwrap() + Duration::from_secs(packet_info.ts_sec) + Duration::from_micros(packet_info.ts_usec as u64)
                })?;
                if let Err(e) = res {
                    println!("error {:?}", e);
                }
            }
        }
    }
}

fn start_gc_adapter(device: Device<GlobalContext>, sender: &Sender<ControllerPoll>) -> Result<Device<GlobalContext>, Box<dyn std::error::Error>> {
    println!("device speed {:?}", device.speed());
    let mut handle = device.open()?;
    println!("{:?}", handle);
    let config = device.config_descriptor(0)?;
    let mut endpoint_in = 0;
    let mut endpoint_out = 0;
    let mut interface_to_claim = 0;
    for interface in config.interfaces() {
        for descriptor in interface.descriptors() {
            for endpoint_descriptor in descriptor.endpoint_descriptors() {
                println!("endpoint found {:?}", endpoint_descriptor);
                if endpoint_descriptor.address() & rusb::constants::LIBUSB_ENDPOINT_IN != 0 {
                    endpoint_in = endpoint_descriptor.address();
                    interface_to_claim = interface.number();
                }
                else {
                    endpoint_out = endpoint_descriptor.address();
                }
            }
        }
    }
    device.address();
    let res = handle.claim_interface(interface_to_claim);
    if let Err(err) = res {
        if let rusb::Error::Busy = err {
            return Ok(device)
        }
        else {
            Err(err)?;
        }
    }
    handle.write_interrupt(endpoint_out, &[0x13], Duration::from_millis(32))?;
    poll_loop(handle, endpoint_in, sender)
}

pub fn is_gc_adapter(device: &Device<GlobalContext>) -> bool {
    if let Ok (device_desc) = device.device_descriptor() {
        device_desc.vendor_id() == 0x057e && device_desc.product_id() == 0x0337
    }
    else {
        false
    }
}

pub fn poll_loop(handle: DeviceHandle<GlobalContext>, endpoint_in: u8, sender: &Sender<ControllerPoll>) -> Result<Device<GlobalContext>, Box<dyn std::error::Error>> {
    let mut controllers = [Controller::new(); 4];
    let mut time = Instant::now();
    let mut time_diff = 0;
    let mut last_print = Instant::now();
    let mut err_count = 0;
    loop {
        let mut buffer = [0u8; 37];
        let res = handle.read_interrupt(endpoint_in, &mut buffer, Duration::from_millis(32));
        let now = Instant::now();
        if let Err(res) = res {
            if err_count >= 10 {
                return Err(Box::new(res));
            }
            err_count += 1;
            println!("error reading: {:?}", res);
        }
        else {
            err_count = 0;
            sender.send(ControllerPoll { buffer, time: now })?;
        }
        let new_time = Instant::now();
        if new_time - last_print > Duration::from_millis(100) {
            update_controllers(&mut controllers, &buffer);
            print!("\rbtns: [{:<20}] stk: [{:<4?}] c: [{:<4?}]", controllers[0].to_string(), controllers[0].stick_clamp(), controllers[0].c_stick_clamp());
            print!("p2: [{:<20}] stk: [{:<4?}] c: [{:<4?}]", controllers[1].to_string(), controllers[1].stick_clamp(), controllers[1].c_stick_clamp());
            print!("time: {:>4}", time_diff);
            std::io::stdout().flush()?;
            last_print = new_time;
        }

        //sleep(Duration::from_micros(1000));
        time_diff = (new_time - time).as_micros();
        time = new_time;
    }
    //Ok(())
}
