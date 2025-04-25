use crate::{error::Error, hackrf::HackRF};

/// Tests in this module require physical `HackRF` hardware to be connected.
///
/// These tests are marked with `#[ignore]` to prevent them from running during
/// normal test execution. To run these tests, use the following command:
///
/// ```bash
/// cargo test -p libhackrf -- --ignored
/// ```
///
/// Lists all connected `HackRF` devices
///
/// This test requires a `HackRF` device to be connected to the system.
#[test]
#[ignore = "Requires HackRF hardware"]
fn list_device() -> Result<(), Error> {
    let devices = HackRF::list_devices()?;
    println!("Found {} devices", devices.len());
    println!("{devices:#?}");
    Ok(())
}

/// Retrieves and displays information about the connected `HackRF` device
///
/// This test requires a `HackRF` device to be connected to the system.
#[test]
#[ignore = "Requires HackRF hardware"]
fn hackrf_info() -> Result<(), Error> {
    let sdr: HackRF = HackRF::new_auto()?;
    println!("Board ID: {}", sdr.board_id()?);
    println!("Firmware version: {}", sdr.version()?);
    println!("API version: {}", sdr.device_version());

    let part_and_serial: ((u32, u32), String) = sdr.part_id_serial_read()?;
    println!(
        "{}",
        format_args!(
            "Part ID number: 0x{:08x?} 0x{:08x?}\nSerial number: {:032x?}",
            part_and_serial.0.0, part_and_serial.0.1, part_and_serial.1,
        )
    );
    Ok(())
}

/// Tests setting the frequency on a `HackRF` device
///
/// This test requires a `HackRF` device to be connected to the system.
/// It sets the frequency to 1575.42 MHz (GPS L1 frequency).
#[test]
#[ignore = "Requires HackRF hardware"]
fn hackrf_setting() -> Result<(), Error> {
    let mut sdr: HackRF = HackRF::new_auto()?;
    sdr.set_freq(1_575_420_000)?;
    Ok(())
}

/// Tests resetting a `HackRF` device
///
/// This test requires a `HackRF` device to be connected to the system.
/// It performs a full reset of the device.
#[test]
#[ignore = "Requires HackRF hardware"]
fn hackrf_reset() -> Result<(), Error> {
    let sdr: HackRF = HackRF::new_auto()?;
    sdr.reset()?;
    Ok(())
}
