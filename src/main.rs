//! # UART DMA Example
//!
//! This application demonstrates how to use the UART peripheral with the
//! DMA controller.
//!
//! It may need to be adapted to your particular board layout and/or pin
//! assignment.
//!
//! See the `Cargo.toml` file for Copyright and license details.

#![no_std]
#![no_main]

use hal::singleton;
// Ensure we halt the program on panic (if we don't mention this crate it won't
// be linked)
use panic_halt as _;

// Alias for our HAL crate
use rp235x_hal as hal;

// Some things we need
use embedded_hal::delay::DelayNs;
use hal::clocks::Clock;
use hal::dma::DMAExt;
use hal::fugit::RateExtU32;

// UART related types
use hal::uart::{DataBits, StopBits, UartConfig};

/// Tell the Boot ROM about our application
#[link_section = ".start_block"]
#[used]
pub static IMAGE_DEF: hal::block::ImageDef = hal::block::ImageDef::secure_exe();

/// External high-speed crystal on the Raspberry Pi Pico 2 board is 12 MHz.
/// Adjust if your board has a different frequency
const XTAL_FREQ_HZ: u32 = 12_000_000u32;

/// Entry point to our bare-metal application.
///
/// The `#[hal::entry]` macro ensures the Cortex-M start-up code calls this function
/// as soon as all global variables and the spinlock are initialised.
///
/// The function configures the rp235x peripherals, then writes to the UART in
/// an infinite loop.
#[hal::entry]
fn main() -> ! {
    // Grab our singleton objects
    let mut pac = hal::pac::Peripherals::take().unwrap();

    // Set up the watchdog driver - needed by the clock setup code
    let mut watchdog = hal::Watchdog::new(pac.WATCHDOG);

    // Configure the clocks
    let clocks = hal::clocks::init_clocks_and_plls(
        XTAL_FREQ_HZ,
        pac.XOSC,
        pac.CLOCKS,
        pac.PLL_SYS,
        pac.PLL_USB,
        &mut pac.RESETS,
        &mut watchdog,
    )
    .unwrap();

    let mut delay = hal::Timer::new_timer0(pac.TIMER0, &mut pac.RESETS, &clocks);

    // The single-cycle I/O block controls our GPIO pins
    let sio = hal::Sio::new(pac.SIO);

    // Set the pins to their default state
    let pins = hal::gpio::Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );

    let uart_pins = (
        // UART TX (characters sent from rp235x) on pin 1 (GPIO0)
        pins.gpio0.into_function(),
        // UART RX (characters received by rp235x) on pin 2 (GPIO1)
        pins.gpio1.into_function(),
    );
    let uart = hal::uart::UartPeripheral::new(pac.UART0, uart_pins, &mut pac.RESETS)
        .enable(
            UartConfig::new(115200.Hz(), DataBits::Eight, None, StopBits::One),
            clocks.peripheral_clock.freq(),
        )
        .unwrap();
    // Initialize DMA.
    let dma = pac.DMA.split(&mut pac.RESETS);

    uart.write_full_blocking(b"\r\n\r\nHello Kubecon 2025 UK !!, UART DMA echo example\r\n\r\n");

    // In order to use DMA we need to split the UART into a RX (receive) and TX (transmit) pair
    let (rx, tx) = uart.split();

    // We can still write to the tx side of the UART after splitting
    tx.write_full_blocking(b"Regular UART write\r\n");

    // And we can DMA from a buffer into the UART
    let teststring = b"DMA UART write\r\n";
    let tx_transfer = hal::dma::single_buffer::Config::new(dma.ch0, teststring, tx).start();

    // Wait for the DMA transfer to finish so we can reuse the tx and the dma channel
    let (ch0, _teststring, tx) = tx_transfer.wait();

    // Let's test DMA RX into a buffer.
    tx.write_full_blocking(b"Waiting for you to type 5 letters...\r\n");
    let rx_buf = singleton!(: [u8; 5] = [0; 5]).unwrap();
    let rx_transfer = hal::dma::single_buffer::Config::new(ch0, rx, rx_buf).start();
    let (ch0, rx, rx_buf) = rx_transfer.wait();

    // Echo back the 5 characters the user typed
    tx.write_full_blocking(b"You wrote \"");
    tx.write_full_blocking(rx_buf);
    tx.write_full_blocking(b"\"\r\n");

    // Now just keep echoing anything that is received back out of TX
    tx.write_full_blocking(b"Now echoing any character you write...\r\n");
    let _tx_transfer = hal::dma::single_buffer::Config::new(ch0, rx, tx).start();

    loop {
        // everything should be handled by DMA, nothing else to do
        delay.delay_ms(1000);
    }
}

/// Program metadata for `picotool info`
#[link_section = ".bi_entries"]
#[used]
pub static PICOTOOL_ENTRIES: [hal::binary_info::EntryAddr; 5] = [
    hal::binary_info::rp_cargo_bin_name!(),
    hal::binary_info::rp_cargo_version!(),
    hal::binary_info::rp_program_description!(c"UART DMA Example"),
    hal::binary_info::rp_cargo_homepage_url!(),
    hal::binary_info::rp_program_build_attribute!(),
];

// End of file