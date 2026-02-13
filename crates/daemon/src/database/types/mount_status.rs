use serde::{Deserialize, Serialize};
use sqlx::encode::IsNull;
use sqlx::error::BoxDynError;
use sqlx::sqlite::{SqliteArgumentValue, SqliteTypeInfo, SqliteValueRef};
use sqlx::{Decode, Encode, Sqlite, Type};

/// Mount status enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MountStatus {
    Stopped,
    Starting,
    Running,
    Stopping,
    Error,
}

impl MountStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            MountStatus::Stopped => "stopped",
            MountStatus::Starting => "starting",
            MountStatus::Running => "running",
            MountStatus::Stopping => "stopping",
            MountStatus::Error => "error",
        }
    }
}

impl std::str::FromStr for MountStatus {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "stopped" => MountStatus::Stopped,
            "starting" => MountStatus::Starting,
            "running" => MountStatus::Running,
            "stopping" => MountStatus::Stopping,
            "error" => MountStatus::Error,
            _ => MountStatus::Stopped,
        })
    }
}

impl std::fmt::Display for MountStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl Decode<'_, Sqlite> for MountStatus {
    fn decode(value: SqliteValueRef<'_>) -> Result<Self, BoxDynError> {
        let s = <String as Decode<Sqlite>>::decode(value)?;
        Ok(s.parse().unwrap())
    }
}

impl Encode<'_, Sqlite> for MountStatus {
    fn encode_by_ref(
        &self,
        args: &mut Vec<SqliteArgumentValue<'_>>,
    ) -> Result<IsNull, BoxDynError> {
        args.push(SqliteArgumentValue::Text(self.as_str().into()));
        Ok(IsNull::No)
    }
}

impl Type<Sqlite> for MountStatus {
    fn compatible(ty: &SqliteTypeInfo) -> bool {
        <String as Type<Sqlite>>::compatible(ty)
    }

    fn type_info() -> SqliteTypeInfo {
        <String as Type<Sqlite>>::type_info()
    }
}
