# .HPS Audio File Documentation

## File Layout

The overall structure of a stereo .hps file is the following:

| Offset | Section                             |
| ------ | ----------------------------------- |
| 0x00   | [File Header](#file-header)         |
| 0x10   | [Left Channel Info](#channel-info)  |
| 0x48   | [Right Channel Info](#channel-info) |
| 0x80   | [DSP Blocks](#dsp-block-layout)     |

## DSP Block Layout

The meat of an .hps file is the "DSP block" data. The song contained in the
file is split into multiple "blocks", each containing encdoded audio data as
well as a link to the start of the next block.

The first half of the [frames](#dsp-audio-frame) in each block are for the
left [audio channel](https://docs.rs/rodio/latest/rodio/source/trait.Source.html#channels), and other half are for the right.

| Offset | Section                                       |
| ------ | --------------------------------------------- |
| 0x00   | [DSP Block Header](#dsp-block-header)         |
| 0x0C   | [Left DSP Decoder State](#dsp-decoder-state)  |
| 0x14   | [Right DSP Decoder State](#dsp-decoder-state) |
| 0x1C   | Padding (Always 0)                            |
| 0x20   | [DSP Audio Frames](#dsp-audio-frame)          |

## Sections

### Endianness

All numeric types are in big-endian format

### File Header

The file header is the first section within an .hps file. It contains the
[magic string](<https://en.wikipedia.org/wiki/Magic_number_(programming)>), the
[sample rate](https://docs.rs/rodio/latest/rodio/source/trait.Source.html#sampling) of
the song, and the number of
[audio channels](https://docs.rs/rodio/latest/rodio/source/trait.Source.html#channels) used.

_Length: 0x10_

| Offset | Name          | Type    | Length | Description                                                                                          |
| ------ | ------------- | ------- | ------ | ---------------------------------------------------------------------------------------------------- |
| 0x00   | Magic String  | [u8; 8] | 0x08   | " HALPST\0" [magic string](<https://en.wikipedia.org/wiki/Magic_number_(programming)>)               |
| 0x08   | Sample Rate   | u32     | 0x04   | Number of [samples](https://docs.rs/rodio/latest/rodio/source/trait.Source.html#sampling) per second |
| 0x0C   | Channel Count | u32     | 0x04   | Number of [audio channels](https://docs.rs/rodio/latest/rodio/source/trait.Source.html#channels)     |

### Channel Info

The .hps file should have a channel info section for each audio channel.
Notably, an audio channel contains 16 "coefficients" that are used in the
calculation to decode samples within the channel blocks' [frames](#dsp-audio-frame)

_Length: 0x38_

| Offset | Name                      | Type                                    | Length | Description                                                                                                         |
| ------ | ------------------------- | --------------------------------------- | ------ | ------------------------------------------------------------------------------------------------------------------- |
| 0x00   | Largest Block Length      | u32                                     | 0x04   | Length of the largest block in the channel                                                                          |
| 0x04   | (Unknown)                 | u32                                     | 0x04   | Always 0x2                                                                                                          |
| 0x08   | Sample Count              | u32                                     | 0x04   | [!!UNSURE] Number of [samples](https://docs.rs/rodio/latest/rodio/source/trait.Source.html#sampling) in the channel |
| 0x0C   | (Unknown)                 | u32                                     | 0x04   | Always 0x2                                                                                                          |
| 0x10   | DSP Decode Coefficients   | [i16; 16]                               | 0x20   | Each audio frame requires a 'coefficient' to calculate values of the 14 samples within the frame                    |
| 0x30   | Initial DSP Decoder State | [DSP Decoder State](#dsp-decoder-state) | 0x08   | The first [DSP decoder state](#dsp-decoder-state) for the channel                                                   |

### DSP Block Header

Each block of audio has it's own header. It contains:

- The length of the data in the block (excluding the header itself)
- A pointer to the next block

_Length: 0x20_

| Offset | Name                  | Type | Length | Description                                                                |
| ------ | --------------------- | ---- | ------ | -------------------------------------------------------------------------- |
| 0x00   | DSP Data Length       | u32  | 0x04   | Length of non-header data contained within the block: `blockLength - 0x20` |
| 0x04   | (Unknown)             | u32  | 0x04   | Often 0xFFFF, but not always                                               |
| 0x08   | Pointer to Next Block | u32  | 0x04   | Address of the next block to read (offset from the start of the file)      |

### DSP Decoder State

_Length: 0x08_

| Offset | Name           | Type | Length | Description                       |
| ------ | -------------- | ---- | ------ | --------------------------------- |
| 0x00   | P/S high byte  | u8   | 0x01   | [!!UNSURE] (predictor and scale?) |
| 0x01   | P/S            | u8   | 0x01   | [!!UNSURE] (predictor and scale?) |
| 0x02   | Initial hist 1 | i16  | 0x02   | Initial hist1 value for the block |
| 0x04   | Initial hist 2 | i16  | 0x02   | Initial hist2 value for the block |
| 0x06   | (Unknown)      | u16  | 0x02   | Always 0                          |

### DSP Audio Frame

Each frame of audio data contains a one byte header followed by seven bytes of
encoded [samples](https://docs.rs/rodio/latest/rodio/source/trait.Source.html#sampling).

The header byte contains a `scale (u16)` which can be calculated like so:
`1 << (header & 0xF)` as well as a `coefficient_index (usize)`, which can be
calculated like so: `header >> 4`. The `coefficient_index` can be used to index
into the array of "DSP decode coefficient"s contained in the [channel info](#channel-info)
to obtain the `coefficient` we need to decode the [samples](https://docs.rs/rodio/latest/rodio/source/trait.Source.html#sampling)
in this frame.

Each of the seven bytes following the frame header contains two encoded samples,
one in the first nibble of the byte, and the other in the second. To decode a
nibble into a sample, we can use the following formula:

`clamp_i16(((nibble * scale) << 11) + 1024 + ((coef1 * hist1) + (coef2 * hist2)) >> 11);`

where `hist1` and `hist2` represent the two previously decoded samples.

Note: Whether a frame belongs to the left or the right [audio channel](https://docs.rs/rodio/latest/rodio/source/trait.Source.html#channels)
depends on where it appears in the block. The first half of the frames in a
block are for the left audio channel, and the other half are for the right.

_Length: 0x08_

| Offset | Name             | Type    | Length | Description                                                    |
| ------ | ---------------- | ------- | ------ | -------------------------------------------------------------- |
| 0x00   | DSP Frame header | u8      | 0x01   | This byte contains an encdoded 'scale' and 'coefficient_index' |
| 0x01   | Encoded Samples  | [u8; 7] | 0x07   | Each of these 7 bytes contains 2 encoded samples               |

## Resources

This documentation was put together using knowledge learned from the following sources:

- https://docs.rs/rodio/0.17.1/rodio/source/trait.Source.html#a-quick-lesson-about-sounds
- https://github.com/pdeljanov/Symphonia/blob/398dab0/GETTING_STARTED.md#multimedia-basics
- https://github.com/jmlee337/dsp2hps/blob/6531757/dsp2hps/dsp2hps/main.cpp
- https://github.com/Thealexbarney/VGAudio/blob/9d8f6ea/src/VGAudio/Containers/Hps
- https://github.com/vgmstream/vgmstream/blob/8d0dd44/src/meta/halpst.c
- https://www.metroid2002.com/retromodding/wiki/DSP_(File_Format)#ADPCM_Data
