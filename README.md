# 🦀 STM32 Multi-Application Bootloader System

> **100% Written in Rust!** 🎉  
> No C, no assembly (well, just a tiny bit in the bootloader) - pure, Rust from bootloader to applications!

A practical example of running multiple applications on a single STM32F411CEU6 (Blackpill) microcontroller with seamless switching between them using a custom bootloader.

**Why Rust?** 🚀
- **Memory safety** without garbage collection
- **Zero-cost abstractions** - as fast as C
- **Fearless concurrency** with RTIC framework
- **Excellent embedded ecosystem** with `embedded-hal` and `cortex-m`
- **Modern tooling** - cargo makes building embedded systems a breeze!

## What Problem Does This Solve? 🤔

Imagine you want to run different programs on your microcontroller without having to reflash it every time. Maybe you want a "settings mode" and a "normal operation mode," or different diagnostic tools that you can switch between with a button press.

This project demonstrates exactly that: **two independent applications living in the same chip's flash memory**, with the ability to switch between them at runtime.

### The Two Applications 💡

**App1** (The Slow Blinker):
- Blinks the LED in a distinctive pattern: *blink-blink-looong pause*
- When you press the button, it triggers an interrupt and switches to App2
- Located at flash address `0x08004000`

**App2** (The Fast Blinker):
- Blinks the LED rapidly with a steady rhythm
- Continuously checks the button state (polling approach)
- When you press the button, it switches back to App1
- Located at flash address `0x08024000`

Both apps can switch to each other, creating a complete bidirectional switching system!

## The Bootloader: The Traffic Controller 🚦

Think of the bootloader as a **tiny program that decides which application to run** when the chip starts up. It sits at the very beginning of flash memory (`0x08000000`) where the chip always starts executing after a reset.

### How the Bootloader Works

1. **On Power-Up or Reset**: The STM32 chip always starts executing code from address `0x08000000` (the bootloader)

2. **Check the Magic Value**: The bootloader reads a special location in RAM (`0x2001FFF8`) looking for a "magic number"
   - If it finds `0xDEADBEEF` → Jump to App1
   - If it finds `0xCAFEBABE` → Jump to App2
   - If it finds anything else (or nothing) → Default to App1

3. **Clear the Magic**: After reading it, the bootloader clears the magic value to prevent boot loops

4. **Jump to the Application**: The bootloader sets up the processor to start running the chosen application
   - Updates the Vector Table Offset Register (VTOR) to point to the app's interrupt vectors
   - Jumps to the application's entry point using `cortex_m::asm::bootload()`

### The memory layout for this example

```
Flash Memory (512KB total):
┌─────────────────────────────────┐ 0x08000000
│      Bootloader (16KB)          │ <- Chip always starts here
├─────────────────────────────────┤ 0x08004000
│        App1 (128KB)             │ <- Default application
├─────────────────────────────────┤ 0x08024000
│        App2 (368KB)             │ <- Alternate application
└─────────────────────────────────┘ 0x0807FFFF
```

## Understanding the `memory.x` Files 📝

Each component (bootloader, App1, App2) needs its own `memory.x` file to tell the linker where in memory to place the code.

### Bootloader's `memory.x`

```ld
MEMORY
{
  FLASH : ORIGIN = 0x08000000, LENGTH = 16K
  RAM : ORIGIN = 0x20000000, LENGTH = 128K - 8
  NOINIT_RAM : ORIGIN = 0x2001FFF8, LENGTH = 8
}
```

**Key Points:**
- `FLASH`: Starts at the very beginning (`0x08000000`) - only 16KB to keep it small
- `RAM`: Normal RAM for variables and stack (slightly reduced to make room for NOINIT)
- `NOINIT_RAM`: **The magic ingredient!** This is a special 8-byte section at the end of RAM

#### What is `.noinit` and Why Do We Need It? 🔮

Normally, when a program starts, the runtime automatically clears (zeros out) all RAM. This is good for normal programs, but **we need to preserve the magic value across resets!**

The `.noinit` section tells the linker: **"Don't initialize this portion of memory!"** 

Here's the magic trick:
- On STM32F4, when you trigger a software reset (not a power cycle), the RAM contents are **NOT cleared** by the hardware
- BUT the C/Rust runtime would normally clear it during startup
- By marking it as `.noinit`, we prevent the runtime from touching it
- This allows the magic value written by App1 or App2 to **survive the reset** and be read by the bootloader

### App1's `memory.x`

```ld
MEMORY
{
  FLASH : ORIGIN = 0x08004000, LENGTH = 128K
  RAM : ORIGIN = 0x20000000, LENGTH = 128K
}
```

**Key Points:**
- `FLASH`: Starts at `0x08004000` (right after the 16KB bootloader)
- Gets 128KB of space for its code
- Uses full RAM since it doesn't need the noinit section

### App2's `memory.x`

```ld
MEMORY
{
  FLASH : ORIGIN = 0x08024000, LENGTH = 368K
  RAM : ORIGIN = 0x20000000, LENGTH = 128K
}
```

**Key Points:**
- `FLASH`: Starts at `0x08024000` (after bootloader + App1)
- Gets the remaining 368KB of flash memory
- Also uses full RAM

## How App Switching Works: Step by Step 🔄

Let's walk through what happens when you press the button in App1:

### Step 1: Button Press in App1
```
You press the button → App1's interrupt handler fires
```

### Step 2: Write the Magic Value
```rust
const MAGIC_ADDR: *mut u32 = 0x2001_FFF8 as *mut u32;
const MAGIC_APP2: u32 = 0xCAFE_BABE;
write_volatile(MAGIC_ADDR, MAGIC_APP2);
```
App1 writes `0xCAFEBABE` (magic value for App2) to the special RAM location

### Step 3: Trigger a System Reset
```rust
const SCB_AIRCR: *mut u32 = 0xE000_ED0C as *mut u32;
write_volatile(SCB_AIRCR, 0x05FA0004);  // System reset command
```
App1 triggers a software reset of the entire chip

### Step 4: Chip Resets
```
The processor resets → All peripherals reset → But RAM keeps its contents!
```
**Critical:** Software reset does NOT clear RAM on STM32F4

### Step 5: Bootloader Starts
```
Chip starts executing from 0x08000000 (bootloader entry point)
```
As always after any reset, execution begins at the bootloader

### Step 6: Bootloader Reads Magic
```rust
let magic = unsafe { read_volatile(&MAGIC_VALUE) };  // Reads 0xCAFEBABE
```
The bootloader finds `0xCAFEBABE` in the noinit section

### Step 7: Bootloader Clears Magic
```rust
unsafe { write_volatile(&mut MAGIC_VALUE, 0); }
```
Clears it so next power-on boots App1 by default

### Step 8: Bootloader Jumps to App2
```rust
let app_addr = match magic {
    MAGIC_APP2 => APP2_ADDR,  // 0x08024000
    _ => APP1_ADDR,
};
jump_to_app(app_addr);
```
The bootloader sets VTOR and jumps to App2 at `0x08024000`

### Step 9: App2 Runs
```
App2 starts fresh → Full reset means clean initialization
LED now blinks fast → Button polling works perfectly
```

**The same process works in reverse when App2 switches back to App1!**

## What if adding more applications is needed?

Want to add App3, App4, or more? Here's exactly what you need to do:

### Step 1: Decide on Memory Layout

First, determine where your new app will live in flash. You need to adjust the existing apps to make room.

**Example: Adding App3 (64KB)**

Current layout:
- Bootloader: `0x08000000` - `0x08003FFF` (16KB)
- App1: `0x08004000` - `0x08023FFF` (128KB)
- App2: `0x08024000` - `0x0807FFFF` (368KB)

New layout:
- Bootloader: `0x08000000` - `0x08003FFF` (16KB) - *unchanged*
- App1: `0x08004000` - `0x08023FFF` (128KB) - *unchanged*
- App2: `0x08024000` - `0x08043FFF` (128KB) - *reduced from 368KB*
- **App3: `0x08044000` - `0x0807FFFF` (240KB)** - *new!*

### Step 2: Create App3 Directory Structure

```bash
mkdir -p app3/src
mkdir -p app3/.cargo
```

### Step 3: Create `app3/Cargo.toml`

Copy from app1 or app2 and change the package name:
```toml
[package]
name = "app3"
version = "0.1.0"
edition = "2021"

[dependencies]
# ... same dependencies as app1/app2 ...
```

**Rationale:** Each app is a separate Rust binary with its own dependencies.

### Step 4: Create `app3/memory.x`

```ld
MEMORY
{
  FLASH : ORIGIN = 0x08044000, LENGTH = 240K
  RAM : ORIGIN = 0x20000000, LENGTH = 128K
}
```

**Rationale:** 
- `ORIGIN`: Must match your chosen flash address (`0x08044000` in this example)
- `LENGTH`: The space allocated for this app (240KB remaining flash)
- All apps share the same RAM space (only one app runs at a time)

### Step 5: Create `app3/.cargo/config.toml`

```toml
[target.thumbv7em-none-eabihf]
rustflags = [
  "-C", "link-arg=-Tlink.x",
  "-C", "link-arg=-Tdefmt.x",
]
```

**Rationale:** Ensures app3 uses the defmt linker script (not needed by bootloader, but needed by apps).

### Step 6: Create `app3/src/main.rs`

Copy from app1 or app2, then customize:

```rust
pub unsafe fn jump_to_other(_addr: u32) -> ! {
    use core::ptr::write_volatile;
    
    // Magic RAM location and value for your target app
    const MAGIC_ADDR: *mut u32 = 0x2001_FFF8 as *mut u32;
    const MAGIC_APP1: u32 = 0xDEAD_BEEF;  // Or whatever app you want to jump to
    
    write_volatile(MAGIC_ADDR, MAGIC_APP1);
    cortex_m::asm::dsb();
    
    // Trigger system reset
    const SCB_AIRCR: *mut u32 = 0xE000_ED0C as *mut u32;
    write_volatile(SCB_AIRCR, 0x05FA0004);
    
    loop { cortex_m::asm::nop(); }
}

// ... rest of your app code ...
```

**Rationale:** Each app needs the jump function to write the appropriate magic value for switching.

### Step 7: Update Workspace `Cargo.toml`

Add app3 to the workspace members:

```toml
[workspace]
members = ["bootloader", "app1", "app2", "app3"]
```

**Rationale:** Makes `cargo build --workspace` include app3.

### Step 8: Update App2's `memory.x`

Since we reduced App2's size to make room for App3:

```ld
MEMORY
{
  FLASH : ORIGIN = 0x08024000, LENGTH = 128K  # Changed from 368K
  RAM : ORIGIN = 0x20000000, LENGTH = 128K
}
```

**Rationale:** Apps can't overlap in flash - you must resize existing apps if needed.

### Step 9: Add Magic Value for App3

Define a unique magic value for App3. In the bootloader's `src/main.rs`:

```rust
const MAGIC_APP1: u32 = 0xDEAD_BEEF;
const MAGIC_APP2: u32 = 0xCAFE_BABE;
const MAGIC_APP3: u32 = 0xBAAD_F00D;  // New magic value

const APP3_ADDR: u32 = 0x0804_4000;  // New app address
```

**Rationale:** Each app needs a unique magic value so the bootloader knows which one to boot.

### Step 10: Update Bootloader Logic

In `bootloader/src/main.rs`, update the match statement:

```rust
let app_addr = match magic {
    MAGIC_APP2 => APP2_ADDR,
    MAGIC_APP3 => APP3_ADDR,  // New case
    MAGIC_APP1 => APP1_ADDR,
    _ => APP1_ADDR,  // Default
};
```

**Rationale:** The bootloader must recognize the new magic value and know where to jump.

### Step 11: Update Other Apps to Jump to App3

If App1 or App2 need to jump to App3, update their `jump_to_other` functions:

```rust
const MAGIC_ADDR: *mut u32 = 0x2001_FFF8 as *mut u32;
const MAGIC_APP3: u32 = 0xBAAD_F00D;

write_volatile(MAGIC_ADDR, MAGIC_APP3);
// ... trigger reset ...
```

**Rationale:** Apps need to know the magic values of other apps they want to switch to.

### Step 12: Add Build Tasks (Optional)

Update `.vscode/tasks.json` to include build and flash tasks for app3:

```json
{
  "label": "Build app3 (release)",
  "type": "shell",
  "command": "cargo build --release -p app3"
},
{
  "label": "Flash app3",
  "type": "shell",
  "command": "probe-rs download target/thumbv7em-none-eabihf/release/app3 --chip STM32F411CEUx --base-address 0x08044000"
}
```

**Rationale:** Makes building and flashing easier from VS Code.

### Step 13: Build Everything

```bash
cargo build --release -p bootloader
cargo build --release -p app1
cargo build --release -p app2
cargo build --release -p app3
```

**Rationale:** Each component must be built separately before flashing.

### Step 14: Flash in Order

```bash
# Flash bootloader first (always at 0x08000000)
probe-rs download target/thumbv7em-none-eabihf/release/bootloader \
  --chip STM32F411CEUx --base-address 0x08000000

# Flash app1
probe-rs download target/thumbv7em-none-eabihf/release/app1 \
  --chip STM32F411CEUx --base-address 0x08004000

# Flash app2
probe-rs download target/thumbv7em-none-eabihf/release/app2 \
  --chip STM32F411CEUx --base-address 0x08024000

# Flash app3
probe-rs download target/thumbv7em-none-eabihf/release/app3 \
  --chip STM32F411CEUx --base-address 0x08044000
```

**Rationale:** The value for `--base-address` must match each app's `ORIGIN` in its `memory.x`.

### Quick Checklist for Adding Apps ✅

- [ ] Choose a flash address and size that doesn't overlap existing apps
- [ ] Create app directory with `src/`, `.cargo/`, `Cargo.toml`, `memory.x`
- [ ] Set correct `ORIGIN` in `memory.x` to match your chosen flash address
- [ ] Add app to workspace `Cargo.toml` members
- [ ] Choose a unique magic value (e.g., `0xBAAD_F00D`)
- [ ] Update bootloader to recognize the new magic value and app address
- [ ] Update other apps' jump functions if they need to switch to the new app
- [ ] Build with `cargo build --release -p appN`
- [ ] Flash with correct `--base-address` matching your flash origin
- [ ] Rebuild bootloader if you changed its code
- [ ] Test by power cycling and pressing buttons

## Building and Flashing ⚡

```bash
# Build all components
cargo build --release -p bootloader
cargo build --release -p app1
cargo build --release -p app2

# Flash in order (bootloader first!)
probe-rs download target/thumbv7em-none-eabihf/release/bootloader \
  --chip STM32F411CEUx --base-address 0x08000000

probe-rs download target/thumbv7em-none-eabihf/release/app1 \
  --chip STM32F411CEUx --base-address 0x08004000

probe-rs download target/thumbv7em-none-eabihf/release/app2 \
  --chip STM32F411CEUx --base-address 0x08024000
```

## Hardware 🔧

- **Board**: STM32F411CEU6 Blackpill
- **LED**: PC13 (onboard LED)
- **Button**: PA0 (with pull-up resistor)
- **Clock**: 25 MHz HSE (external crystal)

## Key Takeaways 💎

1. **The bootloader is your app selector** - it always runs first and decides what to run next
2. **The `.noinit` section is the secret sauce** - it preserves data across software resets (you could see it as a very low level mailbox for message passing between the different apps and the bootloader in which the content of the message tells the bootloader which app should run).
3. **Each app lives at its own flash address** - they can't overlap or overwrite each other
4. **System reset gives a clean slate** - each app starts fresh, solving interrupt initialization issues
5. **Magic values are the communication protocol** - simple but effective way to pass information through a reset

This architecture is commonly used in production embedded systems for features like firmware updates, multiple operating modes, and diagnostic tools!

---

## License 📄

MIT License - Feel free to use this as a learning resource or starting point for your own projects.

**Happy hacking!** 🦀✨
