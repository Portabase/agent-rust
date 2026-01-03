#[derive(Clone, Copy)]
pub enum BackupMethod {
    Automatic,
    Manual,
}

impl ToString for BackupMethod {
    fn to_string(&self) -> String {
        match self {
            BackupMethod::Automatic => "automatic".into(),
            BackupMethod::Manual => "manual".into(),
        }
    }
}
