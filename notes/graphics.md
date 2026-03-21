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
- 640x480 or 800x600
- 4bpp or 8bpp, 16b palette entries (RGB565)
- Graphics memory
    - 512kB video RAM (TODO: 1MB?)
- Selectable framebuffer offset and row stride
    - Can be used to double buffer and/or scroll the display
- Commands:
    - Upload to graphics memory
    - Blit
        with or without transparent color
    - Draw primitives in graphics memory
        - Axis aligned rectangle
            - filled
            - outlined
        - Line
        - Filled triangle
- Events:
    - Keyboard
    - Mouse
    - Joystick?
- Maybe we could also support sound?
- Emulation details
  - Blitter runs on 2-4MPx/s
  - Emulates scanlines
