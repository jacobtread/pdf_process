use std::fmt::{Debug, Display};

/// Password for a DPF
#[derive(Debug, Clone)]
pub enum Password {
    /// Specify the owner password for the PDF file.  Providing this will bypass all security re‐strictions.
    Owner(Secret<String>),
    /// Specify the user password for the PDF file.
    User(Secret<String>),
}

impl Password {
    pub fn owner(value: impl Into<String>) -> Self {
        Self::Owner(Secret(value.into()))
    }

    pub fn user(value: impl Into<String>) -> Self {
        Self::User(Secret(value.into()))
    }

    pub fn push_arg(&self, args: &mut Vec<String>) {
        match self {
            Password::Owner(password) => {
                args.push("-opw".to_string());
                args.push(password.0.to_string())
            }
            Password::User(password) => {
                args.push("-upw".to_string());
                args.push(password.0.to_string())
            }
        }
    }
}

/// Wrapper around some value to hide the [Debug] and [Display] for
/// values that shouldn't be printed
#[derive(Clone)]
pub struct Secret<T>(pub T);

impl<T> From<T> for Secret<T> {
    fn from(value: T) -> Self {
        Self(value)
    }
}

impl<T> Debug for Secret<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("******")
    }
}

impl<T> Display for Secret<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("******")
    }
}
