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
    - Even when running over low bandwidth SLIP link

# Design
- 640x480 or 800x600 resolution (fixed?)
- Graphics memory
    - 64kB at 4bpp is not enough even for single 640x480 screen!
    - 64kW at 4bpp is slightly more than single 800x600 screen
    - => 16bit pointers are not enough
    - 2MB (1MW) might be a reasonable amount of overkill (17 full screens @ 800x600)
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
        - How to support transparent uploaded data?
            - (useful  for client side text rendering)
    - Copy in graphics memory
    - Draw primitives in graphics memory
        - Axis aligned rectangles
            - filled
            - outlined
        - Lines?
        - Circles?
    - Set sprite parameters
- Events:
    - Keyboard
    - Mouse
    - Joystick?
- Maybe we could also support sound?
