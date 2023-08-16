# rust-pastebin-worker

[![Deploy to Cloudflare Workers](https://deploy.workers.cloudflare.com/button)](https://deploy.workers.cloudflare.com/?url=https://github.com/codebam/pastebin-worker)

Set up your own KV in the wrangler.toml and put uploader.html at /

Supports MIME types by putting .file-ext at the end of filenames when
downloading.

Files are limited to 15MB due to KV limitations.

Example usage:

```sh
curl -v -X POST -F upload=@yourfile.txt https://pastebin.seanbehan.ca
```

See the redirect URL to get where your paste is stored.

```javascript
export const pastebin_url = "https://pastebin.seanbehan.ca";
export const pastebin = {
  upload: async (filename) =>
    fetch(pastebin_url, {
      method: "POST",
      body: new URLSearchParams({ upload: await read_file(filename) }),
      redirect: "manual",
    }).then(get_redirect_location),
  delete: async (filename) =>
    fetch(pastebin_url + `/${filename}`, { method: "DELETE" }).then(
      (response) => response.text()
    ),
  upload_encrypt: async (filename) =>
    fetch(pastebin_url + "/encrypt", {
      method: "POST",
      body: new URLSearchParams({ upload: await read_file(filename) }),
      redirect: "manual",
    }).then(get_redirect_location),
  list: async () =>
    fetch(pastebin_url + "/list").then((response) => response.text()),
  upload_string: async (str) =>
    fetch(pastebin_url, {
      method: "POST",
      body: new URLSearchParams({ upload: str }),
      redirect: "manual",
    }).then(get_redirect_location),
};
```

Or just use the provided uploader.html
