MEMORY
{
  FLASH : ORIGIN = 0x08000000, LENGTH = 16K
  RAM : ORIGIN = 0x20000000, LENGTH = 128K - 8
  /* NOINIT_RAM: Special RAM section that survives soft resets
   * This 8-byte region at the end of RAM is used to store the "magic value"
   * that tells the bootloader which application to boot after a reset.
   * On STM32F4, SRAM is NOT cleared by software resets (SYSRESETREQ),
   * only by power-on reset or brownout reset. */
  NOINIT_RAM : ORIGIN = 0x2001FFF8, LENGTH = 8
}

SECTIONS
{
  /* .noinit section: Uninitialized data that persists across soft resets
   * 
   * The (NOLOAD) attribute tells the linker not to initialize this section,
   * so its contents remain intact during software-triggered resets.
   * 
   * This is critical for the bootloader mechanism:
   * 1. An app writes a magic value (0xDEADBEEF or 0xCAFEBABE) to this section
   * 2. The app triggers a software reset via SCB->AIRCR SYSRESETREQ bit
   * 3. The bootloader runs after reset and reads the magic value
   * 4. Based on the magic value, bootloader jumps to the correct app
   * 5. Bootloader clears the magic to prevent boot loops
   * 
   * Without .noinit, the runtime would zero this memory during startup,
   * losing the magic value before the bootloader could read it. */
  .noinit (NOLOAD) : ALIGN(4)
  {
    *(.noinit .noinit.*);
    . = ALIGN(4);
  } > NOINIT_RAM
}

_stack_start = ORIGIN(RAM) + LENGTH(RAM);
