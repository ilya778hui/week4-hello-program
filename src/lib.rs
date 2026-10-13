//! Неделя 4 — первая нативная Solana Program (без Anchor).
//!
//! Программа хранит в аккаунте счётчик приветствий и адрес последнего,
//! кто поздоровался. Инструкция `Greet { name }` увеличивает счётчик
//! и пишет приветствие в логи транзакции.

use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint,
    entrypoint::ProgramResult,
    msg,
    program_error::ProgramError,
    pubkey::Pubkey,
};

/// Данные, которые лежат в аккаунте программы (40 байт).
#[derive(BorshSerialize, BorshDeserialize, Debug, Default, PartialEq)]
pub struct GreetingAccount {
    pub counter: u64,
    pub last_greeter: Pubkey,
}

impl GreetingAccount {
    pub const LEN: usize = 8 + 32;
}

/// Инструкции программы. Первый байт instruction data — номер варианта.
#[derive(BorshSerialize, BorshDeserialize, Debug, PartialEq)]
pub enum HelloInstruction {
    /// Аккаунты: [0] greeting account (writable, owner = эта программа), [1] greeter (signer)
    Greet { name: String },
}

entrypoint!(process_instruction);

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    let instruction = HelloInstruction::try_from_slice(instruction_data)
        .map_err(|_| ProgramError::InvalidInstructionData)?;

    match instruction {
        HelloInstruction::Greet { name } => greet(program_id, accounts, name),
    }
}

fn greet(program_id: &Pubkey, accounts: &[AccountInfo], name: String) -> ProgramResult {
    let iter = &mut accounts.iter();
    let greeting_account = next_account_info(iter)?;
    let greeter = next_account_info(iter)?;

    // 1. Проверки безопасности
    if greeting_account.owner != program_id {
        msg!("Ошибка: аккаунт не принадлежит этой программе");
        return Err(ProgramError::IncorrectProgramId);
    }
    if !greeting_account.is_writable {
        return Err(ProgramError::InvalidAccountData);
    }
    if !greeter.is_signer {
        msg!("Ошибка: greeter должен подписать транзакцию");
        return Err(ProgramError::MissingRequiredSignature);
    }
    if name.is_empty() || name.len() > 32 {
        msg!("Ошибка: имя должно быть от 1 до 32 байт");
        return Err(ProgramError::InvalidArgument);
    }

    // 2. Читаем состояние, меняем, записываем обратно
    let mut data = greeting_account.try_borrow_mut_data()?;
    if data.len() < GreetingAccount::LEN {
        return Err(ProgramError::AccountDataTooSmall);
    }
    let mut state = GreetingAccount::try_from_slice(&data[..GreetingAccount::LEN])?;
    state.counter = state.counter.checked_add(1).ok_or(ProgramError::ArithmeticOverflow)?;
    state.last_greeter = *greeter.key;
    state.serialize(&mut &mut data[..GreetingAccount::LEN])?;

    msg!("Hello, {}! 👋", name);
    msg!("Всего приветствий: {}", state.counter);
    msg!("Последний greeter: {}", greeter.key);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ix(name: &str) -> Vec<u8> {
        borsh::to_vec(&HelloInstruction::Greet { name: name.to_string() }).unwrap()
    }

    #[test]
    fn greet_increments_counter() {
        let program_id = Pubkey::new_unique();
        let acc_key = Pubkey::new_unique();
        let greeter_key = Pubkey::new_unique();
        let (mut l1, mut l2) = (1_000_000u64, 1_000_000u64);
        let mut data = vec![0u8; GreetingAccount::LEN];
        let mut empty: Vec<u8> = vec![];
        let system = Pubkey::default();

        let acc = AccountInfo::new(&acc_key, false, true, &mut l1, &mut data, &program_id, false);
        let greeter = AccountInfo::new(&greeter_key, true, true, &mut l2, &mut empty, &system, false);
        let accounts = vec![acc, greeter];

        process_instruction(&program_id, &accounts, &ix("Ilya")).unwrap();
        process_instruction(&program_id, &accounts, &ix("AlmaU")).unwrap();

        let state = GreetingAccount::try_from_slice(&accounts[0].data.borrow()).unwrap();
        assert_eq!(state.counter, 2);
        assert_eq!(state.last_greeter, greeter_key);
    }

    #[test]
    fn rejects_foreign_account() {
        let program_id = Pubkey::new_unique();
        let other_program = Pubkey::new_unique();
        let (k1, k2) = (Pubkey::new_unique(), Pubkey::new_unique());
        let (mut l1, mut l2) = (0u64, 0u64);
        let mut data = vec![0u8; GreetingAccount::LEN];
        let mut empty: Vec<u8> = vec![];

        // аккаунт принадлежит чужой программе
        let acc = AccountInfo::new(&k1, false, true, &mut l1, &mut data, &other_program, false);
        let greeter = AccountInfo::new(&k2, true, true, &mut l2, &mut empty, &program_id, false);
        let accounts = vec![acc, greeter];
        assert_eq!(
            process_instruction(&program_id, &accounts, &ix("x")),
            Err(ProgramError::IncorrectProgramId)
        );
    }

    #[test]
    fn rejects_missing_signature() {
        let program_id = Pubkey::new_unique();
        let (k1, k2) = (Pubkey::new_unique(), Pubkey::new_unique());
        let (mut l1, mut l2) = (0u64, 0u64);
        let mut data = vec![0u8; GreetingAccount::LEN];
        let mut empty: Vec<u8> = vec![];

        let acc = AccountInfo::new(&k1, false, true, &mut l1, &mut data, &program_id, false);
        // greeter НЕ подписал транзакцию (is_signer = false)
        let greeter = AccountInfo::new(&k2, false, true, &mut l2, &mut empty, &program_id, false);
        let accounts = vec![acc, greeter];
        assert_eq!(
            process_instruction(&program_id, &accounts, &ix("x")),
            Err(ProgramError::MissingRequiredSignature)
        );
    }

    #[test]
    fn rejects_garbage_instruction_data() {
        let program_id = Pubkey::new_unique();
        assert_eq!(
            process_instruction(&program_id, &[], &[9, 9, 9]),
            Err(ProgramError::InvalidInstructionData)
        );
    }
}