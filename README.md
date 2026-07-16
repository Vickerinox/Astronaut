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


## State of the project (Last updated 2026-07-15)
Currently, while astronaut is adequate to recreate the basic functionality of unlaunch i (vikrinox) do not personally deem it adequate for a full 1.0 release as of right now. Instead, i've choosen to make this first proof of concept public. 


## Compiling on your own
Due to the complexity of compiling DS binaries with rust, the crate at the very root is designed for compiling the whole project.
In order for the project to build correctly, one must also have the

## Compatability with Unlaunch
In order to make sure there is not a sea of chaos within the DSi Modding community, the `a+b` button combo is fixed to start astronauts gui.

## Compatability with slot-1
There is no support for launching the cartridge inserted into slot-1 from the astronaut gui. Instead, it is recommended to autoboot the DSi Menu or a homebrew slot-1 launcher to get this functionality. 

## Configuration
Astronaut currently looks for the settings in two locations; ``sdmc:/_nds/astronaut/settings.ini``, and ``nand:/astronaut.ini``. If none of these are found, a default is selected. The GUI currently only saves to the location on the SD card, as nand writes have not been tested.


## Themes
It is possible to style the astronaut GUI with the help of themes. These are centered around arbitrarily placed `.ini` file.
