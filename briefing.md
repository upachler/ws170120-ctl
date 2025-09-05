The goal is to create a command line application in Rust that can control the brightness of a Waveshare 17 inch LCD (C) display (internally called WS170120). The display is connected by HDMI and USB. The brightness of the display can be controlled via the USB port.

Previous work has already been done in Python: Someone reverse engineered the USB protocol for the display and created a script in python to adjust the brightness. This script won't work on MacOS.

Consider the python script here:
https://github.com/rotdrop/waveshare-ws170120-brightness/blob/main/waveshare_ws170120_brightness/__init__.py

It contains important information about the USB vendor ID of WS170120 and also about the data to send. The rust program should do this in the same manner.

The Rust program should be based on the `nusb` crate as it offers cross platform USB functionality (https://docs.rs/nusb/latest/nusb/).

The program should have the following commandline interface:

`ws170120-ctl <brightness>`, where
* `ws170120-ctl` is the name of the program
* `<brightness>` is an integer value between 0 and 100

If the program cannot find the device on the USB bus, it prints a respective error message and terminates with error code 1.

If the the provided `<brightness>` value is out of range, it prints an error message.

The `-?` switch prints the program's usage, which is also printed if no parameters are given.
