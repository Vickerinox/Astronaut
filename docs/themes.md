# Themes

Astronaut can be styled using themes, which are based around a `.ini` file. Which may use the following segments and keys.  If any fields are left unassigned, default values are used instead. For information on how to specify colors, please see the colors documentation.

## assets

This section contains filepaths which can either be absolute, or relative to the directory the theme file is kept in.
* ``wallpaper``: The image displayed on the top screen
* ``background``: The image displayed on the bottom screen
* ``music``: The sound file to play for the theme
* ``font``: A custom font for the theme, which can be generated with the font converter tool in `tools/font_converter`

## colors

This section contains basic colors

* ``background``: The main background color
* ``text``: The main text color
* ``roms``: The text color used for roms in the file explorer
* ``folders``: The text color used for folders in the file explorer
* ``assets``: The text color used for other astronaut related assets in the file explorer


## widgets

This section contains the styling for the GUI's widgets. There is also the `widgets.pressed` subsection for the widget being pressed on, and `widgets.active` for the currently highlighted widget.

* `outline`: color of the outline on the widget
* `fill`: color of the fill in boxes on the widget