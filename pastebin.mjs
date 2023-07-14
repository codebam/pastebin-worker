import fs from "fs";

const path = "btrfs";
const file_path = "/home/codebam/btrfs.md";

const url = new URL("https://worker-rust.codebam.workers.dev/" + path);
const method = "POST";
const body = fs.readFileSync(file_path).toString();

await fetch(url, { method, body })
  .then((response) => response.text())
  .then(console.log);
