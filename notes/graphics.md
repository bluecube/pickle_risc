# Graphics

While the CPU's main focus is to be a multi-user system mainly focused on networking (like Magic-1!),
I'd also like to have at least some fancy graphics.
But I definitely don't want to implement a VGA graphics from discrete logic.

As a fun middle ground, the plan is to have a network based graphics protocol
and implement the display server on a "real" computer.

## Goals
- High-ish resolution
- GUI system similar to Windows 3.0 can be built on top of it
- Fast enough for GUI and simple games
    - Even when runnign over low bandwidth SLIP link

# Design
- 640x480 or 800x600 resolution (fixed?)
- 64kB (or 64kW ?) graphics memory
- 16 sprites
    - Rendered with fixed Z order
        - Start pointer in graphics memory
        - x, y, w, h
        - 16 color palette
            - From RGB565?
            - From RGBA4444?
        - scaling?
        - 90° rotations?
- Commands:
    - Upload to graphics memory
    - Copy in graphics memory
    - Set sprite parameters
- Events:
    - Keyboard
    - Mouse
    - Joystick?
- Maybe we could also support sound?
