// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use std::env;

use coinstr_sdk::core::Result;
use dialoguer::{Confirm, Input, Password};

pub fn get_input<S>(prompt: S) -> Result<String>
where
    S: Into<String>,
{
    Ok(Input::new().with_prompt(prompt).interact_text()?)
}

/// Get password
///
/// If the `COINSTR_PASSWORD` env variable exists, that will be used as the password,
/// otherwise it will be asked interactively.
pub fn get_password() -> Result<String> {
    Ok(Password::new().with_prompt("Password").interact()?)
}

pub fn get_new_password() -> Result<String> {
    Ok(Password::new().with_prompt("New password").interact()?)
}

pub fn get_confirmation_password() -> Result<String> {
    Ok(Password::new().with_prompt("Confirm password").interact()?)
}

pub fn ask<S>(prompt: S) -> Result<bool>
where
    S: Into<String> + std::marker::Copy,
{
    if Confirm::new()
        .with_prompt(prompt)
        .default(false)
        .interact()?
    {
        Ok(true)
    } else {
        Ok(false)
    }
}

pub fn get_password_from_env() -> Option<String> {
    env::var("COINSTR_PASSWORD").ok()
}
