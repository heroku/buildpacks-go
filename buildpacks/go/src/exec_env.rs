use libcnb::Env;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ExecutionEnvironment {
    Production,
    Test,
}

#[derive(Debug, PartialEq, Eq, thiserror::Error)]
#[error(
    "Unsupported execution environment: `CNB_EXEC_ENV={value}`. Supported values are `production` and `test`."
)]
pub(crate) struct ExecutionEnvironmentError {
    pub(crate) value: String,
}

impl ExecutionEnvironment {
    pub(crate) fn from_env(env: &Env) -> Result<Self, ExecutionEnvironmentError> {
        match env.get_string_lossy("CNB_EXEC_ENV").as_deref() {
            None | Some("production") => Ok(ExecutionEnvironment::Production),
            Some("test") => Ok(ExecutionEnvironment::Test),
            Some(other) => Err(ExecutionEnvironmentError {
                value: other.to_string(),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_env_not_set() {
        assert_eq!(
            ExecutionEnvironment::from_env(&Env::new()),
            Ok(ExecutionEnvironment::Production)
        );
    }

    #[test]
    fn test_from_env_production() {
        let mut env = Env::new();
        env.insert("CNB_EXEC_ENV", "production");
        assert_eq!(
            ExecutionEnvironment::from_env(&env),
            Ok(ExecutionEnvironment::Production)
        );
    }

    #[test]
    fn test_from_env_test() {
        let mut env = Env::new();
        env.insert("CNB_EXEC_ENV", "test");
        assert_eq!(
            ExecutionEnvironment::from_env(&env),
            Ok(ExecutionEnvironment::Test)
        );
    }

    #[test]
    fn test_from_env_invalid() {
        let mut env = Env::new();
        env.insert("CNB_EXEC_ENV", "invalid");
        let result = ExecutionEnvironment::from_env(&env);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().value, "invalid");
    }
}
