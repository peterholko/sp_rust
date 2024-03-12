use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};

use crate::obj::HeroClassList;
use thiserror::Error;

pub type Accounts = Arc<Mutex<HashMap<i32, Account>>>;

#[derive(Clone, Debug)]
pub struct Account {
    pub player_id: i32,
    pub username: String,
    pub password: String,
    pub class: HeroClassList,
}

#[derive(Error, Debug)]
pub enum AccountError {
    #[error("Incorrect Password")]
    IncorrectPassword,
}

impl Account {
    pub fn new(player_id: i32, username: String, password: String) -> Account {
        let password_bytes = password.as_bytes();

        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();

        let password_hash = argon2
            .hash_password(password_bytes, &salt)
            .unwrap()
            .to_string();

        Account {
            player_id: player_id,
            username: username,
            password: password_hash,
            class: HeroClassList::None,
        }
    }

    pub fn verify_password(password: String, account_password: String) -> Result<(), AccountError> {
        let password_bytes = password.as_bytes();
        let parsed_hash = PasswordHash::new(&account_password).unwrap();

        let result = Argon2::default().verify_password(password_bytes, &parsed_hash);

        match result {
            Ok(_) => Ok(()),
            Err(e) => Err(AccountError::IncorrectPassword),
        }
    }
}
