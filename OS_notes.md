# OS
Pickle risc will run a multitasking OS.

## Design

Microkernels are cool.

### Functionatity in kernel
- Scheduler
- IPC
    - Message passing
        - Synchronous
            - Blocking and non-blocking mode
            - Small fixed payload
                - Registers only?
    - Shared memory
        - `sharemem`
            - "Share this page of my memory to that process"
            - returns some sort of ID uniquely identifying the share
        - `mmap` can take the ID from previous step to map the memory

- Memory / paging

### Functionality in servers
- UART driver
- Disk driver
- Filesystem
- Timer
- Networking
- Synchronization & locking

### Semaphores & locking
- Process can only block on message sending / receiving
- Mutex impl -- a little like futex in Linux
    - Atomic variable
        - 0: unlocked
        - 1: locked, noone is waiting
        - >=2 : locked, waiting processes
    - Sends message to synchronization server
        - Block on this
            - How is the lock object identified?
                - the sync server should not need (privileged) access to memory mappings
                - Lock objects carries ID inside?
