MEMORY {
    /*     NOTE 1 K = 1 KiBi = 1024 bytes   */
    /*                                      */
    /*            FLASH LAYOUT              */
    /*                                      */
    /* |------------------- 0x10000000 ---| */
    /* | BOOT2      : 0x100               | */
    /* |------------------- 0x10000100 ---| */
    /* | BOOT FLASH : 24K - 0x100         | */
    /* |------------------- 0x10006000 ---| */
    /* | BOOT STATE : 4K                  | */
    /* |------------------- 0x10007000 ---| */
    /* | APP FLASH  : 832K                | */
    /* |------------------- 0x100D7000 ---| */
    /* | DFU FLASH  : 836K                | */
    /* |------------------- 0x101A8000 ---| */
    /* |            : 32K                 | */
    /* |------------------- 0x101B0000 ---| */
    /* | CYW FW     : 256K                | */
    /* |------------------- 0x101F0000 ---| */
    /* | CYW CLM    : 8K                  | */
    /* |------------------- 0x101F2000 ---| */
    /* |            : 56K                 | */
    /* |------------------- 0x10200000 ---| */

    BOOT2                             : ORIGIN = 0x10000000, LENGTH = 0x100
    BOOTLOADER_STATE                  : ORIGIN = 0x10006000, LENGTH = 4K
    FLASH                             : ORIGIN = 0x10007000, LENGTH = 832K
    DFU                               : ORIGIN = 0x100D7000, LENGTH = 836K

    /* Use all RAM banks as one big block   */
    RAM   : ORIGIN = 0x20000000, LENGTH = 264K
}

__bootloader_state_start = ORIGIN(BOOTLOADER_STATE) - ORIGIN(BOOT2);
__bootloader_state_end = ORIGIN(BOOTLOADER_STATE) + LENGTH(BOOTLOADER_STATE) - ORIGIN(BOOT2);

__bootloader_dfu_start = ORIGIN(DFU) - ORIGIN(BOOT2);
__bootloader_dfu_end = ORIGIN(DFU) + LENGTH(DFU) - ORIGIN(BOOT2);
