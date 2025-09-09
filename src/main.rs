use clap::Parser;
use hidapi::{HidApi, HidDevice};
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

fn find_ws170120_device(api: &HidApi) -> Result<HidDevice, String> {
    let device_info = api
        .device_list()
        .find(|info| {
            info.vendor_id() == WS170120_VENDOR_ID && info.product_id() == WS170120_PRODUCT_ID
        })
        .ok_or_else(|| "Waveshare monitor WS170120 is not connected.".to_string())?;

    device_info
        .open_device(api)
        .map_err(|e| translate_device_error("opening device failed", e))
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

fn set_brightness(device: &HidDevice, brightness: u8, verbose: u8) -> Result<(), String> {
    // Brightness validation is now handled by clap's value parser

    // Prepare the data buffer
    let mut data_buffer = [0u8; DATA_LENGTH];
    data_buffer[..CONTROL_MAGIC.len()].copy_from_slice(&CONTROL_MAGIC);
    data_buffer[BRIGHTNESS_ADDRESS] = brightness;

    // For HID devices, we use write to send the report
    let result = device.write(&data_buffer);

    match result {
        Ok(bytes_written) => {
            if bytes_written != DATA_LENGTH {
                return Err(format!(
                    "Unexpected result {} from writing brightness data, expected {}.",
                    bytes_written, DATA_LENGTH
                ));
            }
            if verbose > 0 {
                println!("Brightness has been set to {}%.", brightness);
            }
            Ok(())
        }
        Err(e) => {
            // If regular write fails, try send_feature_report as fallback
            if verbose > 0 {
                println!("Regular write failed, trying feature report...");
            }

            let feature_result = device.send_feature_report(&data_buffer);

            match feature_result {
                Ok(()) => {
                    if verbose > 0 {
                        println!("Brightness has been set to {}%.", brightness);
                    }
                    Ok(())
                }
                Err(e2) => Err(format!(
                    "Failed to write brightness data via both regular write and feature report. Write error: {}, Feature report error: {}",
                    e, e2
                )),
            }
        }
    }
}

fn main() {
    let args = Args::parse();

    if args.verbose > 0 {
        println!("Attempting to set brightness to {}%.", args.brightness);
    }

    // Initialize HID API
    let api = match HidApi::new() {
        Ok(api) => api,
        Err(e) => {
            eprintln!("Failed to initialize HID API: {}", e);
            process::exit(1);
        }
    };

    let device = match find_ws170120_device(&api) {
        Ok(device) => device,
        Err(e) => {
            eprintln!("{}", e);
            process::exit(1);
        }
    };

    if let Err(e) = set_brightness(&device, args.brightness, args.verbose) {
        eprintln!("{}", e);
        process::exit(1);
    }
}
