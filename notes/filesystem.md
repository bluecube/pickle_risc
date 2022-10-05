# Filesystem notes
These are just quick notes about how I'd like to implement the filesystem,
based only on assumptions on how the storage will work.
No real work has been done on this.

This assumes that we will have something like an SD card connected through SPI.

## Goals
- Simple (will be implemented in assembler)
    - Specifically: Not COW :-)
- Fun!

## Design
- Block size 1kWord = 2kB
    - Matches the MMU page size
- 16b block pointers
- => 128MB max addressable space

### Superblock
- First block of the device
- "PICKLEFS" magic
- Parameters?
- 16b Pointer to first unused block
- 16b Pointer to root directory

### Boot area
- Optional?
- Contiguous range of blocks starting at the second block of the device
- Must have a corresponding directory entry and file header

### Unused blocks
- 16b pointer to next unused block

### Root directory
- `..` entry points to itself
- Otherwise like normal directory

### Directory
- Creation date
    - ?
- Flags
    - ?
- 16b refcount
- List of entries
    - Name
        - Length?
        - Format?
    - 16b pointer to first block of entry (either file header or directory)
- 16b pointer to next block of directory
- Special entry
    - `..` pointing to parent
- Additional directory blocks only contain the entry list and next pointer

### File header
- Creation date
    - ?
- Flags
    - ?
- 16b refcount
- Size in B
    - 32b?
- List of 16b data block pointers
    - Number of valid blocks can be calculated from file size
- 16b pointer to next block of file header
- Additional file header blocks only contain the block pointer list and next pointer
