//! Command-line modes of the single executable.

use crate::i18n::Strings;

/// Invalid command line; [`CliError::message`] renders it in the user's language.
/// The usage text is `Strings::cli_usage`.
#[derive(Debug, Clone, PartialEq)]
pub enum CliError {
    UnknownMode(String),
    SeletorTakesNoArgs,
    LupaArgCount,
    ZeroSize,
    InvalidValue { name: &'static str, value: String },
    InvalidZoom(String),
}

impl CliError {
    pub fn message(&self, s: &Strings) -> String {
        match self {
            Self::UnknownMode(mode) => (s.cli_unknown_mode)(mode),
            Self::SeletorTakesNoArgs => s.cli_seletor_no_args.to_owned(),
            Self::LupaArgCount => s.cli_lupa_args.to_owned(),
            Self::ZeroSize => s.cli_zero_size.to_owned(),
            Self::InvalidValue { name, value } => (s.cli_invalid_value)(name, value),
            Self::InvalidZoom(value) => (s.cli_invalid_zoom)(value),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Mode {
    Panel,
    Selector,
    Lupa(LupaArgs),
}

#[derive(Debug, Clone, PartialEq)]
pub struct LupaArgs {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
    /// `None` = use the zoom from the saved config.
    pub zoom: Option<f32>,
    pub passthrough: bool,
    /// Window position; when given it is also saved to the config.
    pub position: Option<[f32; 2]>,
}

/// Parses the arguments after the program name.
pub fn parse(args: &[String]) -> Result<Mode, CliError> {
    let Some(mode) = args.first() else {
        return Ok(Mode::Panel);
    };

    match mode.as_str() {
        "seletor" if args.len() == 1 => Ok(Mode::Selector),
        "seletor" => Err(CliError::SeletorTakesNoArgs),
        "lupa" => parse_lupa(&args[1..]).map(Mode::Lupa),
        other => Err(CliError::UnknownMode(other.to_owned())),
    }
}

fn parse_lupa(args: &[String]) -> Result<LupaArgs, CliError> {
    if !(4..=7).contains(&args.len()) || args.len() == 6 {
        return Err(CliError::LupaArgCount);
    }

    let mut lupa = LupaArgs {
        x: number(&args[0], "x")?,
        y: number(&args[1], "y")?,
        w: number(&args[2], "w")?,
        h: number(&args[3], "h")?,
        zoom: None,
        passthrough: false,
        position: None,
    };

    if lupa.w == 0 || lupa.h == 0 {
        return Err(CliError::ZeroSize);
    }

    if let Some(mode) = args.get(4) {
        if mode.eq_ignore_ascii_case("passthrough") {
            lupa.passthrough = true;
        } else {
            let zoom: f32 = number(mode, "zoom")?;
            if !(zoom.is_finite() && zoom > 0.0) {
                return Err(CliError::InvalidZoom(mode.clone()));
            }
            lupa.zoom = Some(zoom);
        }
    }

    if let (Some(px), Some(py)) = (args.get(5), args.get(6)) {
        lupa.position = Some([number(px, "pos_x")?, number(py, "pos_y")?]);
    }

    Ok(lupa)
}

fn number<T: std::str::FromStr>(value: &str, name: &'static str) -> Result<T, CliError> {
    value.parse().map_err(|_| CliError::InvalidValue {
        name,
        value: value.to_owned(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(s: &str) -> Vec<String> {
        s.split_whitespace().map(String::from).collect()
    }

    fn lupa(s: &str) -> LupaArgs {
        match parse(&args(s)) {
            Ok(Mode::Lupa(l)) => l,
            other => panic!("expected lupa mode, got {other:?}"),
        }
    }

    #[test]
    fn no_args_opens_panel() {
        assert_eq!(parse(&[]), Ok(Mode::Panel));
    }

    #[test]
    fn seletor_mode() {
        assert_eq!(parse(&args("seletor")), Ok(Mode::Selector));
        assert!(parse(&args("seletor extra")).is_err());
    }

    #[test]
    fn lupa_minimal() {
        let l = lupa("lupa 10 20 300 200");
        assert_eq!((l.x, l.y, l.w, l.h), (10, 20, 300, 200));
        assert_eq!((l.zoom, l.passthrough, l.position), (None, false, None));
    }

    #[test]
    fn lupa_with_zoom_and_position() {
        let l = lupa("lupa 10 20 300 200 2.5 50 60");
        assert_eq!(l.zoom, Some(2.5));
        assert_eq!(l.position, Some([50.0, 60.0]));
    }

    #[test]
    fn lupa_passthrough_is_case_insensitive() {
        let l = lupa("lupa 10 20 300 200 PassThrough");
        assert!(l.passthrough);
        assert_eq!(l.zoom, None);
    }

    #[test]
    fn lupa_rejects_bad_input() {
        for bad in [
            "lupa",
            "lupa 1 2 3",
            "lupa a 2 3 4",
            "lupa -1 2 3 4",
            "lupa 1 2 0 4",
            "lupa 1 2 3 4 abc",
            "lupa 1 2 3 4 0",
            "lupa 1 2 3 4 2 50",
            "lupa 1 2 3 4 2 50 x",
            "lupa 1 2 3 4 2 50 60 70",
        ] {
            assert!(parse(&args(bad)).is_err(), "should reject: {bad}");
        }
    }

    #[test]
    fn unknown_mode_is_error() {
        assert_eq!(
            parse(&args("lixo")),
            Err(CliError::UnknownMode("lixo".into()))
        );
    }

    #[test]
    fn errors_are_translated() {
        use crate::i18n::{tr, Lang};
        let err = parse(&args("lupa 1 2 x 4")).unwrap_err();
        assert_eq!(
            err,
            CliError::InvalidValue {
                name: "w",
                value: "x".into()
            }
        );
        assert!(err.message(tr(Lang::Pt)).contains("valor inválido"));
        assert!(err.message(tr(Lang::En)).contains("invalid value"));
    }
}
