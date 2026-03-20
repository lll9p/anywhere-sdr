use std::path::PathBuf;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::{
    app::{App, InputMode},
    worker::describe_sinks,
};
use crate::{
    cli::TxBackend,
    tui_config::{ManualMotionConfig, TuiConfig},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum EditField {
    Ephemerides,
    Output,
    Frequency,
    Bits,
    Duration,
    HackrfSerial,
    HackrfRfFreqHz,
    ManualInitialLlh,
    ManualHeading,
    ManualCruiseSpeed,
    ManualAccelLimit,
    ManualTurnRate,
}

impl EditField {
    pub(super) fn label(self) -> &'static str {
        match self {
            Self::Ephemerides => "ephemerides",
            Self::Output => "output",
            Self::Frequency => "frequency",
            Self::Bits => "bits",
            Self::Duration => "duration",
            Self::HackrfSerial => "hackrf_serial",
            Self::HackrfRfFreqHz => "hackrf_rf_freq_hz",
            Self::ManualInitialLlh => "manual_initial_llh",
            Self::ManualHeading => "manual_heading_deg",
            Self::ManualCruiseSpeed => "manual_cruise_speed_mps",
            Self::ManualAccelLimit => "manual_accel_limit_mps2",
            Self::ManualTurnRate => "manual_turn_rate_limit_dps",
        }
    }
}

pub(super) fn begin_edit(app: &mut App, field: EditField) {
    app.input_mode = InputMode::Editing;
    app.input_field = Some(field);
    app.input_buffer = match field {
        EditField::Ephemerides => app
            .config
            .ephemerides
            .as_ref()
            .map(|path| path.display().to_string())
            .unwrap_or_default(),
        EditField::Output => app
            .config
            .output
            .as_ref()
            .map(|path| path.display().to_string())
            .unwrap_or_default(),
        EditField::Frequency => app.config.frequency.to_string(),
        EditField::Bits => app.config.bits.to_string(),
        EditField::Duration => app
            .config
            .duration
            .map_or_else(String::new, |duration| duration.to_string()),
        EditField::HackrfSerial => {
            app.config.hackrf_serial.clone().unwrap_or_default()
        }
        EditField::HackrfRfFreqHz => app.config.hackrf_rf_freq_hz.to_string(),
        EditField::ManualInitialLlh => app
            .config
            .manual_motion
            .initial_llh
            .map_or_else(String::new, |[latitude, longitude, height]| {
                format!("{latitude:.6},{longitude:.6},{height:.1}")
            }),
        EditField::ManualHeading => {
            app.config.manual_motion.initial_heading_deg.to_string()
        }
        EditField::ManualCruiseSpeed => {
            app.config.manual_motion.cruise_speed_mps.to_string()
        }
        EditField::ManualAccelLimit => {
            app.config.manual_motion.accel_limit_mps2.to_string()
        }
        EditField::ManualTurnRate => {
            app.config.manual_motion.turn_rate_limit_dps.to_string()
        }
    };
    app.message = Some(format!(
        "editing {} (Enter=save, Esc=cancel)",
        field.label()
    ));
}

pub(super) fn handle_edit_key(app: &mut App, key: KeyEvent) {
    let Some(field) = app.input_field else {
        app.input_mode = InputMode::Normal;
        return;
    };

    match key.code {
        KeyCode::Esc => {
            app.input_mode = InputMode::Normal;
            app.input_field = None;
            app.input_buffer.clear();
            app.message = None;
        }
        KeyCode::Enter => match apply_edit(app, field) {
            Ok(()) => {
                app.input_mode = InputMode::Normal;
                app.input_field = None;
                app.input_buffer.clear();
                app.message = None;
                app.sinks_desc = describe_sinks(&app.config);
            }
            Err(message) => {
                app.message = Some(message);
            }
        },
        KeyCode::Backspace => {
            app.input_buffer.pop();
        }
        KeyCode::Char(ch) => {
            if !key.modifiers.contains(KeyModifiers::CONTROL) {
                app.input_buffer.push(ch);
            }
        }
        _ => {}
    }
}

pub(super) fn toggle_motion_source(app: &mut App) {
    app.config.motion_source = app.config.motion_source.toggle();
    app.message = app.config.uses_manual_motion().then(|| {
        "manual mode runs until cancel; duration is ignored".to_string()
    });
}

pub(super) fn toggle_tx_backend_and_update_status(
    app: &mut App, backend: TxBackend,
) {
    toggle_tx_backend(&mut app.config.tx, backend);
    app.sinks_desc = describe_sinks(&app.config);

    if !app.config.tx.is_empty() && app.config.output.is_none() {
        app.message = Some(
            "tx enabled: file output disabled unless output path is set \
             (press o)"
                .to_string(),
        );
    } else {
        app.message = None;
    }
}

fn apply_edit(app: &mut App, field: EditField) -> Result<(), String> {
    let value = app.input_buffer.trim();

    match field {
        EditField::ManualInitialLlh
        | EditField::ManualHeading
        | EditField::ManualCruiseSpeed
        | EditField::ManualAccelLimit
        | EditField::ManualTurnRate => {
            apply_manual_edit(&mut app.config.manual_motion, field, value)
        }
        _ => apply_general_edit(&mut app.config, field, value),
    }
}

fn apply_general_edit(
    config: &mut TuiConfig, field: EditField, value: &str,
) -> Result<(), String> {
    match field {
        EditField::Ephemerides => {
            config.ephemerides = optional_path(value);
        }
        EditField::Output => {
            config.output = optional_path(value);
        }
        EditField::Frequency => {
            config.frequency = parse_nonzero_usize(value, "frequency")?;
        }
        EditField::Bits => {
            let bits = value
                .parse::<usize>()
                .map_err(|err| format!("invalid bits: {err}"))?;
            if !matches!(bits, 1 | 8 | 16) {
                return Err("bits must be one of: 1, 8, 16".to_string());
            }
            config.bits = bits;
        }
        EditField::Duration => {
            config.duration = optional_f64(value, "duration")?;
        }
        EditField::HackrfSerial => {
            config.hackrf_serial = optional_string(value);
        }
        EditField::HackrfRfFreqHz => {
            config.hackrf_rf_freq_hz = parse_nonzero_u64(
                value,
                "hackrf_rf_freq_hz",
                "invalid rf freq",
            )?;
        }
        _ => {
            return Err(format!(
                "unsupported general config edit: {}",
                field.label()
            ));
        }
    }

    Ok(())
}

fn apply_manual_edit(
    manual: &mut ManualMotionConfig, field: EditField, value: &str,
) -> Result<(), String> {
    match field {
        EditField::ManualInitialLlh => {
            manual.initial_llh = if value.is_empty() {
                None
            } else {
                Some(parse_llh_triplet(value)?)
            };
        }
        EditField::ManualHeading => {
            let heading_deg =
                parse_f64(value, "manual heading", "invalid manual heading")?;
            if !heading_deg.is_finite() {
                return Err("manual heading must be finite".to_string());
            }
            manual.initial_heading_deg = heading_deg;
        }
        EditField::ManualCruiseSpeed => {
            manual.cruise_speed_mps =
                parse_non_negative_f64(value, "cruise speed")?;
        }
        EditField::ManualAccelLimit => {
            manual.accel_limit_mps2 = parse_positive_f64(value, "accel limit")?;
        }
        EditField::ManualTurnRate => {
            manual.turn_rate_limit_dps =
                parse_positive_f64(value, "turn rate limit")?;
        }
        _ => {
            return Err(format!(
                "unsupported manual config edit: {}",
                field.label()
            ));
        }
    }

    Ok(())
}

fn optional_path(value: &str) -> Option<PathBuf> {
    (!value.is_empty()).then(|| PathBuf::from(value))
}

fn optional_string(value: &str) -> Option<String> {
    (!value.is_empty()).then(|| value.to_string())
}

fn optional_f64(value: &str, label: &str) -> Result<Option<f64>, String> {
    if value.is_empty() {
        Ok(None)
    } else {
        parse_f64(value, label, &format!("invalid {label}")).map(Some)
    }
}

fn parse_f64(
    value: &str, label: &str, error_prefix: &str,
) -> Result<f64, String> {
    value
        .parse::<f64>()
        .map_err(|err| format!("{error_prefix}: {err}"))
        .and_then(|parsed| {
            if parsed.is_finite() {
                Ok(parsed)
            } else {
                Err(format!("{label} must be finite"))
            }
        })
}

fn parse_non_negative_f64(value: &str, label: &str) -> Result<f64, String> {
    let parsed = parse_f64(value, label, &format!("invalid {label}"))?;
    if parsed < 0.0 {
        Err(format!("manual {label} must be >= 0"))
    } else {
        Ok(parsed)
    }
}

fn parse_positive_f64(value: &str, label: &str) -> Result<f64, String> {
    let parsed = parse_f64(value, label, &format!("invalid {label}"))?;
    if parsed <= 0.0 {
        Err(format!("manual {label} must be > 0"))
    } else {
        Ok(parsed)
    }
}

fn parse_nonzero_usize(value: &str, label: &str) -> Result<usize, String> {
    let parsed = value
        .parse::<usize>()
        .map_err(|err| format!("invalid {label}: {err}"))?;
    if parsed == 0 {
        Err(format!("{label} must be > 0"))
    } else {
        Ok(parsed)
    }
}

fn parse_nonzero_u64(
    value: &str, label: &str, error_prefix: &str,
) -> Result<u64, String> {
    let parsed = value
        .parse::<u64>()
        .map_err(|err| format!("{error_prefix}: {err}"))?;
    if parsed == 0 {
        Err(format!("{label} must be > 0"))
    } else {
        Ok(parsed)
    }
}

fn parse_llh_triplet(value: &str) -> Result<[f64; 3], String> {
    let parts = value
        .split(',')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>();
    if parts.len() != 3 {
        return Err(
            "manual_initial_llh must be latitude,longitude,height".to_string()
        );
    }

    let latitude_deg = parts[0]
        .parse::<f64>()
        .map_err(|err| format!("invalid latitude: {err}"))?;
    let longitude_deg = parts[1]
        .parse::<f64>()
        .map_err(|err| format!("invalid longitude: {err}"))?;
    let height_m = parts[2]
        .parse::<f64>()
        .map_err(|err| format!("invalid height: {err}"))?;

    if !(-90.0..=90.0).contains(&latitude_deg) {
        return Err("manual latitude must be in -90..=90".to_string());
    }
    if !(-180.0..=180.0).contains(&longitude_deg) {
        return Err("manual longitude must be in -180..=180".to_string());
    }
    if !height_m.is_finite() {
        return Err("manual height must be finite".to_string());
    }

    Ok([latitude_deg, longitude_deg, height_m])
}

fn toggle_tx_backend(tx: &mut Vec<TxBackend>, backend: TxBackend) {
    if let Some(position) = tx.iter().position(|existing| *existing == backend)
    {
        tx.remove(position);
    } else {
        tx.push(backend);
    }
}
