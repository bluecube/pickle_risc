# CPU Mechanical & Electrical Construction

- Eurocard (3U, 100x100mm or 100x160mm)
- 96 pin connector
- Backplane split into two zones
  - Core zone:
    - CPU internal cards (register decoder, register slices, ALU, instruction decoder, ...).
    - Each backplane slot has potentially different pinout
    - 3D printed keys enforcing correct card placement?
  - Memory / peripherial zone
    - All pinouts identical
- "Tiled" backplane?
  - Individual pieces of backplane connected together using edge connectors
  - To simplify development, make cheaper fails
- Register file
  - Probably 5 boards
    - Decoder
    - 4 boards of 8bit slices of 8 registers each
