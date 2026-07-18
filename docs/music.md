# Music

Through themes and the main settings file it's possible to play a background song in the astronaut GUI. The file may either be of ``.mod`` (Amiga module) or `.wav` (Wave) format.

## Notice on MODs

The built in MOD player is currently not ProTracker compatible or support all typical commands.

## Specifics on WAVs

The built in WAV player should support 8 and 16 bit (both signed and unsigned) data, in either mono or stereo, up to 48KHz sampling rate. However, as streaming can be slow from the sd card, the gui may experience slowdown to keep the audio buffer filled. 44.1KHz 16-bit signed stereo audio should be able to be comfortably streamed.