import fs from "fs";

const path = "/delete";
const args = process.argv.slice(2);
const file_path = args[0];

// const url = new URL("https://worker-rust.codebam.workers.dev/" + path);
const url = new URL("http://127.0.0.1:8787" + path);
const method = "PUT";
const body = fs.readFileSync(file_path).toString();

await fetch(url, { method, body })
  .then((response) => response.text())
  .then(console.log);
