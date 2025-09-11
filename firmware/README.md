This directory contains a few demo projects.

The first, [demo_neopixel_onboard](./demo_neopixel_onboard), simply flashes the single neopixel on the devboard. That is, it doesn't require any additional connected hardware.

The second, [demo_usb_keyboard](./demo_usb_keyboard), allows you to connect a USB keyboard the esp32. Note that you need to set the solder bridge.

The third, [demo_tic_tac_toe](./demo_tic_tac_toe), is an extension of the keyboard demo and implements a simple Tic Tac Toe game, with the game logic exported from matlab, on a 16x16 neopixel matrix.