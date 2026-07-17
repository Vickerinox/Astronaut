# Crates Folder

This folder contains the additional crates that drive astronaut. They are all licensed under the MIT license (as opposed to astronaut as a whole which is licensed under GPLv3) and is astronauts equivalent to a toolchain.

### common

Common contains the most basic DSi specific info which has no dependencies and is used by all components.

### micro_imgui

micro_imgui is a tiny gui framework inspired by egui designed to have a small footprint and rich functionality. With support for touch controls, navigation via buttons, and styling. It is also designed to be backend agnostic such as that it can be used on various platforms of a smaller stature

### micro_imgui_ds

micro_imgui_ds is an micro_imgui backend that works for the DS. This is what allows astronaut to use micro_imgui.

### reboot_lib

reboot_lib is astronauts core logic crate akin to libnds. It contains functions and definitions for the hardware features of the DS/DSi.