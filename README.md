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
* Sudoku (DSiWare) - Shows garbage on bottom screen during ESRB splash


## State of the project (Last updated 2026-07-15)
Currently, while astronaut is adequate to recreate the basic functionality of unlaunch i (vikrinox) do not personally deem it adequate for a full 1.0 release as of right now. Instead, i've choosen to make this first proof of concept public. 

## Compiling yourself
When compiling Astronaut yourself you will need the rust programming lanugage installed as well as a suitable C/C++ compiler for the fatfs dependency. You will also need access to the `lld` linker and the `arm-none-eabi-gcc` compiler. Due to the complexity of building DS binaries from rust, the main crate of this repository is actually a builder program, as opposed to the actual code. (which you will instead find in the `astronaut` folder)

Once rust and the other dependencies are installed, compiling *should* be as simple as running `cargo run`. Optionally, you can provide 2 paths as command line arguments. The first is a custom path for the `astronaut.bin` file which is the final binary. The second is a path for a NAND image (`nand.bin`) file for the DSi which you wish to install astronaut onto (WARNING; PLEASE ONLY DO THIS ON A NAND IMAGE WHERE UNLAUNCH OR ASTRONAUT HAS ALREADY BEEN INSTALLED WITH AN OFFICIAL INSTALLER. AS THIS METHOD HAS NOT BEEN PROPERLY TESTED TO PREVENT THE DSI FIRMWARE FROM TRIPPING ITS ANTI TAMPERING CHECKS AND DELETING ITSELF.)


## Compatability with Unlaunch (and the a+b combo)
In order to make sure there is not a sea of chaos within the DSi Modding community, the `a+b` button combo is fixed to start astronauts gui.

## Compatability with slot-1
There is no support for launching the cartridge inserted into slot-1 from the astronaut gui. Instead, it is recommended to autoboot the DSi Menu or a homebrew slot-1 launcher to get this functionality. 

## Configuration
Astronaut currently looks for the settings in two locations; ``sdmc:/_nds/astronaut/settings.ini``, and ``nand:/astronaut.ini``. If none of these are found, a default is selected. The GUI currently saves to both locations when available, prefering to use the settings on the SD card when possible.

## Themes
It is possible to style the astronaut GUI with the help of themes. These are centered around arbitrarily placed `.ini` file. Once selected from the gui and saved, it will act as the theme upon next reboot.


