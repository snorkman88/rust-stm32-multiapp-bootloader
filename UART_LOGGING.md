# UART Logging Setup

## Hardware Connections

The apps use UART2 for logging with the following pins on the STM32F411CEU6:
- **PA2**: UART2_TX (connect to RX of USB-to-serial adapter)
- **PA3**: UART2_RX (connect to TX of USB-to-serial adapter)  
- **GND**: Ground (connect to GND of adapter)

## UART Settings

- **Baud rate**: 115200
- **Data bits**: 8
- **Parity**: None
- **Stop bits**: 1

## Viewing Logs

### On macOS/Linux:
```bash
# Find your USB-to-serial device
ls /dev/tty.*

# Connect with screen (replace with your device)
screen /dev/tty.usbserial-XXXXXXXX 115200

# Or use minicom
minicom -D /dev/tty.usbserial-XXXXXXXX -b 115200
```

### On Windows:
- Use PuTTY, TeraTerm, or Arduino Serial Monitor
- Set to 115200 baud, 8N1

## What You'll See

When the device boots:
```
=== APP1 STARTING ===
APP1: Init complete - button interrupt enabled
APP1: Press button to switch to APP2
```

When you press the button:
```
APP1: Button pressed! Switching to APP2...

=== APP2 STARTING ===
APP2: Init complete - fast blinker mode
APP2: Press button to switch to APP1
```

When switching back:
```
APP2: Button pressed! Switching to APP1...

=== APP1 STARTING ===
APP1: Init complete - button interrupt enabled
APP1: Press button to switch to APP2
```

## Benefits of UART Logging

- ✅ Works across app switches and resets
- ✅ No debugger needed
- ✅ Simple, reliable, production-ready
- ✅ Works with multi-app bootloader at non-standard addresses
- ✅ Both apps can log simultaneously
- ✅ Bootloader could also log if needed

## Alternative: No USB-to-Serial Adapter?

If you don't have a USB-to-serial adapter, you can:
1. Use the ST-Link's Virtual COM Port (if your board supports it)
2. Get a cheap CH340/CP2102 USB-to-serial adapter ($2-5)
3. Use another microcontroller as a bridge
