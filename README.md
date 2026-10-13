# hello-program — неделя 4: первая Solana Program

Нативная программа Solana на Rust (без Anchor). Хранит в своём аккаунте счётчик приветствий
и адрес последнего пользователя. Клиент на JavaScript (`@solana/web3.js`) создаёт аккаунт и вызывает программу.

## Архитектура

- **Program** (`src/lib.rs`) — `entrypoint!` → `process_instruction` → `match` по `HelloInstruction`.
- **Instruction data** — Borsh: `[0]` (номер варианта `Greet`) + `u32` длина имени + байты имени.
- **Аккаунт данных** (40 байт): `counter: u64` + `last_greeter: Pubkey`. Владелец — эта программа.
- **Проверки безопасности**: владелец аккаунта = program_id, аккаунт writable,
  greeter подписал транзакцию, длина имени 1–32 байта, `checked_add` против переполнения.

## Клиент (`client/client.mjs`)

1. Если аккаунта данных нет — создаёт его через `SystemProgram.createAccount` (rent-exempt, owner = программа).
2. Отправляет инструкцию `Greet { name }`.
3. Печатает логи программы и читает состояние аккаунта.

## Запуск

```bash
cargo test                          # 4 unit-теста
cargo build-sbf --arch v3           # сборка (.so в target/deploy)
solana-test-validator --reset       # в отдельном терминале — локальный блокчейн
solana airdrop 10 --url localhost
solana program deploy target/deploy/hello_program.so --url localhost
npm install
RPC_URL=http://127.0.0.1:8899 node client/client.mjs "Ilya"
```

## Результат

```
Program log: Hello, Ilya! 👋
Program log: Всего приветствий: 1
Program ... consumed 3379 of 202850 compute units
Состояние аккаунта: { counter: '1', lastGreeter: '7pC4w6...dNLox' }
```

> Примечание: Solana 4.x локальный валидатор отключает исполнение sBPF v0,
> поэтому программа собирается с `--arch v3`.