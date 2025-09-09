use clap::Parser;
use nusb::DeviceInfo;
use std::{error::Error, process};

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

fn translate_device_error(title: &str, e: impl Error) -> String {
    let err_str = e.to_string().to_lowercase();
    if err_str.contains("access denied")
        || err_str.contains("exclusive access")
        || err_str.contains("permission denied")
    {
        format!("{title}: Device access denied. Try running with elevated privileges (sudo). Error message was {e:?}")
    } else {
        format!("{title}: Failed to open device: {e}")
    }
}

async fn set_brightness(
    device_info: &DeviceInfo,
    brightness: u8,
    verbose: u8,
) -> Result<(), String> {
    // Brightness validation is now handled by clap's value parser

    let device = device_info
        .open()
        .map_err(|e| translate_device_error("opening device failed", e))?;

    // Claim the HID interface (typically interface 0)
    let interface = device
        .claim_interface(0)
        .map_err(|e| translate_device_error("claim_interface on device failed", e))?;

    // Prepare the data buffer
    let mut data_buffer = [0u8; DATA_LENGTH];
    data_buffer[..CONTROL_MAGIC.len()].copy_from_slice(&CONTROL_MAGIC);
    data_buffer[BRIGHTNESS_ADDRESS] = brightness;

    // For HID devices, we use control transfers (HID Set Report)
    let transfer = nusb::transfer::ControlOut {
        control_type: nusb::transfer::ControlType::Class,
        recipient: nusb::transfer::Recipient::Interface,
        request: 0x09, // HID Set Report
        value: 0x0200, // Report Type: Output (0x02), Report ID: 0x00
        index: 0,      // Interface number
        data: &data_buffer,
    };

    let result = interface.control_out(transfer).await;

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
        Err(e) => {
            // If control transfer fails, try interrupt transfer as fallback
            if verbose > 0 {
                println!("Control transfer failed, trying interrupt transfer...");
            }

            // Try interrupt out transfer
            let interrupt_result = interface.interrupt_out(0x01, data_buffer.to_vec()).await;

            match interrupt_result.status {
                Ok(()) => {
                    if interrupt_result.data.actual_length() != DATA_LENGTH {
                        return Err(format!(
                            "Unexpected result {} from writing brightness data, expected {}.",
                            interrupt_result.data.actual_length(), DATA_LENGTH
                        ));
                    }
                    if verbose > 0 {
                        println!("Brightness has been set to {}%.", brightness);
                    }
                    Ok(())
                }
                Err(e2) => Err(format!("Failed to write brightness data via both control and interrupt transfers. Control error: {}, Interrupt error: {}", e, e2)),
            }
        }
    }
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    if args.verbose > 0 {
        println!("Attempting to set brightness to {}%.", args.brightness);
    }

    let device_info = match find_ws170120_device() {
        Ok(device) => device,
        Err(e) => {
            eprintln!("{}", e);
            process::exit(1);
        }
    };

    if let Err(e) = set_brightness(&device_info, args.brightness, args.verbose).await {
        eprintln!("{}", e);
        process::exit(1);
    }
}
