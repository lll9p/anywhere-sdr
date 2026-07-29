use nusb::transfer::Direction;

use crate::{
    error::Error,
    hackrf::{
        HackRF, decode_board_id, decode_firmware_version, decode_gain_ack,
        decode_part_id_serial, exact_control_reply,
    },
};

fn assert_control_transfer<const N: usize>(actual: usize) {
    match exact_control_reply::<N>(vec![0; actual]) {
        Err(Error::ControlTransfer {
            direction,
            actual: transferred,
            expected,
        }) => {
            assert_eq!(direction, Direction::In);
            assert_eq!(transferred, actual);
            assert_eq!(expected, N);
        }
        Err(error) => panic!("unexpected error: {error}"),
        Ok(_) => panic!("accepted a {actual}-byte reply as {N} bytes"),
    }
}

#[test]
fn control_exact_one_byte_reply_validates_cardinality() {
    assert_control_transfer::<1>(0);
    assert_control_transfer::<1>(2);

    match exact_control_reply::<1>(vec![0xa5]) {
        Ok(data) => assert_eq!(data, [0xa5]),
        Err(error) => panic!("exact reply failed: {error}"),
    }
}

#[test]
fn control_exact_part_serial_reply_validates_cardinality() {
    for actual in [0, 23, 25] {
        assert_control_transfer::<24>(actual);
    }

    let expected = [0x5a; 24];
    match exact_control_reply::<24>(expected.to_vec()) {
        Ok(data) => assert_eq!(data, expected),
        Err(error) => panic!("exact reply failed: {error}"),
    }
}

#[test]
fn control_board_id_decoder_preserves_byte() {
    assert_eq!(decode_board_id([0x7f]), 0x7f);
}

#[test]
fn control_part_serial_decoder_preserves_wire_order_and_format() {
    let words: [u32; 6] = [
        0x1122_3344,
        0xa1b2_c3d4,
        0x0000_0001,
        0x00ab_cdef,
        0x1234_5678,
        0x0000_abcd,
    ];
    let mut data = [0; 24];
    for (chunk, word) in data.chunks_exact_mut(4).zip(words) {
        chunk.copy_from_slice(&word.to_le_bytes());
    }

    match decode_part_id_serial(data) {
        Ok((part_id, serial)) => {
            assert_eq!(part_id, (0x1122_3344, 0xa1b2_c3d4));
            assert_eq!(serial, "0000000100abcdef123456780000abcd");
            assert_eq!(serial.len(), 32);
        }
        Err(error) => panic!("part/serial decode failed: {error}"),
    }
}

#[test]
fn control_gain_acknowledgement_preserves_semantics() {
    assert!(matches!(decode_gain_ack([0]), Err(Error::Argument)));
    assert!(decode_gain_ack([1]).is_ok());
    assert!(decode_gain_ack([u8::MAX]).is_ok());
}

#[test]
fn control_firmware_version_remains_bounded_variable_data() {
    assert_eq!(decode_firmware_version(Vec::new()), "");
    assert_eq!(decode_firmware_version(b"v1.2".to_vec()), "v1.2");
    assert_eq!(
        decode_firmware_version(b"0123456789abcdef".to_vec()),
        "0123456789abcdef"
    );
    assert_eq!(decode_firmware_version(vec![b'v', 0xff]), "v\u{fffd}");
}

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
