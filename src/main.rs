use clap::Parser;
use nusb::DeviceInfo;
use std::{error::Error, fs::OpenOptions, io::Write, process};

/// Control the brightness of a Waveshare WS170120 display
#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(disable_help_flag = true)]
struct Args {
    /// Brightness percentage (0-100)
    #[arg(value_parser = clap::value_parser!(u8).range(0..=100))]
    brightness: u8,

    /// Increase verbosity
    #[arg(short, long, action = clap::ArgAction::Count)]
    verbose: u8,

    /// Print help
    #[arg(short = '?', short_alias = 'h', long = "help", action = clap::ArgAction::Help)]
    help: Option<bool>,
}

const WS170120_VENDOR_ID: u16 = 0x0eef;
const WS170120_PRODUCT_ID: u16 = 0x0005;
const DATA_LENGTH: usize = 38;
const BRIGHTNESS_ADDRESS: usize = 6;
const CONTROL_MAGIC: [u8; 4] = [0x04, 0xaa, 0x01, 0x00];

fn find_ws170120_device() -> Result<DeviceInfo, String> {
    let devices = nusb::list_devices().map_err(|e| format!("Failed to list USB devices: {}", e))?;

    for device in devices {
        if device.vendor_id() == WS170120_VENDOR_ID && device.product_id() == WS170120_PRODUCT_ID {
            return Ok(device);
        }
    }

    Err("Waveshare monitor WS170120 is not connected.".to_string())
}

fn find_hidraw_device() -> Result<String, String> {
    use std::fs;
    use std::path::Path;

    // First verify the device exists via USB enumeration
    let _device_info = find_ws170120_device()?;

    // On macOS and Linux, look for hidraw devices
    let sys_devices_path = "/sys/class/hidraw";
    if Path::new(sys_devices_path).exists() {
        // Linux approach - scan /sys/class/hidraw
        let entries = fs::read_dir(sys_devices_path)
            .map_err(|e| format!("Failed to read {}: {}", sys_devices_path, e))?;

        for entry in entries {
            let entry = entry.map_err(|e| format!("Failed to read directory entry: {}", e))?;
            let hidraw_name = entry.file_name();
            let hidraw_path = format!("/dev/{}", hidraw_name.to_string_lossy());

            // Check if this hidraw device belongs to our WS170120
            let device_path = entry.path().join("device");
            if let Ok(uevent_path) = fs::canonicalize(device_path.join("uevent")) {
                if let Ok(uevent_content) = fs::read_to_string(uevent_path) {
                    if uevent_content.contains(&format!(
                        "HID_ID=0003:{:04X}:{:04X}",
                        WS170120_VENDOR_ID, WS170120_PRODUCT_ID
                    )) {
                        return Ok(hidraw_path);
                    }
                }
            }
        }
    }

    // macOS approach - scan /dev for potential HID devices
    #[cfg(target_os = "macos")]
    {
        use std::fs;

        // On macOS, we need to find the device in /dev
        // The device enumeration confirmed it exists via USB
        let dev_entries =
            fs::read_dir("/dev").map_err(|e| format!("Failed to read /dev directory: {}", e))?;

        for entry in dev_entries {
            if let Ok(entry) = entry {
                let name = entry.file_name();
                let name_str = name.to_string_lossy();

                // Look for devices that might be our HID device
                // On macOS, these are often named like /dev/hidg*, /dev/usb*, or similar
                if name_str.starts_with("hidg")
                    || name_str.starts_with("usb")
                    || name_str.contains("hid")
                {
                    let device_path = format!("/dev/{}", name_str);

                    // Try to open the device to see if it's accessible
                    if let Ok(_) = fs::File::open(&device_path) {
                        // For now, try the first accessible HID-like device
                        // This is a simplified approach - in production you'd want
                        // to verify this is actually the WS170120
                        return Ok(device_path);
                    }
                }
            }
        }

        return Err("Could not find accessible HID device on macOS. The device may require different permissions or a different approach.".to_string());
    }

    Err("Could not find hidraw device for WS170120.".to_string())
}

async fn set_brightness_direct_write(
    device_path: &str,
    brightness: u8,
    verbose: u8,
) -> Result<(), String> {
    // Prepare the data buffer exactly like the Python version
    let mut data_buffer = [0u8; DATA_LENGTH];
    data_buffer[..CONTROL_MAGIC.len()].copy_from_slice(&CONTROL_MAGIC);
    data_buffer[BRIGHTNESS_ADDRESS] = brightness;

    // Open device file directly and write data
    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        .open(device_path)
        .map_err(|e| {
            let err_str = e.to_string().to_lowercase();
            if err_str.contains("permission denied") || err_str.contains("access denied") {
                format!(
                    "Device access denied: {}. Try running with elevated privileges (sudo).",
                    e
                )
            } else {
                format!("Failed to open device {}: {}", device_path, e)
            }
        })?;

    let bytes_written = file
        .write(&data_buffer)
        .map_err(|e| format!("Failed to write brightness data: {}", e))?;

    if bytes_written != DATA_LENGTH {
        return Err(format!(
            "Unexpected result {} from writing brightness data, expected {}.",
            bytes_written, DATA_LENGTH
        ));
    }

    file.flush()
        .map_err(|e| format!("Failed to flush data to device: {}", e))?;

    if verbose > 0 {
        println!("Brightness has been set to {}%.", brightness);
    }

    Ok(())
}

async fn set_brightness_usb(
    device_info: &DeviceInfo,
    brightness: u8,
    verbose: u8,
) -> Result<(), String> {
    let device = device_info
        .open()
        .map_err(|e| format!("Failed to open device: {}", e))?;

    // Prepare the data buffer
    let mut data_buffer = [0u8; DATA_LENGTH];
    data_buffer[..CONTROL_MAGIC.len()].copy_from_slice(&CONTROL_MAGIC);
    data_buffer[BRIGHTNESS_ADDRESS] = brightness;

    // Try control transfer without claiming interface
    let transfer = nusb::transfer::ControlOut {
        control_type: nusb::transfer::ControlType::Class,
        recipient: nusb::transfer::Recipient::Interface,
        request: 0x09, // HID Set Report
        value: 0x0200, // Report Type: Output (0x02), Report ID: 0x00
        index: 0,      // Interface number
        data: &data_buffer,
    };

    let result = device.control_out(transfer).await;

    match result.status {
        Ok(()) => {
            if result.data.actual_length() != DATA_LENGTH {
                return Err(format!(
                    "Unexpected result {} from writing brightness data, expected {}.",
                    result.data.actual_length(),
                    DATA_LENGTH
                ));
            }
            if verbose > 0 {
                println!("Brightness has been set to {}%.", brightness);
            }
            Ok(())
        }
        Err(e) => Err(format!(
            "Failed to write brightness data via control transfer: {}",
            e
        )),
    }
}

async fn set_brightness(brightness: u8, verbose: u8) -> Result<(), String> {
    if verbose > 0 {
        println!("Attempting to set brightness to {}%.", brightness);
    }

    // First, try to find and use hidraw device (like Python version)
    match find_hidraw_device() {
        Ok(device_path) => {
            if verbose > 1 {
                println!("Found hidraw device: {}", device_path);
            }
            return set_brightness_direct_write(&device_path, brightness, verbose).await;
        }
        Err(e) => {
            if verbose > 1 {
                println!("Could not find hidraw device: {}", e);
                println!("Falling back to USB control transfer...");
            }
        }
    }

    // Fallback to USB control transfer
    let device_info = find_ws170120_device()?;
    set_brightness_usb(&device_info, brightness, verbose).await
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    if let Err(e) = set_brightness(args.brightness, args.verbose).await {
        eprintln!("{}", e);
        process::exit(1);
    }
}
