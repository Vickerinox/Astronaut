# Colors

When styling astronaut with the help of themes, there are a few different supported color formats. Colors are always represented as a number in hexadecimal format, and is parsed differently depending on length. In the end however, it always becomes a r5g5b5a1 type color.

### 4 character hex
This will be parsed as a r5g5b5a1 color.

### 6 character hex
This will be parsed as a r8g8b8 color with the alpha bit set.

### 7/8 character hex
this will be parsed as a r8g8b8a4/8 color with alpha being set on nonzero alpha values.

## Meaning of the alpha bit

As most rendering in astronaut is done with the 3D hardware of the DS, there is limited functionality for the alpha bit. However, for "solid" colors, it shall be set. When it is cleared however, they still have the following function:

* When the alpha bit is cleared on a text color, it enables the secondary color palette of the current font. On the default font, this inverts the text and outline color. Meaning the text becomes black and the outline shows the text color. If the font does not have a secondary palette, it will instead appear as black.

* When the alpha bit is cleared on a background color, it enables color tinting of the current background image. This uses color multiplication, and lets you create "semi transparent" elements of the GUI. If no background image is present, black is used instead and you are advised to set the alpha bit to see your color. (In practice, this means that if you want to make an element transprent and just show the background, use color `7FFF`)
