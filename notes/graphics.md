# Graphics

While the CPU's main focus is to be a multi-user system mainly focused on networking (like Magic-1!),
I'd also like to have at least some fancy graphics.
But I definitely don't want to implement a VGA graphics from discrete logic.

Right now there are two separate options:
1. RPi based GPU, connected over SPI to the CPU memory bus.
2. Network based graphics protocol, display server on a "real" computer.

Both of the options would provide basically the same functionality.

## Goals
- High-ish resolution
- GUI system similar to Windows 3.0 can be built on top of it
- Fast enough for GUI and simple games
    - Even when running over low bandwidth SLIP link

# Design
- 640x480 resolution
- 4bpp, 256 entry global palette, 16b palette entries (RGB565)
- Graphics memory
    - 512kB video RAM (TODO: 1MB?)
- Main and overlay framebuffers
    - Start at pointer in graphics memory (1 word is 4 pixels, how to handle that?)
    - Overlay framebuffer supports transparency as color 0, rendered on top
    - Palette offset into global palette
- 64 sprites
    - 16x16 tiles
    - 1 tile width, arbitrary number of tiles height (16x16, 16x32, 16x48...)
    - Rendered with fixed Z order (TODO: Order relative to overlay framebuffer?)
    - Start tile index
    - Palette offset into global palette
- Commands:
    - Upload to graphics memory
    - Blit
        with transparent color 0, or without
    - Draw primitives in graphics memory
        - Axis aligned rectangles
            - filled
            - outlined
        - Lines?
        - Circles?
        - Filled triangles ?
- Events:
    - Keyboard
    - Mouse
    - Joystick?
- Maybe we could also support sound?
