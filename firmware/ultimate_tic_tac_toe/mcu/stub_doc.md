# TinyUSB ESP32 Platform Stub Functions

This document describes the stub functions that must be implemented by the embedder when using TinyUSB on ESP32 platforms without ESP-IDF dependencies.

## Overview

The TinyUSB ESP32 port has been modified to remove direct dependencies on ESP-IDF components like FreeRTOS, esp_intr_alloc, and esp_log. Instead, it uses weak stub functions that must be implemented by the embedder platform.

## Required Stub Functions

### Interrupt Management

#### `tusb_esp32_int_enable`
```c
void tusb_esp32_int_enable(uint32_t irq_num, void (*handler)(void*), void* arg);
```

**Purpose:** Enable an interrupt and attach a handler function.

**Parameters:**
- `irq_num`: The interrupt number to enable (e.g., ETS_USB_INTR_SOURCE = 38 for ESP32-S2/S3)
- `handler`: Function pointer to the interrupt handler
- `arg`: Argument to pass to the handler function

**Behavior:** 
- Configure the interrupt controller to enable the specified interrupt
- Register the handler function to be called when the interrupt occurs
- Pass the provided argument to the handler when invoked
- Should handle interrupt priority appropriately (medium priority recommended)

#### `tusb_esp32_int_disable`
```c
void tusb_esp32_int_disable(uint32_t irq_num);
```

**Purpose:** Disable a previously enabled interrupt.

**Parameters:**
- `irq_num`: The interrupt number to disable

**Behavior:**
- Disable the specified interrupt in the interrupt controller
- Clean up any associated interrupt handler registration

### Timing Functions

#### `tusb_esp32_delay_ms`
```c
void tusb_esp32_delay_ms(uint32_t ms);
```

**Purpose:** Provide a blocking delay in milliseconds.

**Parameters:**
- `ms`: Number of milliseconds to delay

**Behavior:**
- Block execution for the specified number of milliseconds
- Used specifically for USB remote wakeup timing (typically called with 1ms)
- Must be accurate enough for USB timing requirements
- Can be implemented using hardware timers or system tick counters

### Logging Functions (Optional)

#### `tusb_esp32_logv`
```c
void tusb_esp32_logv(const char *tag, const char *fmt, ...);
```

**Purpose:** Log verbose/debug messages during normal operation.

**Parameters:**
- `tag`: Log tag string (e.g., "TUSB:DCD")
- `fmt`: Printf-style format string
- `...`: Variable arguments for format string

**Behavior:**
- Output formatted log message for debugging purposes
- Can be implemented as no-op if logging not desired
- Typically used for non-time-critical debug output

#### `tusb_esp32_early_logv`
```c
void tusb_esp32_early_logv(const char *tag, const char *fmt, ...);
```

**Purpose:** Log verbose/debug messages from interrupt context.

**Parameters:**
- `tag`: Log tag string
- `fmt`: Printf-style format string  
- `...`: Variable arguments for format string

**Behavior:**
- Output formatted log message from interrupt handlers
- Must be interrupt-safe (avoid blocking operations)
- Can be implemented as no-op if logging not desired
- Should use interrupt-safe logging mechanisms

### Cache Management Functions (Conditional)

These functions are only required when DMA is enabled (`CFG_TUD_DWC2_DMA_ENABLE` or `CFG_TUH_DWC2_DMA_ENABLE`) and the SoC uses internal L1 cache (`SOC_CACHE_INTERNAL_MEM_VIA_L1CACHE`).

#### `tusb_esp32_dcache_clean`
```c
bool tusb_esp32_dcache_clean(const void* addr, uint32_t size);
```

**Purpose:** Clean (write-back) data cache for specified memory region.

**Parameters:**
- `addr`: Starting address of memory region
- `size`: Size of memory region in bytes

**Returns:**
- `true` on success, `false` on failure

**Behavior:**
- Write any dirty cache lines back to memory
- Ensure CPU writes are visible to DMA hardware
- Must handle cache line alignment internally

#### `tusb_esp32_dcache_invalidate`
```c
bool tusb_esp32_dcache_invalidate(const void* addr, uint32_t size);
```

**Purpose:** Invalidate data cache for specified memory region.

**Parameters:**
- `addr`: Starting address of memory region
- `size`: Size of memory region in bytes

**Returns:**
- `true` on success, `false` on failure

**Behavior:**
- Mark cache lines as invalid, forcing reload from memory
- Ensure CPU reads get fresh data written by DMA hardware
- Must handle cache line alignment internally

#### `tusb_esp32_dcache_clean_invalidate`
```c
bool tusb_esp32_dcache_clean_invalidate(const void* addr, uint32_t size);
```

**Purpose:** Both clean and invalidate data cache for specified memory region.

**Parameters:**
- `addr`: Starting address of memory region
- `size`: Size of memory region in bytes

**Returns:**
- `true` on success, `false` on failure

**Behavior:**
- Combines clean and invalidate operations
- Write back dirty data then mark cache lines invalid
- Must handle cache line alignment internally

## Implementation Notes

### Weak Linkage
All stub functions are marked with `TU_ATTR_WEAK`, allowing them to be overridden at link time. If not implemented, they will be no-ops (for logging functions) or may cause link errors (for required functions).

### Thread Safety
The interrupt management functions may be called from different contexts and should be implemented with appropriate thread safety considerations.

### Platform Integration
These stubs provide the integration points between TinyUSB and the target platform's hardware abstraction layer, RTOS, or bare-metal environment.
