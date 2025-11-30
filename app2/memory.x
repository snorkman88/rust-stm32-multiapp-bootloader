/* App2 Memory Layout - starts at 144KB offset (16KB bootloader + 128KB app1) */
MEMORY
{
FLASH : ORIGIN = 0x08024000, LENGTH = 368K
RAM : ORIGIN = 0x20000000, LENGTH = 128K
}
