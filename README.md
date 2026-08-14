# Astronaut
Astronaut is a custom stage2 firmware made exclusively for DSi consoles. Which is an alternative Nocash's Unlaunch. 
It uses the same primary exploit as unlaunch, wherein it hijacks the console while loading the TMD data for the DSi Menu. Unlocking all features of the console for homebrew use.

## Licensing
Astronaut is primarily licensed under the GPL version 3 license, with exception to the cargo crates found in the `crates` directory on the root of this repository. These crates instead use the MIT license. For specifics, please check the top of a given source file to know it's licensing.

## Features
* Launch DSi Compatible Homebrew and DSiWare software from the DSi SD card and DSi NAND
* A tiny file explorer GUI with touchscreen support and themeing
* Selective Autobooting via button combos during reset/startup
* Patching of the DSi menu to nullify any anti-tampering and region locking checks. (Currently required)

## Limitations
* ROMS may only occupy address 0x2000000 to 0x2ffffff and the arm 7 may additionally use 0x37F8000 to 0x3790000, any other binary locations can't be loaded.
* DS Mode roms have no audio, as the codec chip isn't initialized for them yet.
* Wifi initialization is slow and unstable. (Use the `Wifi Firmware Upload` option to toggle it)

### Known problematic titles
* DSi System Settings - Shows ``An Error Has Occured.`` (can be booted from the DSi menu)
* DSi Shop - Shows ``An Error Has Occured.`` (can be booted from the DSi menu)
* DSi Sound - Randomly doesn't boot, has a chance of deleting it's save if it does
* Rayman (DSiWare) - Crashes on save points, show corrupted graphics on title screen
* Mario VS DK minis march again - top screen while loading is messed up
* Brain Age Express: Sudoku - Shows garbage on bottom screen during ESRB splash
* GodMode9i - NAND drive fails to mount (relaunch godmode9i from within itself and the problem is fixed) 


## State of the project (Last updated 2026-08-14)
Currently, while astronaut is adequate to recreate the basic functionality of unlaunch, i (vikrinox) do not personally deem it adequate for a full 1.0 release as of right now.

## Compiling yourself
Due to the complexity of building DS binaries from rust, the main crate of this repository is actually a builder program, as opposed to the actual code (which you will instead find in the `astronaut` folder). 

When compiling Astronaut yourself you will need the following installed:

* the rust programming language (rustc, cargo, etc.) and the ``rust-src`` component
* a suitable C/C++ compiler for the fatfs dependency. (this will be searched for by the build program)
* the `lld` linker 
* the `arm-none-eabi-gcc` compiler.  

Once rust and the other dependencies are installed, compiling *should* be as simple as running `cargo run`. Optionally, you can provide a number of command line arguments to change how astronaut is compiled. For information, use ``cargo run -- --help``. (NOTE: the first set of ``--`` means we're finished providing command line arguments for cargo, and everything thereafter goes to the compiled build program.)

## Compatability with Unlaunch (and the a+b combo)
In order to make sure there is not a sea of chaos within the DSi Modding community, the `a+b` button combo is fixed to start astronauts gui.

Astronaut also supports unlaunch's "autoload" API. Meaning that options for loading DS homebrew with unlaunch will (in most cases) be redirected to astronaut accordingly.

## Compatability with slot-1
There is no support for launching the cartridge inserted into slot-1 from the astronaut gui. Instead, it is recommended to autoboot the DSi Menu or a homebrew slot-1 launcher to get this functionality. 

## Configuration
Astronaut currently looks for the settings in two locations; ``sdmc:/_nds/astronaut/settings.ini``, and ``nand:/astronaut.ini``. If none of these are found, a default is selected. The GUI currently saves to both locations when available, prefering to use the settings on the SD card when possible.

## Themes
It is possible to style the astronaut GUI with the help of themes. These are centered around arbitrarily placed `.ini` file. Once selected from the gui and saved, it will act as the theme upon next reboot.


