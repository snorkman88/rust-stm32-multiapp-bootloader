/* Minimal device.x for bootloader - no interrupts needed */
PROVIDE(NMI = DefaultHandler);
PROVIDE(HardFault = HardFault_);
PROVIDE(MemManage = DefaultHandler);
PROVIDE(BusFault = DefaultHandler);
PROVIDE(UsageFault = DefaultHandler);
PROVIDE(SVCall = DefaultHandler);
PROVIDE(PendSV = DefaultHandler);
PROVIDE(SysTick = DefaultHandler);
