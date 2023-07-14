# rust-pastebin-worker

[![Deploy to Cloudflare Workers](https://deploy.workers.cloudflare.com/button)](https://deploy.workers.cloudflare.com/?url=https://github.com/codebam/rust-pastebin-worker)

Set up your own KV in the wrangler.toml

Comment out the part that disallows uploading to / and upload the uploader.html
there if you want.

```
wrangler deploy
```

Upload it with curl

```
curl -X POST --data-binary @uploader.html https://pastebin.u.workers.dev
```
