/* App1 Memory Layout - starts at 16KB offset (after bootloader) */
MEMORY
{
FLASH : ORIGIN = 0x08004000, LENGTH = 128K
RAM : ORIGIN = 0x20000000, LENGTH = 128K
}
