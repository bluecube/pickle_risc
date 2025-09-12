# Graphics

While the CPU's main focus is to be a multi-user system mainly focused on networking (like Magic-1!),
I'd also like to have at least some fancy graphics.
But I definitely don't want to implement a VGA graphics from discrete logic.

As a fun middle ground, the plan is to have a network based graphics protocol
and implement the display server on a "real" computer.

## Goals
- Handle graphics, sound and input.
- Games and demos!
- GUI system similar to Windows 3.0 can be built on top of it
- Works well even over a slow nework connection
- A bit of an anachronistic overkill is ok (Better than the best 90s consoles!)
- Since we there won't really be any HW limitations here, the interface should try to be simpler than the inspirations.

## Inspiration
The protocol itself is mostly inspired by the X window system, but significantly simplified.

For the graphical capabilities I'd like to keep close to 4th gen consoles, or perhaps Atari ST.

### Games
- Blockout
- Duke Nukem 2
- Jazz Jackrabit
- TFX (?)
- Starglider 2
- Another World

# Design
- 640x480 resolution (fixed? resizable server window?)
- Graphics memory
    - 64kW at 4bpp is not enough even for single 640x480 screen!
    - => 16bit pointers are not enough
    - 2MB (1MW) might be a reasonable amount of overkill (17 full screens @ 800x600)
- Server-side allocations
    - Allocated block is assigned a 16bit identifier by the server. 
- Sprites
    - Limit sprite size to something like 16x16 or 32x32?
      - More period-accurate, but feels artificial
    - Scaling?
    - Palette selected per sprite
- Framebuffer
    - If sprites are not limited by size, we can use one as a FB
- Commands:
    - Upload to graphics memory
        - How to support transparent uploaded data?
            - (useful for client side text rendering)
            - Color 0 in every palette?
    - Copy in graphics memory
    - Draw triangles in memory
    - Set sprite parameters
- Events:
    - Keyboard
    - Mouse
    - Joystick?
- Maybe we could also support sound?
