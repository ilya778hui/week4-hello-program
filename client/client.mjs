// Клиент для hello-program: создаёт аккаунт программы (один раз) и вызывает Greet.
// Запуск: node client/client.mjs "Ilya"            (сеть берётся из RPC_URL, по умолчанию devnet)
//         RPC_URL=http://127.0.0.1:8899 node client/client.mjs "Ilya"   (локальный валидатор)
import {
  Connection, Keypair, PublicKey, SystemProgram, Transaction,
  TransactionInstruction, sendAndConfirmTransaction,
} from "@solana/web3.js";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";

const RPC_URL = process.env.RPC_URL ?? "https://api.devnet.solana.com";
const NAME = process.argv[2] ?? "AlmaU";
const GREETING_SIZE = 8 + 32; // u64 counter + Pubkey last_greeter

const loadKeypair = (p) => Keypair.fromSecretKey(Uint8Array.from(JSON.parse(fs.readFileSync(p, "utf8"))));

const payer = loadKeypair(path.join(os.homedir(), ".config/solana/id.json"));
const programId = loadKeypair("target/deploy/hello_program-keypair.json").publicKey;

// Аккаунт для данных программы. Сохраняем его ключ, чтобы при повторных запусках счётчик рос.
const GREETING_KEY_FILE = "client/greeting-keypair.json";
if (!fs.existsSync(GREETING_KEY_FILE)) {
  fs.writeFileSync(GREETING_KEY_FILE, JSON.stringify(Array.from(Keypair.generate().secretKey)));
}
const greeting = loadKeypair(GREETING_KEY_FILE);

// Borsh-кодирование HelloInstruction::Greet { name }: [0] + u32 LE длина + utf8 байты
function encodeGreet(name) {
  const nameBytes = Buffer.from(name, "utf8");
  const len = Buffer.alloc(4);
  len.writeUInt32LE(nameBytes.length);
  return Buffer.concat([Buffer.from([0]), len, nameBytes]);
}

function decodeGreeting(data) {
  return {
    counter: data.readBigUInt64LE(0),
    lastGreeter: new PublicKey(data.subarray(8, 40)).toBase58(),
  };
}

const connection = new Connection(RPC_URL, "confirmed");
console.log("RPC         :", RPC_URL);
console.log("Program ID  :", programId.toBase58());
console.log("Payer       :", payer.publicKey.toBase58());
console.log("Greeting acc:", greeting.publicKey.toBase58());

const tx = new Transaction();

// 1) если аккаунта ещё нет — создаём его через System Program, владелец = наша программа
const existing = await connection.getAccountInfo(greeting.publicKey);
const signers = [payer];
if (!existing) {
  const lamports = await connection.getMinimumBalanceForRentExemption(GREETING_SIZE);
  tx.add(SystemProgram.createAccount({
    fromPubkey: payer.publicKey,
    newAccountPubkey: greeting.publicKey,
    lamports,
    space: GREETING_SIZE,
    programId,
  }));
  signers.push(greeting);
  console.log(`Создаём аккаунт программы (rent-exempt: ${lamports} lamports)`);
}

// 2) инструкция нашей программы
tx.add(new TransactionInstruction({
  programId,
  keys: [
    { pubkey: greeting.publicKey, isSigner: false, isWritable: true },
    { pubkey: payer.publicKey, isSigner: true, isWritable: false },
  ],
  data: encodeGreet(NAME),
}));

const signature = await sendAndConfirmTransaction(connection, tx, signers);
console.log("\nTx signature:", signature);
if (RPC_URL.includes("devnet")) {
  console.log("Explorer    :", `https://explorer.solana.com/tx/${signature}?cluster=devnet`);
}

const txInfo = await connection.getTransaction(signature, { commitment: "confirmed", maxSupportedTransactionVersion: 0 });
console.log("\nЛоги программы:");
for (const line of txInfo?.meta?.logMessages ?? []) console.log("  ", line);

const info = await connection.getAccountInfo(greeting.publicKey);
const state = decodeGreeting(info.data);
console.log("\nСостояние аккаунта:", { counter: state.counter.toString(), lastGreeter: state.lastGreeter });