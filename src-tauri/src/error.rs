use serde::{Serialize, Serializer, ser::SerializeStruct};

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("Ein Scan oder Update läuft bereits")]
    Busy,
    #[error("{0} ist nicht installiert")]
    ManagerUnavailable(String),
    #[error("Befehl fehlgeschlagen: {0}")]
    Command(String),
    #[error("Zeitüberschreitung: {0}")]
    Timeout(String),
    #[error("Ausgabe nicht lesbar: {0}")]
    Parse(String),
    #[error("Speicherfehler: {0}")]
    Storage(String),
    #[error("{0}")]
    Other(String),
}

impl AppError {
    fn kind(&self) -> &'static str {
        match self {
            Self::Busy => "busy",
            Self::ManagerUnavailable(_) => "managerUnavailable",
            Self::Command(_) => "command",
            Self::Timeout(_) => "timeout",
            Self::Parse(_) => "parse",
            Self::Storage(_) => "storage",
            Self::Other(_) => "other",
        }
    }
}

impl Serialize for AppError {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut state = serializer.serialize_struct("AppError", 2)?;
        state.serialize_field("kind", self.kind())?;
        state.serialize_field("message", &self.to_string())?;
        state.end()
    }
}

pub type AppResult<T> = Result<T, AppError>;
