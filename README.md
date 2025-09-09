# ws170120-ctl

A command-line utility to control the brightness of a Waveshare WS170120 17-inch LCD touch display.

## Description

This Rust application provides cross-platform USB communication to adjust the brightness of Waveshare WS170120 displays. It uses the `hidapi` crate for cross-platform USB functionality and implements the same USB protocol as the original Python implementation.

## Features

- Cross-platform USB communication (Windows, macOS, Linux)
- Simple command-line interface
- Brightness control from 0-100%
- Verbose output option
- Proper error handling and user feedback

## Installation

### Building from Source

1. Install Rust from [rustup.rs](https://rustup.rs/)
2. Clone or download this repository
3. Build the application:

```bash
cargo build --release
```

The compiled binary will be available at `target/release/ws170120-ctl`

## Usage

```
ws170120-ctl <BRIGHTNESS>

Arguments:
  <BRIGHTNESS>  Set the brightness value to the given percentage (0-100)

Options:
  -v, --verbose...  Increase verbosity
  -?, --help        Print help
  -V, --version     Print version
```

### Examples

Set brightness to 75%:
```bash
ws170120-ctl 75
```

Set brightness to 50% with verbose output:
```bash
ws170120-ctl -v 50
```

Show help:
```bash
ws170120-ctl -?
ws170120-ctl --help
```

## Requirements

- A Waveshare WS170120 display connected via USB
- Appropriate USB permissions (may require running with elevated privileges)

### USB Permissions

On Linux and macOS, you may need to run the application with elevated privileges:

```bash
sudo ws170120-ctl 75
```

Alternatively, you can set up udev rules on Linux to allow non-root access to the device.

## Device Detection

The application automatically detects Waveshare WS170120 displays by their USB vendor ID (0x0eef) and product ID (0x0005). If the device is not found, the application will exit with error code 1 and display an appropriate error message.

## Error Handling

The application handles several error conditions:

- **Device not found**: "Waveshare monitor WS170120 is not connected."
- **Invalid brightness value**: Values outside 0-100 range are rejected
- **USB access issues**: Suggests running with elevated privileges
- **Communication errors**: Reports specific USB transfer failures

All errors result in exit code 1.

## Technical Details

The application implements the same USB communication protocol as the original Python version:

- Uses HID (Human Interface Device) protocol
- Sends 38-byte control messages
- Control magic bytes: `[0x04, 0xaa, 0x01, 0x00]`
- Brightness value is written to byte offset 6
- Supports both control transfers and interrupt transfers as fallback

## License

This project is provided as-is for educational and practical purposes.

## Contributing

Feel free to submit issues or pull requests to improve the application.
